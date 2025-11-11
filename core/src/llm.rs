use crate::config::AiRuntimeSettings;
use crate::state::{ChatMessage, MessageRole};
use anyhow::{anyhow, bail, Context, Result};
use async_trait::async_trait;
use chrono::Utc;
use futures::StreamExt;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio::time::{sleep, Duration};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum LlmProviderKind {
    OpenAi,
    AzureOpenAi,
    Mock,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmConfig {
    pub provider: LlmProviderKind,
    pub model: Option<String>,
    pub temperature: Option<f32>,
}

impl LlmConfig {
    pub fn new(provider: LlmProviderKind, model: Option<String>) -> Self {
        Self {
            provider,
            model,
            temperature: None,
        }
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

#[derive(Debug, Clone)]
pub enum LlmStatus {
    Ready,
    Unconfigured(String),
}

#[derive(Debug, Clone)]
pub struct StreamChunk {
    pub delta: String,
    pub done: bool,
}

#[async_trait]
pub trait LanguageModelProvider: Send + Sync {
    async fn send_chat(&self, messages: &[ChatMessage], config: &LlmConfig)
        -> Result<ChatResponse>;
    
    async fn send_chat_stream(
        &self,
        messages: &[ChatMessage],
        config: &LlmConfig,
    ) -> Result<mpsc::UnboundedReceiver<Result<StreamChunk>>>;
}

#[derive(Clone)]
pub struct LlmDriver {
    config: Option<LlmConfig>,
    provider: Option<Arc<dyn LanguageModelProvider>>,
    status: LlmStatus,
}

impl LlmDriver {
    pub async fn from_environment() -> Self {
        match AiRuntimeSettings::load() {
            Ok(settings) => match Self::from_settings(settings).await {
                Ok(driver) => driver,
                Err(err) => Self::unconfigured(err.to_string()),
            },
            Err(err) => Self::unconfigured(err.user_message()),
        }
    }

    async fn from_settings(settings: AiRuntimeSettings) -> Result<Self> {
        let client = Client::builder().build()?;
        match settings.provider {
            LlmProviderKind::OpenAi => {
                let creds = settings
                    .openai
                    .ok_or_else(|| anyhow!("OpenAI credentials missing after resolution"))?;
                let model = creds
                    .model
                    .clone()
                    .unwrap_or_else(|| "gpt-4o-mini".to_string());
                let provider =
                    OpenAiChatProvider::openai(client.clone(), creds.api_key, model.clone());
                Ok(Self::ready(
                    LlmConfig::new(LlmProviderKind::OpenAi, Some(model)),
                    Arc::new(provider),
                ))
            }
            LlmProviderKind::AzureOpenAi => {
                let creds = settings
                    .azure
                    .ok_or_else(|| anyhow!("Azure OpenAI credentials missing after resolution"))?;
                let deployment = creds.deployment_name.clone();
                let provider = OpenAiChatProvider::azure(
                    client.clone(),
                    creds.endpoint,
                    creds.api_key,
                    creds.api_version,
                    deployment.clone(),
                );
                Ok(Self::ready(
                    LlmConfig::new(LlmProviderKind::AzureOpenAi, Some(deployment)),
                    Arc::new(provider),
                ))
            }
            LlmProviderKind::Mock => Ok(Self::configured_mock(settings.model)),
        }
    }

    pub async fn with_provider(provider: LlmProviderKind, model: Option<String>) -> Self {
        match provider {
            LlmProviderKind::Mock => Self::configured_mock(model),
            _ => Self::from_environment().await,
        }
    }

    pub async fn fake() -> Self {
        Self::configured_mock(Some("mock".into()))
    }

    pub fn provider_kind(&self) -> Option<LlmProviderKind> {
        self.config.as_ref().map(|cfg| cfg.provider)
    }

    pub fn status(&self) -> LlmStatus {
        self.status.clone()
    }

    pub async fn respond(
        &self,
        history: &[ChatMessage],
        model_override: Option<&str>,
        temperature: Option<f32>,
    ) -> Result<ChatResponse> {
        match (&self.provider, &self.config) {
            (Some(provider), Some(config)) => {
                let mut effective = config.clone();
                if let Some(model) = model_override {
                    effective.model = Some(model.to_string());
                }
                effective.temperature = temperature;
                provider.send_chat(history, &effective).await
            }
            _ => {
                let message = match &self.status {
                    LlmStatus::Ready => "AI driver not initialized".to_string(),
                    LlmStatus::Unconfigured(msg) => msg.clone(),
                };
                bail!(message);
            }
        }
    }

    pub async fn respond_streaming(
        &self,
        history: &[ChatMessage],
        model_override: Option<&str>,
        temperature: Option<f32>,
    ) -> Result<mpsc::UnboundedReceiver<Result<StreamChunk>>> {
        match (&self.provider, &self.config) {
            (Some(provider), Some(config)) => {
                let mut effective = config.clone();
                if let Some(model) = model_override {
                    effective.model = Some(model.to_string());
                }
                effective.temperature = temperature;
                provider.send_chat_stream(history, &effective).await
            }
            _ => {
                let message = match &self.status {
                    LlmStatus::Ready => "AI driver not initialized".to_string(),
                    LlmStatus::Unconfigured(msg) => msg.clone(),
                };
                bail!(message);
            }
        }
    }

    fn ready(config: LlmConfig, provider: Arc<dyn LanguageModelProvider>) -> Self {
        Self {
            config: Some(config),
            provider: Some(provider),
            status: LlmStatus::Ready,
        }
    }

    fn unconfigured(message: impl Into<String>) -> Self {
        Self {
            config: None,
            provider: None,
            status: LlmStatus::Unconfigured(message.into()),
        }
    }

    fn configured_mock(model: Option<String>) -> Self {
        Self::ready(
            LlmConfig::new(LlmProviderKind::Mock, model),
            Arc::new(MockProvider),
        )
    }
}

struct OpenAiChatProvider {
    client: Client,
    backend: OpenAiBackend,
}

impl OpenAiChatProvider {
    fn openai(client: Client, api_key: String, model: String) -> Self {
        Self {
            client,
            backend: OpenAiBackend::OpenAi { api_key, model },
        }
    }

    fn azure(
        client: Client,
        endpoint: String,
        api_key: String,
        api_version: String,
        deployment: String,
    ) -> Self {
        Self {
            client,
            backend: OpenAiBackend::Azure {
                api_key,
                endpoint,
                api_version,
                deployment,
            },
        }
    }
}

enum OpenAiBackend {
    OpenAi {
        api_key: String,
        model: String,
    },
    Azure {
        api_key: String,
        endpoint: String,
        api_version: String,
        deployment: String,
    },
}

impl OpenAiBackend {
    fn label(&self) -> &'static str {
        match self {
            Self::OpenAi { .. } => "OpenAI",
            Self::Azure { .. } => "Azure OpenAI",
        }
    }

    fn request_builder(&self, client: &Client) -> reqwest::RequestBuilder {
        match self {
            Self::OpenAi { api_key, .. } => client
                .post("https://api.openai.com/v1/chat/completions")
                .bearer_auth(api_key),
            Self::Azure {
                api_key,
                endpoint,
                api_version,
                deployment,
            } => {
                let base = endpoint.trim_end_matches('/');
                let url = format!(
                    "{base}/openai/deployments/{deployment}/chat/completions?api-version={api_version}",
                    base = base,
                    deployment = deployment,
                    api_version = api_version
                );
                client.post(url).header("api-key", api_key)
            }
        }
    }

    fn request_model(&self) -> Option<&str> {
        match self {
            Self::OpenAi { model, .. } => Some(model.as_str()),
            Self::Azure { .. } => None,
        }
    }
}

#[async_trait]
impl LanguageModelProvider for OpenAiChatProvider {
    async fn send_chat(
        &self,
        messages: &[ChatMessage],
        config: &LlmConfig,
    ) -> Result<ChatResponse> {
        let payload = ChatCompletionRequest {
            model: config
                .model
                .clone()
                .or_else(|| self.backend.request_model().map(|model| model.to_string())),
            temperature: config.temperature,
            messages: map_messages(messages),
        };
        let response = self
            .backend
            .request_builder(&self.client)
            .json(&payload)
            .send()
            .await
            .with_context(|| format!("{} request failed", self.backend.label()))?
            .error_for_status()
            .with_context(|| format!("{} returned an error status", self.backend.label()))?;
        let payload: ChatCompletionResponse = response
            .json()
            .await
            .with_context(|| format!("{} response decoding failed", self.backend.label()))?;
        completion_to_chat(payload, config)
    }

    async fn send_chat_stream(
        &self,
        messages: &[ChatMessage],
        config: &LlmConfig,
    ) -> Result<mpsc::UnboundedReceiver<Result<StreamChunk>>> {
        let (tx, rx) = mpsc::unbounded_channel();

        let payload = ChatCompletionStreamRequest {
            model: config
                .model
                .clone()
                .or_else(|| self.backend.request_model().map(|model| model.to_string())),
            temperature: config.temperature,
            messages: map_messages(messages),
            stream: true,
        };

        let response = self
            .backend
            .request_builder(&self.client)
            .json(&payload)
            .send()
            .await
            .with_context(|| format!("{} streaming request failed", self.backend.label()))?
            .error_for_status()
            .with_context(|| format!("{} returned an error status", self.backend.label()))?;

        let backend_label = self.backend.label();
        tokio::spawn(async move {
            let mut stream = response.bytes_stream();
            let mut buffer = String::new();

            while let Some(chunk) = stream.next().await {
                match chunk {
                    Ok(bytes) => {
                        buffer.push_str(&String::from_utf8_lossy(&bytes));

                        // Process complete lines from buffer
                        while let Some(line_end) = buffer.find('\n') {
                            let line = buffer[..line_end].trim().to_string();
                            buffer = buffer[line_end + 1..].to_string();

                            if line.is_empty() {
                                continue;
                            }

                            if let Some(json_str) = line.strip_prefix("data: ") {
                                if json_str == "[DONE]" {
                                    let _ = tx.send(Ok(StreamChunk {
                                        delta: String::new(),
                                        done: true,
                                    }));
                                    return;
                                }

                                match serde_json::from_str::<ChatCompletionStreamResponse>(json_str)
                                {
                                    Ok(chunk_response) => {
                                        if let Some(choice) = chunk_response.choices.first() {
                                            if let Some(content) = &choice.delta.content {
                                                let _ = tx.send(Ok(StreamChunk {
                                                    delta: content.clone(),
                                                    done: false,
                                                }));
                                            }
                                            if choice.finish_reason.is_some() {
                                                let _ = tx.send(Ok(StreamChunk {
                                                    delta: String::new(),
                                                    done: true,
                                                }));
                                                return;
                                            }
                                        }
                                    }
                                    Err(e) => {
                                        tracing::warn!(
                                            error = ?e,
                                            json = json_str,
                                            "{} failed to parse stream chunk",
                                            backend_label
                                        );
                                    }
                                }
                            }
                        }
                    }
                    Err(e) => {
                        let _ = tx.send(Err(anyhow!("{} stream error: {}", backend_label, e)));
                        return;
                    }
                }
            }

            // Stream ended without [DONE] marker
            let _ = tx.send(Ok(StreamChunk {
                delta: String::new(),
                done: true,
            }));
        });

        Ok(rx)
    }
}

#[derive(Default)]
struct MockProvider;

#[async_trait]
impl LanguageModelProvider for MockProvider {
    async fn send_chat(
        &self,
        messages: &[ChatMessage],
        config: &LlmConfig,
    ) -> Result<ChatResponse> {
        synthetic_response("Mock", messages, config).await
    }

    async fn send_chat_stream(
        &self,
        messages: &[ChatMessage],
        config: &LlmConfig,
    ) -> Result<mpsc::UnboundedReceiver<Result<StreamChunk>>> {
        let (tx, rx) = mpsc::unbounded_channel();

        let prompt = messages
            .iter()
            .rev()
            .find(|msg| msg.role == MessageRole::User)
            .map(|msg| msg.content.clone())
            .unwrap_or_else(|| "How can I help you today?".to_string());

        let reply = format!(
            "[Mock] Model {:?} (temp {:?}): received '{}'.",
            config.model.as_deref().unwrap_or("default"),
            config.temperature,
            prompt
        );

        tokio::spawn(async move {
            // Simulate streaming by sending chunks character by character
            for chunk in reply.chars().collect::<Vec<_>>().chunks(5) {
                sleep(Duration::from_millis(20)).await;
                let delta: String = chunk.iter().collect();
                if tx
                    .send(Ok(StreamChunk {
                        delta,
                        done: false,
                    }))
                    .is_err()
                {
                    return;
                }
            }

            // Send completion marker
            let _ = tx.send(Ok(StreamChunk {
                delta: String::new(),
                done: true,
            }));
        });

        Ok(rx)
    }
}

#[derive(Serialize)]
struct ChatCompletionRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    messages: Vec<CompletionRequestMessage>,
}

#[derive(Serialize)]
struct CompletionRequestMessage {
    role: String,
    content: String,
}

#[derive(Deserialize)]
struct ChatCompletionResponse {
    choices: Vec<CompletionChoice>,
    usage: Option<CompletionUsage>,
}

#[derive(Deserialize)]
struct CompletionChoice {
    message: CompletionResponseMessage,
}

#[derive(Deserialize)]
struct CompletionResponseMessage {
    #[allow(dead_code)]
    role: String,
    content: Option<String>,
}

#[derive(Deserialize)]
struct CompletionUsage {
    prompt_tokens: Option<usize>,
    completion_tokens: Option<usize>,
}

#[derive(Serialize)]
struct ChatCompletionStreamRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    messages: Vec<CompletionRequestMessage>,
    stream: bool,
}

#[derive(Deserialize)]
struct ChatCompletionStreamResponse {
    choices: Vec<StreamChoice>,
}

#[derive(Deserialize)]
struct StreamChoice {
    delta: StreamDelta,
    finish_reason: Option<String>,
}

#[derive(Deserialize)]
struct StreamDelta {
    content: Option<String>,
}

fn map_messages(messages: &[ChatMessage]) -> Vec<CompletionRequestMessage> {
    messages
        .iter()
        .map(|message| CompletionRequestMessage {
            role: api_role(&message.role),
            content: message.content.clone(),
        })
        .collect()
}

fn api_role(role: &MessageRole) -> String {
    match role {
        MessageRole::System => "system",
        MessageRole::User => "user",
        MessageRole::Assistant => "assistant",
        MessageRole::Tool => "tool",
    }
    .to_string()
}

fn completion_to_chat(
    payload: ChatCompletionResponse,
    _config: &LlmConfig,
) -> Result<ChatResponse> {
    let choice = payload
        .choices
        .into_iter()
        .next()
        .ok_or_else(|| anyhow!("response contained no choices"))?;
    let content = choice
        .message
        .content
        .unwrap_or_else(|| "[empty response]".to_string());
    let reply = ChatMessage {
        id: Uuid::new_v4(),
        role: MessageRole::Assistant,
        content,
        created_at: Utc::now(),
        tool_calls: Vec::new(),
    };
    let usage = payload.usage.map(|usage| ModelUsage {
        prompt_tokens: usage.prompt_tokens.unwrap_or(0),
        completion_tokens: usage.completion_tokens.unwrap_or(0),
    });
    Ok(ChatResponse {
        message: reply,
        usage,
    })
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
        "[{provider_name}] Model {:?} (temp {:?}): received '{}'.",
        config.model.as_deref().unwrap_or("default"),
        config.temperature,
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
