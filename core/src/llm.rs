use crate::state::{ChatMessage, MessageRole};
use anyhow::Result;
use async_trait::async_trait;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::time::{sleep, Duration};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum LlmProviderKind {
    OpenAi,
    AzureOpenAi,
    Mock,
}

impl LlmProviderKind {
    pub fn from_environment() -> Self {
        match std::env::var("LLM_PROVIDER") {
            Ok(value) if value.eq_ignore_ascii_case("azure_openai") => Self::AzureOpenAi,
            Ok(value) if value.eq_ignore_ascii_case("mock") => Self::Mock,
            _ => Self::OpenAi,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmConfig {
    pub provider: LlmProviderKind,
    pub model: Option<String>,
}

impl LlmConfig {
    pub fn new(provider: LlmProviderKind, model: Option<String>) -> Self {
        Self { provider, model }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatResponse {
    pub message: ChatMessage,
    pub usage: Option<ModelUsage>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ModelUsage {
    pub prompt_tokens: usize,
    pub completion_tokens: usize,
}

#[async_trait]
pub trait LanguageModelProvider: Send + Sync {
    async fn send_chat(&self, messages: &[ChatMessage], config: &LlmConfig)
        -> Result<ChatResponse>;
}

#[derive(Clone)]
pub struct LlmDriver {
    config: LlmConfig,
    provider: Arc<dyn LanguageModelProvider>,
}

impl LlmDriver {
    pub fn new(config: LlmConfig, provider: Arc<dyn LanguageModelProvider>) -> Self {
        Self { config, provider }
    }

    pub async fn from_environment() -> Self {
        let provider = LlmProviderKind::from_environment();
        Self::with_provider(provider, None).await
    }

    pub async fn with_provider(provider: LlmProviderKind, model: Option<String>) -> Self {
        match provider {
            LlmProviderKind::OpenAi => {
                let config = LlmConfig::new(
                    provider,
                    model.or_else(|| std::env::var("OPENAI_MODEL").ok()),
                );
                Self::new(config, Arc::new(OpenAiProvider))
            }
            LlmProviderKind::AzureOpenAi => {
                let config = LlmConfig::new(
                    provider,
                    model.or_else(|| std::env::var("AZURE_OPENAI_DEPLOYMENT_NAME").ok()),
                );
                Self::new(config, Arc::new(AzureOpenAiProvider))
            }
            LlmProviderKind::Mock => {
                let config = LlmConfig::new(provider, model);
                Self::new(config, Arc::new(MockProvider::default()))
            }
        }
    }

    pub async fn fake() -> Self {
        Self::with_provider(LlmProviderKind::Mock, Some("mock".into())).await
    }

    pub fn provider_kind(&self) -> LlmProviderKind {
        self.config.provider.clone()
    }

    pub async fn respond(&self, history: &[ChatMessage]) -> Result<ChatResponse> {
        self.provider.send_chat(history, &self.config).await
    }
}

struct OpenAiProvider;
struct AzureOpenAiProvider;

#[derive(Default)]
struct MockProvider;

#[async_trait]
impl LanguageModelProvider for OpenAiProvider {
    async fn send_chat(
        &self,
        messages: &[ChatMessage],
        config: &LlmConfig,
    ) -> Result<ChatResponse> {
        synthetic_response("OpenAI", messages, config).await
    }
}

#[async_trait]
impl LanguageModelProvider for AzureOpenAiProvider {
    async fn send_chat(
        &self,
        messages: &[ChatMessage],
        config: &LlmConfig,
    ) -> Result<ChatResponse> {
        synthetic_response("Azure OpenAI", messages, config).await
    }
}

#[async_trait]
impl LanguageModelProvider for MockProvider {
    async fn send_chat(
        &self,
        messages: &[ChatMessage],
        config: &LlmConfig,
    ) -> Result<ChatResponse> {
        synthetic_response("Mock", messages, config).await
    }
}

async fn synthetic_response(
    provider_name: &str,
    messages: &[ChatMessage],
    config: &LlmConfig,
) -> Result<ChatResponse> {
    sleep(Duration::from_millis(20)).await;
    let prompt = messages
        .iter()
        .rev()
        .find(|msg| msg.role == MessageRole::User)
        .map(|msg| msg.content.clone())
        .unwrap_or_else(|| "How can I help you today?".to_string());
    let reply = format!(
        "[{provider_name}] Model {:?}: received '{}'.",
        config.model.as_deref().unwrap_or("default"),
        prompt
    );
    let message = ChatMessage {
        id: Uuid::new_v4(),
        role: MessageRole::Assistant,
        content: reply,
        created_at: Utc::now(),
        tool_calls: Vec::new(),
    };
    Ok(ChatResponse {
        message,
        usage: Some(ModelUsage {
            prompt_tokens: messages.len() * 10,
            completion_tokens: 25,
        }),
    })
}
