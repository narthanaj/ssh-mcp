# Contributing to ssh-mcp

Thank you for your interest in contributing! This guide will help you get started.

## Getting Started

1. Fork the repository
2. Clone your fork: `git clone https://github.com/YOUR_USERNAME/ssh-mcp.git`
3. Create a branch: `git checkout -b feature/your-feature`
4. Make your changes
5. Run checks: `cargo fmt && cargo clippy && cargo test`
6. Commit and push
7. Open a pull request

## Development Setup

You need Rust 1.85+ installed. Then:

```bash
cargo build    # Build the project
cargo test     # Run tests
cargo clippy   # Run linter
cargo fmt      # Format code
```

## Code Style

- Run `cargo fmt` before committing
- All `cargo clippy` warnings must be resolved
- Write tests for new command validation logic
- Never write to stdout — all logging goes to stderr (MCP uses stdout for JSON-RPC)

## Pull Request Process

1. Ensure your PR passes CI (build, test, clippy, fmt)
2. Update documentation if you change behavior
3. Add tests for new features
4. Keep PRs focused — one feature or fix per PR
5. Write a clear PR description explaining what and why

## Security

If you discover a security vulnerability, please do **NOT** open a public issue. Instead, see [SECURITY.md](SECURITY.md) for responsible disclosure instructions.

## Architecture Notes

- **`russh::keys`** must be used instead of the separate `russh-keys` crate (type conflicts)
- **Typestate pattern** enforces SSH connection lifecycle at compile time
- **Command validation** is the security boundary — all changes there need thorough review
- **Config-driven security** — the LLM cannot modify whitelist/policy at runtime

## License

By contributing, you agree that your contributions will be licensed under the MIT License.
