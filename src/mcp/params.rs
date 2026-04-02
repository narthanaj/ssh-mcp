use std::collections::HashMap;

use serde::Deserialize;

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ConnectParams {
    /// Hostname or IP address of the remote host
    pub host: String,

    /// SSH port (default: 22)
    pub port: Option<u16>,

    /// Username for authentication
    pub username: String,

    /// Authentication method: "agent", "key", or "password"
    pub auth_method: Option<AuthMethod>,

    /// Path to SSH private key file (for key-based auth)
    pub key_path: Option<String>,

    /// Password (for password-based auth — discouraged, prefer key-based)
    pub password: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum AuthMethod {
    Agent,
    Key,
    Password,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ExecParams {
    /// Session ID returned by ssh_connect
    pub session_id: String,

    /// Command binary to execute (must be in allowed list)
    pub command: String,

    /// Command arguments as an array of strings
    #[serde(default)]
    pub args: Vec<String>,

    /// Environment variables to set (must be in allowed_env whitelist)
    #[serde(default)]
    pub env: HashMap<String, String>,

    /// Timeout in seconds (default: server config default_timeout_secs)
    pub timeout: Option<u64>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct DisconnectParams {
    /// Session ID to disconnect
    pub session_id: String,
}
