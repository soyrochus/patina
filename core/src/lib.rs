pub mod auth;
pub mod config;
pub mod llm;
pub mod mcp;
pub mod project;
pub mod state;
pub mod store;
pub mod telemetry;

#[cfg(test)]
mod llm_streaming_test;

pub use auth::{AuthCoordinator, AuthMode, AuthState};
pub use llm::{LlmDriver, LlmProviderKind, LlmStatus, StreamChunk};
pub use mcp::{CommandSpec, McpClient, McpEndpoint, McpEvent};
pub use project::{ProjectHandle, ProjectPaths};
pub use state::{AppState, ChatMessage, Conversation, MessageRole};
pub use store::TranscriptStore;
