use thiserror::Error;

#[derive(Error, Debug)]
pub enum SshMcpError {
    #[error("SSH connection error: {0}")]
    SshConnection(#[from] russh::Error),

    #[error("Key loading error: {0}")]
    KeyLoad(#[from] russh::keys::Error),

    #[error("SSH agent auth error: {0}")]
    AgentAuth(#[from] russh::AgentAuthError),

    #[error("SSH send error: {0}")]
    SendError(#[from] russh::SendError),

    #[error("Host key verification failed for {host}")]
    HostKeyVerification { host: String },

    #[error("Authentication failed for {user}@{host}")]
    AuthenticationFailed { user: String, host: String },

    #[error("Session not found: {session_id}")]
    SessionNotFound { session_id: String },

    #[error("Command rejected: {reason}")]
    CommandRejected { reason: String },

    #[error("Command execution failed: {0}")]
    CommandExecution(String),

    #[error("Command timed out after {timeout_secs}s")]
    CommandTimeout { timeout_secs: u64 },

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Path traversal detected: {path}")]
    PathTraversal { path: String },

    #[error("Connection limit reached (max {max})")]
    ConnectionLimitReached { max: usize },

    #[error("Rate limited: too many requests for session {session_id}")]
    RateLimited { session_id: String },

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}
