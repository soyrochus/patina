pub mod auth;
pub mod llm;
pub mod mcp;
pub mod state;
pub mod store;
pub mod telemetry;

pub use auth::{AuthCoordinator, AuthMode, AuthState};
pub use llm::{LlmDriver, LlmProviderKind};
pub use mcp::{McpClient, McpEvent};
pub use state::{AppState, ChatMessage, Conversation, MessageRole};
pub use store::TranscriptStore;
