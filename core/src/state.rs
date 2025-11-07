use crate::llm::LlmDriver;
use crate::store::TranscriptStore;
use anyhow::Result;
use chrono::{DateTime, Utc};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::Arc;
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

    pub fn add_message(&mut self, mut message: ChatMessage) {
        if self.messages.is_empty() && message.role == MessageRole::User {
            self.title = snippet(&message.content);
        }
        if message.tool_calls.is_empty() {
            message.tool_calls = Vec::new();
        }
        self.messages.push(message);
        self.updated_at = Utc::now();
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
}

#[derive(Default)]
struct InnerState {
    conversations: Vec<Conversation>,
    current_session: Option<Uuid>,
}

impl AppState {
    pub fn new(store: TranscriptStore, llm: LlmDriver) -> Self {
        let conversations = store.load_conversations().unwrap_or_default();
        let current_session = conversations.first().map(|c| c.id);
        Self {
            inner: Arc::new(RwLock::new(InnerState {
                conversations,
                current_session,
            })),
            store,
            llm,
        }
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
        id
    }

    pub async fn send_user_message(&self, content: impl Into<String>) -> Result<()> {
        let content = content.into();
        if content.trim().is_empty() {
            return Ok(());
        }

        let message = ChatMessage::new(MessageRole::User, content.clone());
        let conversation_id = {
            let mut inner = self.inner.write();
            let conversation = Self::ensure_conversation(&mut inner);
            conversation.add_message(message.clone());
            self.store.append_message(conversation.id, &message)?;
            conversation.id
        };

        let history = self.conversation_history(conversation_id);
        let response = self.llm.respond(&history).await?;
        let assistant_message = response.message;
        {
            let mut inner = self.inner.write();
            if let Some(conversation) = inner
                .conversations
                .iter_mut()
                .find(|conversation| conversation.id == conversation_id)
            {
                conversation.add_message(assistant_message.clone());
                self.store
                    .append_message(conversation.id, &assistant_message)?;
            }
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

    fn ensure_conversation<'a>(inner: &'a mut InnerState) -> &'a mut Conversation {
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
