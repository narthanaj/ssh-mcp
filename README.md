# ssh-mcp

[![CI](https://github.com/narthanaj/ssh-mcp/actions/workflows/ci.yml/badge.svg)](https://github.com/narthanaj/ssh-mcp/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Rust](https://img.shields.io/badge/rust-1.85%2B-orange.svg)](https://www.rust-lang.org)

A secure SSH server for the [Model Context Protocol (MCP)](https://modelcontextprotocol.io/), enabling LLMs to execute commands on remote hosts over SSH.

Built in Rust with compile-time safety guarantees, defense-in-depth security, and strict command validation.

## Features

- **4 MCP Tools** — `ssh_connect`, `ssh_execute`, `ssh_disconnect`, `ssh_list_sessions`
- **MCP Resources** — Read remote files (e.g., syslog) directly without running commands
- **5 MCP Prompts** — Guided diagnostic workflows (CPU, disk, services, auth, network)
- **SSH-Agent Support** — Prefers ssh-agent, falls back to key file, then password
- **Command Whitelisting** — Only explicitly allowed binaries can execute
- **Regex Argument Validation** — Strict allowlist pattern, not just metacharacter blacklisting
- **No Shell Invocation** — Commands are never passed through `sh -c`
- **Strict Timeouts** — Hanging commands (e.g., `tail -f`) are killed automatically
- **Rate Limiting** — Token bucket per session prevents LLM loop storms
- **Typestate Pattern** — Compile-time enforcement of SSH connection lifecycle
- **Context-Rich Errors** — Failed commands return stdout+stderr+exit_code so the LLM can self-correct
- **Host Key Verification** — known_hosts checking (configurable)

## Quick Start

### Prerequisites

- Rust 1.85 or later
- An SSH server you want to connect to

### Install

```bash
git clone https://github.com/narthanaj/ssh-mcp.git
cd ssh-mcp
cp config.example.toml config.toml  # Edit with your settings
cargo build --release
```

### Configure with Claude Code

Add to your Claude Code MCP settings (`~/.claude/settings.json` or project `.mcp.json`):

```json
{
  "mcpServers": {
    "ssh": {
      "command": "/path/to/ssh-mcp/target/release/ssh-mcp",
      "env": {
        "SSH_MCP_CONFIG": "/path/to/ssh-mcp/config.toml"
      }
    }
  }
}
```

### Configure with Claude Desktop

Add to `claude_desktop_config.json`:

```json
{
  "mcpServers": {
    "ssh": {
      "command": "/path/to/ssh-mcp/target/release/ssh-mcp",
      "env": {
        "SSH_MCP_CONFIG": "/path/to/ssh-mcp/config.toml"
      }
    }
  }
}
```

## Configuration

Copy `config.example.toml` to `config.toml` and customize:

```toml
[server]
max_connections = 10
default_timeout_secs = 30
strict_host_key_checking = false  # Set to true in production
use_ssh_agent = true
rate_limit_per_session = 30       # Max tool calls per minute per session

[commands]
allowed = ["ls", "cat", "grep", "ps", "df", "systemctl", "docker", ...]
denied = ["sh", "bash", "zsh", "eval", "sudo", "su"]
max_args = 50
max_output_bytes = 1048576
arg_pattern = '^[a-zA-Z0-9_./:@=, -]+$'

[commands.allowed_env]
PATH = "/usr/local/bin:/usr/bin:/bin"
LANG = "en_US.UTF-8"

# Optional: pre-configured targets
# [[targets]]
# name = "prod-web"
# host = "web.example.com"
# username = "deploy"
# key_path = "~/.ssh/id_ed25519"

# Optional: remote files readable as MCP resources
# [[resources]]
# name = "syslog"
# description = "System log file"
# path = "/var/log/syslog"
# max_bytes = 65536
```

Set the config path via environment variable:

```bash
export SSH_MCP_CONFIG=/path/to/config.toml
```

Or place `config.toml` in the working directory.

## Usage Examples

Once configured, the LLM can:

```
Connect to my server at 192.168.1.100 as user admin with key ~/.ssh/id_ed25519
```

```
Check the disk usage on the connected server
```

```
Show me the last 50 lines of the nginx error log
```

```
What services are failing on the server?
```

## MCP Tools

| Tool | Description |
|------|-------------|
| `ssh_connect` | Connect to a remote host via SSH (agent, key, or password auth) |
| `ssh_execute` | Execute a validated command on an active session |
| `ssh_disconnect` | Close an SSH session |
| `ssh_list_sessions` | List all active sessions |

## MCP Prompts

| Prompt | Description |
|--------|-------------|
| `diagnose_high_cpu` | Guided CPU analysis (top, ps, uptime) |
| `diagnose_disk_space` | Disk usage investigation (df, du, find) |
| `check_service_health` | Systemd service status and logs |
| `analyze_auth_failures` | Authentication failure analysis |
| `network_diagnostics` | Network connectivity checks |

## Security Model

ssh-mcp is designed with defense-in-depth:

1. **Command Whitelist** — Only binaries listed in `commands.allowed` can execute. Shells (`sh`, `bash`, etc.) are explicitly denied.

2. **Regex Validation** — Every argument must match the `arg_pattern` regex (default: `^[a-zA-Z0-9_./:@=, -]+$`). This blocks shell metacharacters (`|`, `;`, `$`, `` ` ``, `>`, `<`, `(`, `)`, etc.) at the character level.

3. **No Shell Execution** — Commands are sent via SSH exec channel. Arguments are additionally quoted with `shlex` as a second defense layer.

4. **Environment Whitelist** — Only env vars listed in `commands.allowed_env` can be set.

5. **Strict Timeouts** — Every command has a timeout. When it fires, the SSH channel is killed and partial output is returned.

6. **Rate Limiting** — Token bucket per session prevents runaway LLM loops.

7. **Host Key Verification** — Configurable known_hosts checking.

8. **Config-Driven Policy** — The LLM cannot modify security settings at runtime. All policy lives in the TOML config file.

## Architecture

```
src/
  main.rs              Entry point (tracing, config, stdio transport)
  lib.rs               Module exports
  error.rs             Unified error types
  ratelimit.rs         Token bucket rate limiter
  config/
    types.rs           Configuration structs
    loader.rs          TOML loading and validation
  ssh/
    typestate.rs       Compile-time state machine (Disconnected -> Authenticated)
    handler.rs         russh client handler (host key verification)
    connection.rs      SSH connect/auth/execute with strict timeouts
    command.rs         Command validation (regex, whitelist, env)
    manager.rs         Session manager (DashMap, rate limiting)
  mcp/
    server.rs          MCP server (tools, resources, prompts)
    params.rs          Tool parameter structs
    prompts.rs         Diagnostic prompt templates
```

**Key design decisions:**

- **Pure Rust SSH** via `russh` (no FFI, no OpenSSL)
- **Typestate pattern** prevents calling execute on unauthenticated sessions at compile time
- **DashMap** for concurrent session access (MCP tools can run in parallel)
- **All logging to stderr** — stdout is reserved for MCP JSON-RPC

## Development

```bash
cargo build          # Build
cargo test           # Run tests
cargo clippy         # Lint
cargo fmt            # Format
RUST_LOG=debug cargo run  # Run with debug logging
```

## Roadmap

- [ ] SFTP tools (upload, download, list) with path jailing
- [ ] OS-level sandboxing (Landlock, seccomp-bpf)
- [ ] Graceful shutdown with CancellationToken
- [ ] OAuth 2.1 for HTTP transport
- [ ] Session health monitoring and auto-cleanup
- [ ] Connection pooling with keepalive

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

## License

[MIT](LICENSE)
