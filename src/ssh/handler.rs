use russh::keys::PublicKey;

use crate::error::SshMcpError;

/// russh client event handler. Manages host key verification.
pub struct SshClientHandler {
    pub host: String,
    pub port: u16,
    pub strict_host_checking: bool,
    pub known_hosts_path: Option<String>,
}

impl russh::client::Handler for SshClientHandler {
    type Error = SshMcpError;

    async fn check_server_key(
        &mut self,
        server_public_key: &PublicKey,
    ) -> Result<bool, Self::Error> {
        if !self.strict_host_checking {
            tracing::warn!(host = %self.host, "Host key checking disabled — accepting key");
            return Ok(true);
        }

        let result = match &self.known_hosts_path {
            Some(path) => russh::keys::check_known_hosts_path(
                &self.host,
                self.port,
                server_public_key,
                std::path::Path::new(path),
            ),
            None => russh::keys::check_known_hosts(&self.host, self.port, server_public_key),
        };

        match result {
            Ok(true) => Ok(true),
            Ok(false) => {
                tracing::error!(host = %self.host, "Host key MISMATCH — possible MITM attack");
                Err(SshMcpError::HostKeyVerification {
                    host: self.host.clone(),
                })
            }
            Err(e) => {
                tracing::warn!(host = %self.host, error = %e, "Known hosts check failed");
                Err(SshMcpError::HostKeyVerification {
                    host: self.host.clone(),
                })
            }
        }
    }
}
