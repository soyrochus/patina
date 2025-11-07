use crate::auth::{AuthCoordinator, AuthMode, AuthState};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::Arc;
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};
use tracing::info;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum McpEvent {
    Connected {
        endpoint: String,
        mode: AuthMode,
    },
    ToolInvoked {
        endpoint: String,
        tool: String,
        payload: Value,
    },
}

#[derive(Clone)]
pub struct McpClient {
    endpoint: String,
    mode: AuthMode,
    auth: AuthCoordinator,
    events_tx: UnboundedSender<McpEvent>,
}

impl McpClient {
    pub fn new(
        endpoint: impl Into<String>,
        mode: AuthMode,
        auth: AuthCoordinator,
    ) -> (Self, UnboundedReceiver<McpEvent>) {
        let endpoint = endpoint.into();
        let (events_tx, events_rx) = unbounded_channel();
        (
            Self {
                endpoint,
                mode,
                auth,
                events_tx,
            },
            events_rx,
        )
    }

    pub async fn handshake(&self) -> Result<AuthState> {
        let state = self
            .auth
            .negotiate(&self.endpoint, self.mode.clone())
            .await?;
        self.events_tx
            .send(McpEvent::Connected {
                endpoint: self.endpoint.clone(),
                mode: self.mode.clone(),
            })
            .ok();
        Ok(state)
    }

    pub async fn simulate_tool_call(&self, tool: &str, payload: Value) -> Result<()> {
        self.events_tx
            .send(McpEvent::ToolInvoked {
                endpoint: self.endpoint.clone(),
                tool: tool.to_owned(),
                payload,
            })
            .ok();
        Ok(())
    }

    pub fn endpoint(&self) -> &str {
        &self.endpoint
    }

    pub fn mode(&self) -> &AuthMode {
        &self.mode
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
        endpoint: &str,
        mode: AuthMode,
    ) -> Result<(Arc<McpClient>, UnboundedReceiver<McpEvent>)> {
        let mode_for_log = mode.clone();
        let (client, rx) = McpClient::new(endpoint.to_owned(), mode, self.auth.clone());
        let client = Arc::new(client);
        client.handshake().await?;
        info!("endpoint" = endpoint, "mode" = ?mode_for_log, "Registered MCP client");
        self.clients.push(client.clone());
        Ok((client, rx))
    }

    pub fn clients(&self) -> &[Arc<McpClient>] {
        &self.clients
    }
}
