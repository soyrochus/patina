use crate::auth::{AuthCoordinator, AuthMode, AuthState};
use anyhow::{anyhow, Context, Result};
use rmcp::model::{
    CallToolRequestParam, CallToolResult, ClientCapabilities, ClientInfo,
    CreateMessageRequestMethod, ElicitationCreateRequestMethod, InitializeResult, JsonObject,
    ListRootsResult, ServerNotification, ServerRequest, Tool,
};
use rmcp::service::QuitReason;
use rmcp::service::{self, Peer, RoleClient, RunningServiceCancellationToken};
use rmcp::transport::child_process::TokioChildProcess;
use rmcp::ErrorData;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::borrow::Cow;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::process::Command;
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};
use tokio::sync::{Mutex, RwLock};
use tracing::{info, warn};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum McpEvent {
    Connected {
        endpoint: String,
        mode: AuthMode,
        #[serde(skip_serializing_if = "Option::is_none")]
        server_info: Option<Value>,
    },
    Disconnected {
        endpoint: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        reason: Option<String>,
    },
    ToolInvoked {
        endpoint: String,
        tool: String,
        arguments: Value,
        result: Value,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandSpec {
    pub program: String,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub env: HashMap<String, String>,
    #[serde(default)]
    pub current_dir: Option<PathBuf>,
}

impl CommandSpec {
    pub fn new(program: impl Into<String>) -> Self {
        Self {
            program: program.into(),
            args: Vec::new(),
            env: HashMap::new(),
            current_dir: None,
        }
    }

    pub fn with_args<I, S>(mut self, args: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.args = args.into_iter().map(Into::into).collect();
        self
    }

    pub fn push_arg(mut self, arg: impl Into<String>) -> Self {
        self.args.push(arg.into());
        self
    }

    pub fn with_env(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.env.insert(key.into(), value.into());
        self
    }

    pub fn with_current_dir(mut self, dir: impl Into<PathBuf>) -> Self {
        self.current_dir = Some(dir.into());
        self
    }

    pub fn to_command(&self) -> Command {
        let mut cmd = Command::new(&self.program);
        cmd.args(&self.args);
        for (key, value) in &self.env {
            cmd.env(key, value);
        }
        if let Some(dir) = &self.current_dir {
            cmd.current_dir(dir);
        }
        cmd
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpEndpoint {
    pub id: String,
    pub mode: AuthMode,
    pub command: CommandSpec,
}

impl McpEndpoint {
    pub fn child_process(id: impl Into<String>, mode: AuthMode, command: CommandSpec) -> Self {
        Self {
            id: id.into(),
            mode,
            command,
        }
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn mode(&self) -> &AuthMode {
        &self.mode
    }
}

#[derive(Default)]
struct ClientConnectionState {
    inner: RwLock<Option<ConnectedState>>,
}

impl ClientConnectionState {
    async fn peer(&self) -> Option<Peer<RoleClient>> {
        self.inner
            .read()
            .await
            .as_ref()
            .map(|state| state.peer.clone())
    }

    async fn server_info(&self) -> Option<InitializeResult> {
        self.inner
            .read()
            .await
            .as_ref()
            .map(|state| state.server_info.clone())
    }

    async fn set(&self, state: ConnectedState) {
        *self.inner.write().await = Some(state);
    }

    async fn take(&self) -> Option<ConnectedState> {
        self.inner.write().await.take()
    }
}

struct ConnectedState {
    peer: Peer<RoleClient>,
    cancel: RunningServiceCancellationToken,
    server_info: InitializeResult,
}

#[derive(Clone)]
pub struct McpClient {
    endpoint: Arc<McpEndpoint>,
    auth: AuthCoordinator,
    events_tx: UnboundedSender<McpEvent>,
    state: Arc<ClientConnectionState>,
    connect_lock: Arc<Mutex<()>>,
}

impl McpClient {
    pub fn new(
        endpoint: McpEndpoint,
        auth: AuthCoordinator,
    ) -> (Self, UnboundedReceiver<McpEvent>) {
        let (events_tx, events_rx) = unbounded_channel();
        (
            Self {
                endpoint: Arc::new(endpoint),
                auth,
                events_tx,
                state: Arc::new(ClientConnectionState::default()),
                connect_lock: Arc::new(Mutex::new(())),
            },
            events_rx,
        )
    }

    pub async fn handshake(&self) -> Result<AuthState> {
        let auth_state = self
            .auth
            .negotiate(self.endpoint.id(), self.endpoint.mode().clone())
            .await?;

        let server_info = self.ensure_connected(auth_state.clone()).await?;
        let server_json = serde_json::to_value(&server_info).ok();
        self.events_tx
            .send(McpEvent::Connected {
                endpoint: self.endpoint.id.clone(),
                mode: self.endpoint.mode.clone(),
                server_info: server_json,
            })
            .ok();
        Ok(auth_state)
    }

    pub async fn disconnect(&self) -> Result<()> {
        if let Some(state) = self.state.take().await {
            state.cancel.cancel();
        }
        Ok(())
    }

    pub async fn call_tool(&self, tool: &str, arguments: Option<Value>) -> Result<CallToolResult> {
        let peer = self
            .state
            .peer()
            .await
            .ok_or_else(|| anyhow!("MCP client is not connected"))?;

        let args_value = arguments.unwrap_or(Value::Null);
        let arguments_map: Option<JsonObject> = match &args_value {
            Value::Null => None,
            Value::Object(map) => Some(map.clone()),
            other => {
                return Err(anyhow!(
                    "tool arguments must be a JSON object, received {:?}",
                    other
                ))
            }
        };

        let result = peer
            .call_tool(CallToolRequestParam {
                name: Cow::Owned(tool.to_owned()),
                arguments: arguments_map,
            })
            .await
            .with_context(|| format!("failed to call tool '{tool}'"))?;

        let result_json =
            serde_json::to_value(&result).context("serialize tool result for event dispatch")?;
        self.events_tx
            .send(McpEvent::ToolInvoked {
                endpoint: self.endpoint.id.clone(),
                tool: tool.to_owned(),
                arguments: args_value,
                result: result_json,
            })
            .ok();
        Ok(result)
    }

    pub async fn list_tools(&self) -> Result<Vec<Tool>> {
        let peer = self
            .state
            .peer()
            .await
            .ok_or_else(|| anyhow!("MCP client is not connected"))?;
        peer.list_all_tools()
            .await
            .map_err(|err| anyhow!("failed to list tools: {err}"))
    }

    pub fn endpoint(&self) -> &str {
        self.endpoint.id()
    }

    pub fn mode(&self) -> &AuthMode {
        self.endpoint.mode()
    }

    async fn ensure_connected(&self, auth_state: AuthState) -> Result<InitializeResult> {
        if let Some(info) = self.state.server_info().await {
            return Ok(info);
        }
        let _guard = self.connect_lock.lock().await;
        if let Some(info) = self.state.server_info().await {
            return Ok(info);
        }
        self.establish_connection(auth_state).await
    }

    async fn establish_connection(&self, auth_state: AuthState) -> Result<InitializeResult> {
        let handler = PatinaClientHandler::new(
            self.endpoint.id.clone(),
            self.endpoint.mode.clone(),
            auth_state,
        );

        let transport = TokioChildProcess::new(self.endpoint.command.to_command())
            .with_context(|| format!("failed to spawn MCP transport for '{}'", self.endpoint.id))?;
        let service = service::serve_client(handler, transport)
            .await
            .with_context(|| format!("failed to initialize MCP client '{}':", self.endpoint.id))?;

        let peer = service.peer().clone();
        let server_info = peer
            .peer_info()
            .cloned()
            .unwrap_or_else(InitializeResult::default);
        let cancel = service.cancellation_token();
        let endpoint = self.endpoint.id.clone();
        let events = self.events_tx.clone();
        tokio::spawn(async move {
            let reason = match service.waiting().await {
                Ok(reason) => format_quit_reason(reason),
                Err(err) => Some(format!("task join error: {err}")),
            };
            let _ = events.send(McpEvent::Disconnected { endpoint, reason });
        });

        self.state
            .set(ConnectedState {
                peer,
                cancel,
                server_info: server_info.clone(),
            })
            .await;
        Ok(server_info)
    }
}

struct PatinaClientHandler {
    endpoint_id: String,
    #[allow(dead_code)]
    mode: AuthMode,
    #[allow(dead_code)]
    auth_state: AuthState,
    client_info: ClientInfo,
}

impl PatinaClientHandler {
    fn new(endpoint_id: String, mode: AuthMode, auth_state: AuthState) -> Self {
        let mut client_info = ClientInfo::default();
        client_info.client_info.name = "patina-desktop".to_string();
        client_info.client_info.title = Some("Patina Desktop Client".to_string());
        client_info.client_info.version = env!("CARGO_PKG_VERSION").to_string();
        client_info.capabilities = ClientCapabilities::builder().build();
        Self {
            endpoint_id: endpoint_id.clone(),
            mode,
            auth_state,
            client_info,
        }
    }
}

impl service::Service<RoleClient> for PatinaClientHandler {
    fn handle_request(
        &self,
        request: ServerRequest,
        _context: service::RequestContext<RoleClient>,
    ) -> impl std::future::Future<Output = Result<rmcp::model::ClientResult, ErrorData>> + Send
    {
        async move {
            match request {
                ServerRequest::PingRequest(_) => Ok(rmcp::model::ClientResult::empty(())),
                ServerRequest::ListRootsRequest(_) => Ok(
                    rmcp::model::ClientResult::ListRootsResult(ListRootsResult::default()),
                ),
                ServerRequest::CreateMessageRequest(_) => {
                    Err(ErrorData::method_not_found::<CreateMessageRequestMethod>())
                }
                ServerRequest::CreateElicitationRequest(_) => {
                    Err(ErrorData::method_not_found::<ElicitationCreateRequestMethod>())
                }
            }
        }
    }

    fn handle_notification(
        &self,
        notification: ServerNotification,
        _context: service::NotificationContext<RoleClient>,
    ) -> impl std::future::Future<Output = Result<(), ErrorData>> + Send {
        async move {
            match notification {
                ServerNotification::LoggingMessageNotification(msg) => {
                    warn!(target = "mcp.logging", endpoint = %self.endpoint_id, ?msg, "Server log message");
                }
                ServerNotification::ProgressNotification(_) => {}
                ServerNotification::CancelledNotification(_) => {}
                other => {
                    warn!(target = "mcp.notification", endpoint = %self.endpoint_id, ?other, "Unhandled server notification");
                }
            }
            Ok(())
        }
    }

    fn get_info(&self) -> ClientInfo {
        self.client_info.clone()
    }
}

fn format_quit_reason(reason: QuitReason) -> Option<String> {
    match reason {
        QuitReason::Cancelled => Some("cancelled".to_string()),
        QuitReason::Closed => Some("transport closed".to_string()),
        QuitReason::JoinError(err) => Some(format!("join error: {err}")),
    }
}

pub struct McpRegistry {
    auth: AuthCoordinator,
    clients: Vec<Arc<McpClient>>,
}

impl McpRegistry {
    pub fn new(auth: AuthCoordinator) -> Self {
        Self {
            auth,
            clients: Vec::new(),
        }
    }

    pub async fn register(
        &mut self,
        endpoint: McpEndpoint,
    ) -> Result<(Arc<McpClient>, UnboundedReceiver<McpEvent>)> {
        let mode = endpoint.mode.clone();
        let id = endpoint.id.clone();
        let (client, rx) = McpClient::new(endpoint, self.auth.clone());
        let client = Arc::new(client);
        client.handshake().await?;
        info!(endpoint = %id, mode = ?mode, "Registered MCP client");
        self.clients.push(client.clone());
        Ok((client, rx))
    }

    pub fn clients(&self) -> &[Arc<McpClient>] {
        &self.clients
    }
}
