# ssh-mcp

## Project Overview
Secure SSH MCP (Model Context Protocol) server in Rust. Allows LLMs to execute commands on remote hosts over SSH via stdio transport.

## Development
- Language: Rust (edition 2024)
- Build: `cargo build`
- Test: `cargo test`
- Run: `cargo run` (requires `config.toml` in working directory, or set `SSH_MCP_CONFIG` env var)
- Check: `cargo check`
- Lint: `cargo clippy`

## Architecture
- `src/error.rs` — Unified error types (`SshMcpError`)
- `src/config/` — TOML configuration (server settings, command whitelist, resources)
- `src/ssh/typestate.rs` — Compile-time SSH state machine (Disconnected → Authenticated)
- `src/ssh/handler.rs` — russh client handler (host key verification)
- `src/ssh/connection.rs` — SSH connect/auth/execute/disconnect with strict timeouts
- `src/ssh/command.rs` — Command validation (regex, whitelist, env var checks)
- `src/ssh/manager.rs` — Session manager (DashMap, rate limiting, connection pooling)
- `src/mcp/server.rs` — MCP server (tools, resources, prompts via rmcp)
- `src/mcp/params.rs` — Tool parameter structs
- `src/mcp/prompts.rs` — Built-in diagnostic prompt templates
- `src/ratelimit.rs` — Token bucket rate limiter per session

## Key Dependencies
- `rmcp` — Official Rust MCP SDK (stdio transport)
- `russh` — Pure-Rust async SSH client (no FFI)
- `russh::keys` — SSH key management (use this, NOT separate `russh-keys` crate — avoids type conflicts)
- `dashmap` — Concurrent session store
- `secrecy`/`zeroize` — Secret memory management

## Notes
- All logging goes to stderr (stdout is reserved for MCP JSON-RPC)
- Command failures return stdout+stderr+exit_code as successful tool results (not MCP errors)
- `russh::keys::*` and `russh_keys::*` are DIFFERENT types — always use `russh::keys`
