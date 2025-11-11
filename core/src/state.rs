use crate::llm::{LlmDriver, LlmStatus, StreamChunk};
use crate::project::ProjectHandle;
use crate::store::TranscriptStore;
use anyhow::Result;
use chrono::{DateTime, Utc};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::Arc;
use tokio::sync::mpsc;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum MessageRole {
    System,
    User,
    Assistant,
    Tool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ToolCallStatus {
    Pending,
    Completed,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: Uuid,
    pub name: String,
    #[serde(default)]
    pub arguments: Value,
    pub status: ToolCallStatus,
    #[serde(default)]
    pub response: Option<Value>,
}

impl ToolCall {
    pub fn new(name: impl Into<String>, arguments: serde_json::Value) -> Self {
        Self {
            id: Uuid::new_v4(),
            name: name.into(),
            arguments,
            status: ToolCallStatus::Pending,
            response: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub id: Uuid,
    pub role: MessageRole,
    pub content: String,
    pub created_at: DateTime<Utc>,
    #[serde(default)]
    pub tool_calls: Vec<ToolCall>,
}

impl ChatMessage {
    pub fn new(role: MessageRole, content: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4(),
            role,
            content: content.into(),
            created_at: Utc::now(),
            tool_calls: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Conversation {
    pub id: Uuid,
    pub title: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub messages: Vec<ChatMessage>,
}

impl Conversation {
    pub fn new() -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            title: "New chat".to_string(),
            created_at: now,
            updated_at: now,
            messages: Vec::new(),
        }
    }

    pub fn with_id(id: Uuid, title: impl Into<String>) -> Self {
        let now = Utc::now();
        Self {
            id,
            title: title.into(),
            created_at: now,
            updated_at: now,
            messages: Vec::new(),
        }
    }

    pub fn add_message(&mut self, mut message: ChatMessage) -> bool {
        let mut title_changed = false;
        if self.messages.is_empty() && message.role == MessageRole::User {
            self.title = snippet(&message.content);
            title_changed = true;
        }
        if message.tool_calls.is_empty() {
            message.tool_calls = Vec::new();
        }
        self.messages.push(message);
        self.updated_at = Utc::now();
        title_changed
    }
}

impl Default for Conversation {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationSummary {
    pub id: Uuid,
    pub title: String,
    pub updated_at: DateTime<Utc>,
    pub message_count: usize,
}

#[derive(Clone)]
pub struct AppState {
    inner: Arc<RwLock<InnerState>>,
    store: TranscriptStore,
    llm: LlmDriver,
    project: ProjectHandle,
}

#[derive(Default)]
struct InnerState {
    conversations: Vec<Conversation>,
    current_session: Option<Uuid>,
}

impl AppState {
    pub fn new(project: ProjectHandle, llm: LlmDriver) -> Self {
        let store = project.transcript_store();
        Self::with_store(project, store, llm)
    }

    pub fn with_store(project: ProjectHandle, store: TranscriptStore, llm: LlmDriver) -> Self {
        let conversations = store.load_conversations().unwrap_or_default();
        let current_session = conversations.first().map(|c| c.id);
        Self {
            inner: Arc::new(RwLock::new(InnerState {
                conversations,
                current_session,
            })),
            store,
            llm,
            project,
        }
    }

    pub fn project(&self) -> &ProjectHandle {
        &self.project
    }

    pub fn conversation_summaries(&self) -> Vec<ConversationSummary> {
        let inner = self.inner.read();
        inner
            .conversations
            .iter()
            .map(|c| ConversationSummary {
                id: c.id,
                title: c.title.clone(),
                updated_at: c.updated_at,
                message_count: c.messages.len(),
            })
            .collect()
    }

    pub fn active_conversation(&self) -> Option<Conversation> {
        let inner = self.inner.read();
        match inner.current_session {
            Some(id) => inner.conversations.iter().find(|c| c.id == id).cloned(),
            None => inner.conversations.first().cloned(),
        }
    }

    pub fn select_conversation(&self, id: Uuid) {
        let mut inner = self.inner.write();
        if inner.conversations.iter().any(|c| c.id == id) {
            inner.current_session = Some(id);
        }
    }

    pub fn start_new_conversation(&self) -> Uuid {
        let mut inner = self.inner.write();
        inner.conversations.insert(0, Conversation::new());
        let id = inner.conversations[0].id;
        inner.current_session = Some(id);
        if let Err(err) = self.store.persist_metadata(&inner.conversations[0]) {
            tracing::warn!(%err, "failed to persist new conversation metadata");
        }
        id
    }

    pub async fn send_user_message(
        &self,
        content: impl Into<String>,
        model: impl Into<String>,
        temperature: f32,
    ) -> Result<()> {
        let content = content.into();
        if content.trim().is_empty() {
            return Ok(());
        }
        let model = model.into();

        let message = ChatMessage::new(MessageRole::User, content.clone());
        let conversation_id = {
            let mut inner = self.inner.write();
            let conversation = Self::ensure_conversation(&mut inner);
            let title_changed = conversation.add_message(message.clone());
            self.store.append_message(conversation.id, &message)?;
            if title_changed {
                self.store.persist_metadata(conversation)?;
            }
            conversation.id
        };

        let history = self.conversation_history(conversation_id);
        let response = self
            .llm
            .respond(&history, Some(model.as_str()), Some(temperature))
            .await?;
        let assistant_message = response.message;
        {
            let mut inner = self.inner.write();
            if let Some(conversation) = inner
                .conversations
                .iter_mut()
                .find(|conversation| conversation.id == conversation_id)
            {
                let title_changed = conversation.add_message(assistant_message.clone());
                self.store
                    .append_message(conversation.id, &assistant_message)?;
                if title_changed {
                    self.store.persist_metadata(conversation)?;
                }
            }
        }
        Ok(())
    }

    pub async fn send_user_message_streaming(
        &self,
        content: impl Into<String>,
        model: impl Into<String>,
        temperature: f32,
    ) -> Result<(Uuid, mpsc::UnboundedReceiver<Result<StreamChunk>>)> {
        let content = content.into();
        if content.trim().is_empty() {
            let (tx, rx) = mpsc::unbounded_channel();
            let _ = tx.send(Ok(StreamChunk {
                delta: String::new(),
                done: true,
            }));
            return Ok((Uuid::new_v4(), rx));
        }
        let model = model.into();

        let message = ChatMessage::new(MessageRole::User, content.clone());
        let conversation_id = {
            let mut inner = self.inner.write();
            let conversation = Self::ensure_conversation(&mut inner);
            let title_changed = conversation.add_message(message.clone());
            self.store.append_message(conversation.id, &message)?;
            if title_changed {
                self.store.persist_metadata(conversation)?;
            }
            conversation.id
        };

        let history = self.conversation_history(conversation_id);
        let stream_rx = self
            .llm
            .respond_streaming(&history, Some(model.as_str()), Some(temperature))
            .await?;

        let (tx, rx) = mpsc::unbounded_channel();
        let assistant_id = Uuid::new_v4();
        let store = self.store.clone();
        let inner = self.inner.clone();

        tokio::spawn(async move {
            let mut accumulated_content = String::new();
            let mut stream = stream_rx;

            while let Some(result) = stream.recv().await {
                match result {
                    Ok(chunk) => {
                        if chunk.done {
                            // Save complete assistant message
                            let assistant_message = ChatMessage {
                                id: assistant_id,
                                role: MessageRole::Assistant,
                                content: accumulated_content.clone(),
                                created_at: Utc::now(),
                                tool_calls: Vec::new(),
                            };

                            let mut inner_guard = inner.write();
                            if let Some(conversation) = inner_guard
                                .conversations
                                .iter_mut()
                                .find(|c| c.id == conversation_id)
                            {
                                let title_changed =
                                    conversation.add_message(assistant_message.clone());
                                if let Err(err) =
                                    store.append_message(conversation.id, &assistant_message)
                                {
                                    tracing::error!(%err, "failed to persist assistant message");
                                }
                                if title_changed {
                                    if let Err(err) = store.persist_metadata(conversation) {
                                        tracing::error!(%err, "failed to persist metadata");
                                    }
                                }
                            }

                            let _ = tx.send(Ok(StreamChunk {
                                delta: String::new(),
                                done: true,
                            }));
                            break;
                        } else {
                            accumulated_content.push_str(&chunk.delta);
                            let _ = tx.send(Ok(chunk));
                        }
                    }
                    Err(e) => {
                        let _ = tx.send(Err(e));
                        break;
                    }
                }
            }
        });

        Ok((assistant_id, rx))
    }

    pub fn rename_conversation(&self, id: Uuid, title: impl Into<String>) -> Result<()> {
        let mut inner = self.inner.write();
        if let Some(conversation) = inner.conversations.iter_mut().find(|c| c.id == id) {
            conversation.title = title.into();
            self.store.persist_metadata(conversation)?;
        }
        Ok(())
    }

    pub fn delete_conversation(&self, id: Uuid) -> Result<bool> {
        let mut inner = self.inner.write();
        if let Some(position) = inner.conversations.iter().position(|c| c.id == id) {
            inner.conversations.remove(position);
            if inner.current_session == Some(id) {
                inner.current_session = inner.conversations.first().map(|c| c.id);
            }
            self.store.delete_conversation(id)?;
            return Ok(true);
        }
        Ok(false)
    }

    pub fn reorder_conversations(&self, dragged: Uuid, target: Uuid) -> Result<()> {
        let mut inner = self.inner.write();
        let from_idx = inner.conversations.iter().position(|c| c.id == dragged);
        let to_idx = inner.conversations.iter().position(|c| c.id == target);
        if let (Some(from), Some(mut to)) = (from_idx, to_idx) {
            if from == to {
                return Ok(());
            }
            let conversation = inner.conversations.remove(from);
            if from < to {
                to -= 1;
            }
            inner.conversations.insert(to, conversation);
        }
        Ok(())
    }

    fn conversation_history(&self, id: Uuid) -> Vec<ChatMessage> {
        let inner = self.inner.read();
        inner
            .conversations
            .iter()
            .find(|c| c.id == id)
            .map(|c| c.messages.clone())
            .unwrap_or_default()
    }

    fn ensure_conversation(inner: &mut InnerState) -> &mut Conversation {
        if let Some(id) = inner.current_session {
            if let Some(position) = inner.conversations.iter().position(|c| c.id == id) {
                return &mut inner.conversations[position];
            }
        }
        if inner.conversations.is_empty() {
            inner.conversations.push(Conversation::new());
        }
        let id = inner.conversations[0].id;
        inner.current_session = Some(id);
        &mut inner.conversations[0]
    }

    pub fn llm_status(&self) -> LlmStatus {
        self.llm.status()
    }
}

fn snippet(content: &str) -> String {
    let trimmed = content.trim();
    const MAX: usize = 42;
    let mut chars = trimmed.chars();
    let mut acc = String::new();
    for _ in 0..MAX {
        if let Some(ch) = chars.next() {
            acc.push(ch);
        } else {
            return trimmed.to_string();
        }
    }
    acc.push('…');
    acc
}
