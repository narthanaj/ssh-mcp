# Security Policy

## Reporting a Vulnerability

If you discover a security vulnerability in ssh-mcp, please report it responsibly.

**Do NOT open a public GitHub issue for security vulnerabilities.**

Instead, please email the maintainer directly or use GitHub's private vulnerability reporting feature:

1. Go to the [Security tab](https://github.com/narthanaj/ssh-mcp/security) of the repository
2. Click "Report a vulnerability"
3. Provide a detailed description of the issue

## What to Include

- Description of the vulnerability
- Steps to reproduce
- Potential impact
- Suggested fix (if any)

## Response Timeline

- **Acknowledgment**: Within 48 hours
- **Assessment**: Within 1 week
- **Fix**: Depending on severity, typically within 2 weeks for critical issues

## Scope

The following are in scope:

- Command injection bypasses (escaping the whitelist or regex validation)
- Authentication/authorization bypasses
- Host key verification bypasses
- Memory safety issues
- Information disclosure (secrets leaking to stdout/logs)
- Rate limiter bypasses
- Path traversal in resource reading

## Security Design

ssh-mcp is designed with defense-in-depth:

- **Regex argument validation** — strict allowlist pattern, not just metacharacter blacklisting
- **Command whitelist** — only explicitly allowed binaries can execute
- **No shell invocation** — commands are never passed through `sh -c`
- **Strict timeouts** — hanging commands are killed automatically
- **Rate limiting** — prevents LLM loop storms
- **Config-driven policy** — the LLM cannot modify security settings at runtime
- **Typestate pattern** — compile-time enforcement of SSH connection lifecycle
