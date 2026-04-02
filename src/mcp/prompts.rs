use rmcp::model::{GetPromptResult, Prompt, PromptArgument, PromptMessage, PromptMessageRole};

pub fn get_prompt(
    name: &str,
    session_id: &str,
    extra_arg: Option<&str>,
) -> Option<GetPromptResult> {
    match name {
        "diagnose_high_cpu" => Some(diagnose_high_cpu(session_id)),
        "diagnose_disk_space" => Some(diagnose_disk_space(session_id)),
        "check_service_health" => Some(check_service_health(session_id, extra_arg?)),
        "analyze_auth_failures" => Some(analyze_auth_failures(session_id)),
        "network_diagnostics" => Some(network_diagnostics(session_id, extra_arg)),
        _ => None,
    }
}

pub fn list_prompts() -> Vec<Prompt> {
    vec![
        Prompt::new(
            "diagnose_high_cpu",
            Some("Analyze high CPU usage on a remote host. Runs top, ps, and uptime."),
            Some(vec![required_arg("session_id", "SSH session ID")]),
        ),
        Prompt::new(
            "diagnose_disk_space",
            Some("Check disk usage and find large files consuming space."),
            Some(vec![required_arg("session_id", "SSH session ID")]),
        ),
        Prompt::new(
            "check_service_health",
            Some("Check systemd service status and recent journal logs."),
            Some(vec![
                required_arg("session_id", "SSH session ID"),
                required_arg("service_name", "Name of the systemd service"),
            ]),
        ),
        Prompt::new(
            "analyze_auth_failures",
            Some("Review recent authentication failures in system logs."),
            Some(vec![required_arg("session_id", "SSH session ID")]),
        ),
        Prompt::new(
            "network_diagnostics",
            Some("Run network connectivity checks (interfaces, routes, DNS)."),
            Some(vec![
                required_arg("session_id", "SSH session ID"),
                PromptArgument::new("target_host")
                    .with_description("Optional host to test connectivity to")
                    .with_required(false),
            ]),
        ),
    ]
}

fn required_arg(name: &str, desc: &str) -> PromptArgument {
    PromptArgument::new(name)
        .with_description(desc)
        .with_required(true)
}

fn user_msg(text: String) -> Vec<PromptMessage> {
    vec![PromptMessage::new_text(PromptMessageRole::User, text)]
}

fn diagnose_high_cpu(session_id: &str) -> GetPromptResult {
    GetPromptResult::new(user_msg(format!(
        "I need to diagnose high CPU usage on the remote host connected via session '{session_id}'. \
         Please follow these steps:\n\n\
         1. Run `ssh_execute` with command \"top\" and args [\"-bn1\", \"-o\", \"%CPU\"]\n\
         2. Run `ssh_execute` with command \"ps\" and args [\"aux\", \"--sort=-%cpu\"]\n\
         3. Run `ssh_execute` with command \"uptime\" to check load averages\n\
         4. Analyze the output and identify the root cause\n\
         5. Suggest remediation steps"
    )))
    .with_description("Diagnose high CPU usage")
}

fn diagnose_disk_space(session_id: &str) -> GetPromptResult {
    GetPromptResult::new(user_msg(format!(
        "I need to check disk space on the remote host connected via session '{session_id}'. \
         Please follow these steps:\n\n\
         1. Run `ssh_execute` with command \"df\" and args [\"-h\"]\n\
         2. Run `ssh_execute` with command \"du\" and args [\"-sh\", \"/var/log\", \"/tmp\", \"/home\"]\n\
         3. Run `ssh_execute` with command \"find\" and args [\"/var/log\", \"-type\", \"f\", \"-size\", \"+100M\"]\n\
         4. Analyze the results and suggest cleanup actions"
    )))
    .with_description("Diagnose disk space issues")
}

fn check_service_health(session_id: &str, service_name: &str) -> GetPromptResult {
    GetPromptResult::new(user_msg(format!(
        "I need to check the health of the '{service_name}' service on session '{session_id}'. \
         Please follow these steps:\n\n\
         1. Run `ssh_execute` with command \"systemctl\" and args [\"status\", \"{service_name}\"]\n\
         2. Run `ssh_execute` with command \"journalctl\" and args [\"-u\", \"{service_name}\", \"--no-pager\", \"-n\", \"50\"]\n\
         3. If the service is failed, check its configuration and suggest fixes\n\
         4. Report the service health status and any issues found"
    )))
    .with_description(format!("Check health of service: {service_name}"))
}

fn analyze_auth_failures(session_id: &str) -> GetPromptResult {
    GetPromptResult::new(user_msg(format!(
        "I need to review authentication failures on session '{session_id}'. \
         Please follow these steps:\n\n\
         1. Run `ssh_execute` with command \"grep\" and args [\"Failed\", \"/var/log/auth.log\"]\n\
         2. Run `ssh_execute` with command \"grep\" and args [\"Invalid user\", \"/var/log/auth.log\"]\n\
         3. Run `ssh_execute` with command \"tail\" and args [\"-n\", \"100\", \"/var/log/auth.log\"]\n\
         4. Summarize the attack patterns and suggest security hardening steps"
    )))
    .with_description("Analyze authentication failures")
}

fn network_diagnostics(session_id: &str, target_host: Option<&str>) -> GetPromptResult {
    let mut steps = format!(
        "I need to run network diagnostics on session '{session_id}'. \
         Please follow these steps:\n\n\
         1. Run `ssh_execute` with command \"ip\" and args [\"addr\", \"show\"]\n\
         2. Run `ssh_execute` with command \"ip\" and args [\"route\", \"show\"]\n\
         3. Run `ssh_execute` with command \"cat\" and args [\"/etc/resolv.conf\"]\n"
    );
    if let Some(host) = target_host {
        steps.push_str(&format!(
            "4. Run `ssh_execute` with command \"ping\" and args [\"-c\", \"4\", \"{host}\"]\n\
             5. Run `ssh_execute` with command \"dig\" and args [\"{host}\"]\n"
        ));
    }
    steps.push_str("Analyze the results and report any network issues found.");

    GetPromptResult::new(user_msg(steps)).with_description("Network diagnostics")
}
