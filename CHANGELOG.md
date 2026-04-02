# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2026-04-02

### Added

- Initial release
- **MCP Tools**: `ssh_connect`, `ssh_execute`, `ssh_disconnect`, `ssh_list_sessions`
- **MCP Resources**: Config-defined remote file reading (e.g., syslog, auth.log)
- **MCP Prompts**: 5 diagnostic templates (CPU, disk, service health, auth failures, network)
- **SSH Authentication**: ssh-agent, key file, and password support (agent-first priority)
- **Command Validation**: Regex-based argument validation, binary whitelist/denylist, env var whitelist
- **Typestate Pattern**: Compile-time SSH connection lifecycle enforcement
- **Rate Limiting**: Token bucket per session to prevent LLM loop storms
- **Strict Timeouts**: Channel-level timeouts that kill hanging commands
- **Host Key Verification**: known_hosts checking via russh
- **Context-Rich Errors**: Command failures return stdout+stderr+exit_code (not opaque errors)
- **TOML Configuration**: Server settings, targets, command policy, resources all config-driven
