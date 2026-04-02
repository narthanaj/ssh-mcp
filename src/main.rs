use std::sync::Arc;

use anyhow::Result;
use rmcp::ServiceExt;
use tracing_subscriber::EnvFilter;

use ssh_mcp::config::loader::load_config;
use ssh_mcp::mcp::server::SshMcpServer;
use ssh_mcp::ssh::manager::ConnectionManager;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing to stderr — stdout is reserved for MCP JSON-RPC
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .with_writer(std::io::stderr)
        .init();

    tracing::info!("ssh-mcp v{} starting", env!("CARGO_PKG_VERSION"));

    let config = load_config()?;
    tracing::info!(
        max_connections = config.server.max_connections,
        timeout = config.server.default_timeout_secs,
        allowed_commands = config.commands.allowed.len(),
        resources = config.resources.len(),
        "Configuration loaded"
    );

    let manager = Arc::new(ConnectionManager::new(Arc::new(config)));
    let server = SshMcpServer::new(manager);

    tracing::info!("Serving on stdio transport");
    let running = server.serve(rmcp::transport::stdio()).await?;
    running.waiting().await?;

    tracing::info!("ssh-mcp shutdown complete");
    Ok(())
}
