use std::marker::PhantomData;

use crate::ssh::handler::SshClientHandler;

// Zero-sized state markers for compile-time SSH connection lifecycle enforcement.

pub struct Disconnected;
pub struct Authenticated;

mod sealed {
    pub trait ConnectionState {}
}
pub trait ConnectionState: sealed::ConnectionState {}

impl sealed::ConnectionState for Disconnected {}
impl sealed::ConnectionState for Authenticated {}
impl ConnectionState for Disconnected {}
impl ConnectionState for Authenticated {}

/// An SSH session whose available operations are governed by its typestate `S`.
///
/// - `SshSession<Disconnected>` can `.connect()` to establish and authenticate.
/// - `SshSession<Authenticated>` can `.execute()`, `.read_file()`, and `.disconnect()`.
pub struct SshSession<S: ConnectionState> {
    pub(crate) id: String,
    pub(crate) host: String,
    pub(crate) port: u16,
    pub(crate) user: String,
    pub(crate) handle: Option<russh::client::Handle<SshClientHandler>>,
    pub(crate) _state: PhantomData<S>,
}

/// Metadata about an active session, safe to serialize and return to the LLM.
#[derive(Debug, serde::Serialize)]
pub struct SessionInfo {
    pub id: String,
    pub host: String,
    pub port: u16,
    pub user: String,
}

/// Output from a remote command execution.
#[derive(Debug, serde::Serialize)]
pub struct ExecOutput {
    pub exit_code: Option<u32>,
    pub stdout: String,
    pub stderr: String,
    pub duration_ms: u64,
    pub truncated: bool,
    pub timed_out: bool,
}
