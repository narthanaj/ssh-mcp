use std::marker::PhantomData;
use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, Instant};

use russh::ChannelMsg;

use crate::error::SshMcpError;
use crate::ssh::command::ValidatedCommand;
use crate::ssh::handler::SshClientHandler;
use crate::ssh::typestate::{Authenticated, Disconnected, ExecOutput, SshSession};

impl SshSession<Disconnected> {
    /// Create a new disconnected session ready for connection.
    pub fn new(id: String, host: String, port: u16, user: String) -> Self {
        Self {
            id,
            host,
            port,
            user,
            handle: None,
            _state: PhantomData,
        }
    }

    /// Connect and authenticate, consuming the Disconnected session and producing
    /// an Authenticated session. Tries ssh-agent first, then key file, then password.
    pub async fn connect(
        self,
        use_ssh_agent: bool,
        key_path: Option<&str>,
        password: Option<&str>,
        strict_host_checking: bool,
        known_hosts_path: Option<&str>,
    ) -> Result<SshSession<Authenticated>, SshMcpError> {
        let config = Arc::new(russh::client::Config::default());
        let handler = SshClientHandler {
            host: self.host.clone(),
            port: self.port,
            strict_host_checking,
            known_hosts_path: known_hosts_path.map(String::from),
        };

        tracing::info!(host = %self.host, port = %self.port, user = %self.user, "Connecting via SSH");

        let mut handle =
            russh::client::connect(config, (self.host.as_str(), self.port), handler).await?;

        // Try authentication methods in priority order
        let mut authenticated = false;

        // 1. SSH-Agent
        if use_ssh_agent && !authenticated {
            match try_agent_auth(&mut handle, &self.user).await {
                Ok(true) => authenticated = true,
                Ok(false) => tracing::debug!("SSH agent auth failed, trying next method"),
                Err(e) => tracing::debug!(error = %e, "SSH agent unavailable, trying next method"),
            }
        }

        // 2. Key file
        if !authenticated {
            if let Some(path) = key_path {
                match try_key_auth(&mut handle, &self.user, path).await {
                    Ok(true) => authenticated = true,
                    Ok(false) => tracing::debug!("Key auth failed, trying next method"),
                    Err(e) => tracing::debug!(error = %e, "Key auth error, trying next method"),
                }
            }
        }

        // 3. Password (discouraged)
        if !authenticated {
            if let Some(pw) = password {
                match try_password_auth(&mut handle, &self.user, pw).await {
                    Ok(true) => authenticated = true,
                    Ok(false) => tracing::debug!("Password auth rejected"),
                    Err(e) => tracing::debug!(error = %e, "Password auth error"),
                }
            }
        }

        if !authenticated {
            return Err(SshMcpError::AuthenticationFailed {
                user: self.user.clone(),
                host: self.host.clone(),
            });
        }

        tracing::info!(host = %self.host, user = %self.user, "SSH authenticated");

        Ok(SshSession {
            id: self.id,
            host: self.host,
            port: self.port,
            user: self.user,
            handle: Some(handle),
            _state: PhantomData,
        })
    }
}

impl SshSession<Authenticated> {
    /// Execute a validated command with a strict timeout.
    pub async fn execute(
        &self,
        cmd: &ValidatedCommand,
        timeout_secs: u64,
        max_output_bytes: usize,
    ) -> Result<ExecOutput, SshMcpError> {
        let handle = self
            .handle
            .as_ref()
            .ok_or_else(|| SshMcpError::CommandExecution("session handle missing".into()))?;

        let exec_string = cmd.to_exec_string();
        tracing::debug!(session = %self.id, cmd = %exec_string, "Executing command");

        let mut channel = handle.channel_open_session().await?;
        channel.exec(true, exec_string.as_bytes()).await?;

        let start = Instant::now();
        let timeout = Duration::from_secs(timeout_secs);

        let result = tokio::time::timeout(timeout, async {
            read_channel_output(&mut channel, max_output_bytes).await
        })
        .await;

        let duration_ms = start.elapsed().as_millis() as u64;

        match result {
            Ok(output) => {
                let (stdout, stderr, exit_code, truncated) = output?;
                let _ = channel.close().await;
                Ok(ExecOutput {
                    exit_code,
                    stdout,
                    stderr,
                    duration_ms,
                    truncated,
                    timed_out: false,
                })
            }
            Err(_) => {
                tracing::warn!(session = %self.id, timeout_secs, "Command timed out, closing channel");
                let _ = channel.close().await;
                Ok(ExecOutput {
                    exit_code: None,
                    stdout: String::new(),
                    stderr: format!("Command timed out after {}s", timeout_secs),
                    duration_ms,
                    truncated: false,
                    timed_out: true,
                })
            }
        }
    }

    /// Read a remote file via `head -c` with a size limit. Used for MCP resources.
    pub async fn read_file(
        &self,
        path: &str,
        max_bytes: usize,
        timeout_secs: u64,
    ) -> Result<String, SshMcpError> {
        let handle = self
            .handle
            .as_ref()
            .ok_or_else(|| SshMcpError::CommandExecution("session handle missing".into()))?;

        let quoted_path = shlex::try_quote(path).map_err(|_| SshMcpError::CommandRejected {
            reason: format!("invalid path: {}", path),
        })?;
        let exec_string = format!("head -c {} {}", max_bytes, quoted_path);

        let mut channel = handle.channel_open_session().await?;
        channel.exec(true, exec_string.as_bytes()).await?;

        let timeout = Duration::from_secs(timeout_secs);
        let result = tokio::time::timeout(timeout, async {
            read_channel_output(&mut channel, max_bytes).await
        })
        .await;

        let _ = channel.close().await;

        match result {
            Ok(Ok((stdout, stderr, exit_code, _))) => {
                if exit_code != Some(0) && !stderr.is_empty() {
                    Err(SshMcpError::CommandExecution(format!(
                        "Failed to read file: {}",
                        stderr
                    )))
                } else {
                    Ok(stdout)
                }
            }
            Ok(Err(e)) => Err(e),
            Err(_) => Err(SshMcpError::CommandTimeout { timeout_secs }),
        }
    }

    /// Gracefully disconnect the session, consuming it.
    pub async fn disconnect(self) -> Result<(), SshMcpError> {
        if let Some(handle) = self.handle {
            tracing::info!(session = %self.id, host = %self.host, "Disconnecting SSH session");
            handle
                .disconnect(russh::Disconnect::ByApplication, "", "en")
                .await?;
        }
        Ok(())
    }

    pub fn info(&self) -> crate::ssh::typestate::SessionInfo {
        crate::ssh::typestate::SessionInfo {
            id: self.id.clone(),
            host: self.host.clone(),
            port: self.port,
            user: self.user.clone(),
        }
    }
}

/// Read stdout, stderr, and exit status from a channel.
async fn read_channel_output(
    channel: &mut russh::Channel<russh::client::Msg>,
    max_bytes: usize,
) -> Result<(String, String, Option<u32>, bool), SshMcpError> {
    let mut stdout_buf = Vec::new();
    let mut stderr_buf = Vec::new();
    let mut exit_code = None;
    let mut truncated = false;

    while let Some(msg) = channel.wait().await {
        match msg {
            ChannelMsg::Data { ref data } => {
                let remaining = max_bytes.saturating_sub(stdout_buf.len());
                if remaining == 0 {
                    truncated = true;
                } else {
                    let to_take = remaining.min(data.len());
                    stdout_buf.extend_from_slice(&data[..to_take]);
                    if to_take < data.len() {
                        truncated = true;
                    }
                }
            }
            ChannelMsg::ExtendedData { ref data, ext } => {
                if ext == 1 {
                    let remaining = max_bytes.saturating_sub(stderr_buf.len());
                    if remaining > 0 {
                        let to_take = remaining.min(data.len());
                        stderr_buf.extend_from_slice(&data[..to_take]);
                    }
                }
            }
            ChannelMsg::ExitStatus { exit_status } => {
                exit_code = Some(exit_status);
            }
            ChannelMsg::Eof | ChannelMsg::Close => break,
            _ => {}
        }
    }

    let stdout = String::from_utf8_lossy(&stdout_buf).into_owned();
    let stderr = String::from_utf8_lossy(&stderr_buf).into_owned();
    Ok((stdout, stderr, exit_code, truncated))
}

async fn try_agent_auth(
    handle: &mut russh::client::Handle<SshClientHandler>,
    user: &str,
) -> Result<bool, SshMcpError> {
    tracing::debug!("Attempting SSH agent authentication");
    let mut agent = russh::keys::agent::client::AgentClient::connect_env()
        .await
        .map_err(SshMcpError::KeyLoad)?;

    let identities = agent
        .request_identities()
        .await
        .map_err(SshMcpError::KeyLoad)?;
    if identities.is_empty() {
        tracing::debug!("SSH agent has no identities");
        return Ok(false);
    }

    for identity in &identities {
        tracing::debug!(algorithm = ?identity.algorithm(), "Trying agent identity");
        let result = handle
            .authenticate_publickey_with(user, identity.clone(), None, &mut agent)
            .await;
        match result {
            Ok(auth_result) => {
                if auth_result.success() {
                    return Ok(true);
                }
            }
            Err(_) => continue,
        }
    }
    Ok(false)
}

async fn try_key_auth(
    handle: &mut russh::client::Handle<SshClientHandler>,
    user: &str,
    key_path: &str,
) -> Result<bool, SshMcpError> {
    tracing::debug!(path = %key_path, "Attempting key-based authentication");
    let expanded = shellexpand_path(key_path);
    let key = russh::keys::load_secret_key(Path::new(&expanded), None)?;
    let key_with_alg = russh::keys::PrivateKeyWithHashAlg::new(Arc::new(key), None);
    let result = handle.authenticate_publickey(user, key_with_alg).await?;
    Ok(result.success())
}

async fn try_password_auth(
    handle: &mut russh::client::Handle<SshClientHandler>,
    user: &str,
    password: &str,
) -> Result<bool, SshMcpError> {
    tracing::debug!("Attempting password authentication");
    let result = handle.authenticate_password(user, password).await?;
    Ok(result.success())
}

fn shellexpand_path(path: &str) -> String {
    if let Some(rest) = path.strip_prefix("~/") {
        if let Ok(home) = std::env::var("HOME") {
            return format!("{}/{}", home, rest);
        }
    }
    path.to_string()
}
