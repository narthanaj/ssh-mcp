use std::path::PathBuf;

use crate::config::types::ServerConfig;
use crate::error::SshMcpError;

pub fn load_config() -> Result<ServerConfig, SshMcpError> {
    let path = config_path();
    let contents = std::fs::read_to_string(&path).map_err(|e| {
        SshMcpError::Config(format!(
            "Failed to read config file {}: {}",
            path.display(),
            e
        ))
    })?;
    let config: ServerConfig = toml::from_str(&contents)
        .map_err(|e| SshMcpError::Config(format!("Failed to parse config: {e}")))?;
    validate(&config)?;
    Ok(config)
}

fn config_path() -> PathBuf {
    std::env::var("SSH_MCP_CONFIG")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("config.toml"))
}

fn validate(config: &ServerConfig) -> Result<(), SshMcpError> {
    if config.commands.allowed.is_empty() {
        return Err(SshMcpError::Config(
            "commands.allowed must not be empty".into(),
        ));
    }
    if config.server.max_connections == 0 {
        return Err(SshMcpError::Config(
            "server.max_connections must be > 0".into(),
        ));
    }
    if config.server.default_timeout_secs == 0 {
        return Err(SshMcpError::Config(
            "server.default_timeout_secs must be > 0".into(),
        ));
    }
    // Validate the arg_pattern compiles
    regex::Regex::new(&config.commands.arg_pattern)
        .map_err(|e| SshMcpError::Config(format!("Invalid arg_pattern regex: {e}")))?;
    Ok(())
}
