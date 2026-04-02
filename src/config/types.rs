use serde::Deserialize;
use std::collections::HashMap;

#[derive(Debug, Deserialize)]
pub struct ServerConfig {
    pub server: ServerSettings,
    #[serde(default)]
    pub targets: Vec<SshTarget>,
    pub commands: CommandConfig,
    #[serde(default)]
    pub resources: Vec<ResourceDef>,
}

#[derive(Debug, Deserialize)]
pub struct ServerSettings {
    #[serde(default = "default_max_connections")]
    pub max_connections: usize,
    #[serde(default = "default_timeout_secs")]
    pub default_timeout_secs: u64,
    #[serde(default = "default_true")]
    pub strict_host_key_checking: bool,
    pub known_hosts_path: Option<String>,
    #[serde(default = "default_true")]
    pub use_ssh_agent: bool,
    #[serde(default = "default_rate_limit")]
    pub rate_limit_per_session: u32,
}

#[derive(Debug, Deserialize)]
pub struct SshTarget {
    pub name: String,
    pub host: String,
    #[serde(default = "default_port")]
    pub port: u16,
    pub username: String,
    pub key_path: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CommandConfig {
    pub allowed: Vec<String>,
    #[serde(default)]
    pub denied: Vec<String>,
    #[serde(default = "default_max_args")]
    pub max_args: usize,
    #[serde(default = "default_max_output_bytes")]
    pub max_output_bytes: usize,
    #[serde(default = "default_arg_pattern")]
    pub arg_pattern: String,
    #[serde(default)]
    pub allowed_env: HashMap<String, String>,
}

#[derive(Debug, Deserialize)]
pub struct ResourceDef {
    pub name: String,
    pub description: String,
    pub path: String,
    #[serde(default = "default_max_resource_bytes")]
    pub max_bytes: usize,
}

fn default_max_connections() -> usize {
    10
}
fn default_timeout_secs() -> u64 {
    30
}
fn default_true() -> bool {
    true
}
fn default_rate_limit() -> u32 {
    30
}
fn default_port() -> u16 {
    22
}
fn default_max_args() -> usize {
    50
}
fn default_max_output_bytes() -> usize {
    1_048_576
}
fn default_arg_pattern() -> String {
    r"^[a-zA-Z0-9_./:@=, -]+$".to_string()
}
fn default_max_resource_bytes() -> usize {
    65_536
}
