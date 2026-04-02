use std::collections::HashMap;
use std::sync::Arc;

use dashmap::DashMap;

use crate::config::ServerConfig;
use crate::error::SshMcpError;
use crate::ratelimit::RateLimiter;
use crate::ssh::command::ValidatedCommand;
use crate::ssh::typestate::{Authenticated, Disconnected, ExecOutput, SessionInfo, SshSession};

pub struct ConnectionManager {
    sessions: DashMap<String, SshSession<Authenticated>>,
    config: Arc<ServerConfig>,
    rate_limiter: RateLimiter,
}

impl ConnectionManager {
    pub fn new(config: Arc<ServerConfig>) -> Self {
        let rate_limiter = RateLimiter::new(config.server.rate_limit_per_session);
        Self {
            sessions: DashMap::new(),
            config,
            rate_limiter,
        }
    }

    /// Establish a new SSH connection. Returns the session ID.
    pub async fn connect(
        &self,
        host: &str,
        port: u16,
        username: &str,
        key_path: Option<&str>,
        password: Option<&str>,
    ) -> Result<String, SshMcpError> {
        if self.sessions.len() >= self.config.server.max_connections {
            return Err(SshMcpError::ConnectionLimitReached {
                max: self.config.server.max_connections,
            });
        }

        let id = uuid::Uuid::new_v4().to_string();
        let session = SshSession::<Disconnected>::new(
            id.clone(),
            host.to_string(),
            port,
            username.to_string(),
        );

        let known_hosts = self.config.server.known_hosts_path.as_deref();

        let authenticated = session
            .connect(
                self.config.server.use_ssh_agent,
                key_path,
                password,
                self.config.server.strict_host_key_checking,
                known_hosts,
            )
            .await?;

        self.sessions.insert(id.clone(), authenticated);
        Ok(id)
    }

    /// Execute a validated command on an existing session.
    pub async fn execute(
        &self,
        session_id: &str,
        cmd: ValidatedCommand,
        timeout_secs: Option<u64>,
    ) -> Result<ExecOutput, SshMcpError> {
        // Rate limit check
        self.rate_limiter.check(session_id)?;

        let session =
            self.sessions
                .get(session_id)
                .ok_or_else(|| SshMcpError::SessionNotFound {
                    session_id: session_id.to_string(),
                })?;

        let timeout = timeout_secs.unwrap_or(self.config.server.default_timeout_secs);
        session
            .execute(&cmd, timeout, self.config.commands.max_output_bytes)
            .await
    }

    /// Read a remote file for MCP resources.
    pub async fn read_file(
        &self,
        session_id: &str,
        path: &str,
        max_bytes: usize,
    ) -> Result<String, SshMcpError> {
        let session =
            self.sessions
                .get(session_id)
                .ok_or_else(|| SshMcpError::SessionNotFound {
                    session_id: session_id.to_string(),
                })?;

        session
            .read_file(path, max_bytes, self.config.server.default_timeout_secs)
            .await
    }

    /// Disconnect and remove a session.
    pub async fn disconnect(&self, session_id: &str) -> Result<(), SshMcpError> {
        let (_, session) =
            self.sessions
                .remove(session_id)
                .ok_or_else(|| SshMcpError::SessionNotFound {
                    session_id: session_id.to_string(),
                })?;
        self.rate_limiter.remove_session(session_id);
        session.disconnect().await
    }

    /// List all active sessions.
    pub fn list_sessions(&self) -> Vec<SessionInfo> {
        self.sessions
            .iter()
            .map(|entry| entry.value().info())
            .collect()
    }

    /// Disconnect all sessions (for shutdown).
    pub async fn disconnect_all(&self) {
        let ids: Vec<String> = self.sessions.iter().map(|e| e.key().clone()).collect();
        for id in ids {
            if let Some((_, session)) = self.sessions.remove(&id) {
                let _ = session.disconnect().await;
            }
        }
    }

    /// Validate a command against the config and return a ValidatedCommand.
    pub fn validate_command(
        &self,
        binary: &str,
        args: &[String],
        env: &HashMap<String, String>,
    ) -> Result<ValidatedCommand, SshMcpError> {
        ValidatedCommand::from_params(binary, args, env, &self.config.commands)
    }

    pub fn config(&self) -> &ServerConfig {
        &self.config
    }
}
