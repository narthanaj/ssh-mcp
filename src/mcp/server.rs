use std::sync::Arc;

use rmcp::handler::server::router::tool::ToolRouter;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::{
    AnnotateAble, CallToolResult, GetPromptRequestParams, GetPromptResult, Implementation,
    ListPromptsResult, ListResourcesResult, PaginatedRequestParams, RawContent, RawResource,
    ReadResourceRequestParams, ReadResourceResult, ResourceContents, ServerCapabilities,
    ServerInfo,
};
use rmcp::service::RequestContext;
use rmcp::{ErrorData, RoleServer, ServerHandler, tool, tool_handler, tool_router};

use crate::mcp::params::{ConnectParams, DisconnectParams, ExecParams};
use crate::mcp::prompts;
use crate::ssh::manager::ConnectionManager;

#[derive(Clone)]
pub struct SshMcpServer {
    tool_router: ToolRouter<Self>,
    manager: Arc<ConnectionManager>,
}

impl SshMcpServer {
    pub fn new(manager: Arc<ConnectionManager>) -> Self {
        Self {
            tool_router: Self::tool_router(),
            manager,
        }
    }
}

#[tool_router]
impl SshMcpServer {
    #[tool(
        description = "Connect to a remote host via SSH. Supports ssh-agent, key file, and password authentication. Returns a session_id for subsequent commands."
    )]
    pub async fn ssh_connect(
        &self,
        Parameters(params): Parameters<ConnectParams>,
    ) -> Result<CallToolResult, ErrorData> {
        let port = params.port.unwrap_or(22);

        let result = self
            .manager
            .connect(
                &params.host,
                port,
                &params.username,
                params.key_path.as_deref(),
                params.password.as_deref(),
            )
            .await;

        match result {
            Ok(session_id) => Ok(CallToolResult::success(vec![content_text(format!(
                "Connected to {}@{}:{}. Session ID: {}",
                params.username, params.host, port, session_id
            ))])),
            Err(e) => Err(ErrorData::internal_error(
                format!("SSH connection failed: {e}"),
                None,
            )),
        }
    }

    #[tool(
        description = "Execute a command on a connected SSH session. The command binary must be in the server's allowed list. Arguments are validated against a strict pattern. Returns stdout, stderr, exit code, and execution metadata."
    )]
    pub async fn ssh_execute(
        &self,
        Parameters(params): Parameters<ExecParams>,
    ) -> Result<CallToolResult, ErrorData> {
        let cmd = self
            .manager
            .validate_command(&params.command, &params.args, &params.env)
            .map_err(|e| ErrorData::invalid_params(format!("{e}"), None))?;

        let result = self
            .manager
            .execute(&params.session_id, cmd, params.timeout)
            .await;

        match result {
            Ok(output) => {
                // Always return as successful CallToolResult — even for non-zero exit codes.
                // The LLM needs stdout+stderr+exit_code to self-correct.
                let json = serde_json::to_string_pretty(&output)
                    .unwrap_or_else(|_| format!("{:?}", output));
                Ok(CallToolResult::success(vec![content_text(json)]))
            }
            Err(crate::error::SshMcpError::RateLimited { session_id }) => {
                Err(ErrorData::internal_error(
                    format!(
                        "Rate limited: too many requests for session {session_id}. Wait before retrying."
                    ),
                    None,
                ))
            }
            Err(crate::error::SshMcpError::SessionNotFound { session_id }) => {
                Err(ErrorData::invalid_params(
                    format!("Session not found: {session_id}. Use ssh_connect first."),
                    None,
                ))
            }
            Err(e) => Err(ErrorData::internal_error(
                format!("Execution error: {e}"),
                None,
            )),
        }
    }

    #[tool(description = "Disconnect an SSH session and release its resources.")]
    pub async fn ssh_disconnect(
        &self,
        Parameters(params): Parameters<DisconnectParams>,
    ) -> Result<CallToolResult, ErrorData> {
        match self.manager.disconnect(&params.session_id).await {
            Ok(()) => Ok(CallToolResult::success(vec![content_text(format!(
                "Session {} disconnected.",
                params.session_id
            ))])),
            Err(e) => Err(ErrorData::internal_error(
                format!("Disconnect failed: {e}"),
                None,
            )),
        }
    }

    #[tool(description = "List all active SSH sessions with their connection details.")]
    pub async fn ssh_list_sessions(&self) -> Result<CallToolResult, ErrorData> {
        let sessions = self.manager.list_sessions();
        let json = serde_json::to_string_pretty(&sessions).unwrap_or_else(|_| "[]".to_string());
        Ok(CallToolResult::success(vec![content_text(json)]))
    }
}

#[tool_handler]
impl ServerHandler for SshMcpServer {
    fn get_info(&self) -> ServerInfo {
        let mut info = ServerInfo::default();
        info.capabilities = ServerCapabilities::builder()
            .enable_tools()
            .enable_resources()
            .enable_prompts()
            .build();
        info.server_info = Implementation::new("ssh-mcp", env!("CARGO_PKG_VERSION"));
        info.instructions = Some(
            "SSH MCP server for secure remote command execution. \
             Connect to hosts with ssh_connect, execute commands with ssh_execute, \
             list sessions with ssh_list_sessions, and disconnect with ssh_disconnect. \
             Resources provide direct access to configured remote log files. \
             Prompts provide guided diagnostic workflows."
                .into(),
        );
        info
    }

    async fn list_resources(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListResourcesResult, ErrorData> {
        let config = self.manager.config();
        let resources: Vec<rmcp::model::Resource> = config
            .resources
            .iter()
            .map(|r| {
                RawResource {
                    uri: format!("ssh://resource/{}", r.name),
                    name: r.name.clone(),
                    title: None,
                    description: Some(r.description.clone()),
                    mime_type: Some("text/plain".into()),
                    size: None,
                    icons: None,
                    meta: None,
                }
                .no_annotation()
            })
            .collect();

        Ok(ListResourcesResult::with_all_items(resources))
    }

    async fn read_resource(
        &self,
        request: ReadResourceRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> Result<ReadResourceResult, ErrorData> {
        let uri = &request.uri;

        let resource_name = uri
            .strip_prefix("ssh://resource/")
            .ok_or_else(|| ErrorData::invalid_params("Invalid resource URI format", None))?;

        let config = self.manager.config();
        let resource_def = config
            .resources
            .iter()
            .find(|r| r.name == resource_name)
            .ok_or_else(|| {
                ErrorData::invalid_params(format!("Unknown resource: {resource_name}"), None)
            })?;

        let sessions = self.manager.list_sessions();
        let session = sessions.first().ok_or_else(|| {
            ErrorData::internal_error(
                "No active SSH sessions. Connect first with ssh_connect.",
                None,
            )
        })?;

        let content = self
            .manager
            .read_file(&session.id, &resource_def.path, resource_def.max_bytes)
            .await
            .map_err(|e| {
                ErrorData::internal_error(format!("Failed to read resource: {e}"), None)
            })?;

        Ok(ReadResourceResult::new(vec![
            ResourceContents::TextResourceContents {
                uri: request.uri,
                mime_type: Some("text/plain".into()),
                text: content,
                meta: None,
            },
        ]))
    }

    async fn list_prompts(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListPromptsResult, ErrorData> {
        Ok(ListPromptsResult::with_all_items(prompts::list_prompts()))
    }

    async fn get_prompt(
        &self,
        request: GetPromptRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> Result<GetPromptResult, ErrorData> {
        let args = request.arguments.unwrap_or_default();
        let session_id = args
            .get("session_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                ErrorData::invalid_params("Missing required argument: session_id", None)
            })?;

        let extra_arg = args
            .get("service_name")
            .or_else(|| args.get("target_host"))
            .and_then(|v| v.as_str());

        prompts::get_prompt(&request.name, session_id, extra_arg).ok_or_else(|| {
            ErrorData::invalid_params(format!("Unknown prompt: {}", request.name), None)
        })
    }
}

/// Helper to create annotated text content for tool results.
fn content_text(text: String) -> rmcp::model::Content {
    use rmcp::model::AnnotateAble;
    RawContent::text(text).no_annotation()
}
