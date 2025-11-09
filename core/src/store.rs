use crate::state::{ChatMessage, Conversation};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fs::{self, File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use uuid::Uuid;

#[derive(Clone)]
pub struct TranscriptStore {
    root: PathBuf,
}

#[derive(Serialize, Deserialize)]
struct ConversationMetadata {
    title: String,
}

impl TranscriptStore {
    pub fn new(root: PathBuf) -> Self {
        fs::create_dir_all(root.join("conversations")).ok();
        fs::create_dir_all(root.join("secrets")).ok();
        Self { root }
    }

    pub fn in_memory() -> Self {
        let mut path = std::env::temp_dir();
        path.push(format!("patina-{}", Uuid::new_v4()));
        Self::new(path)
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    fn conversation_dir(&self) -> PathBuf {
        self.root.join("conversations")
    }

    fn metadata_path(&self, id: Uuid) -> PathBuf {
        self.conversation_dir().join(format!("{}.meta.json", id))
    }

    fn read_metadata(&self, id: Uuid) -> Option<ConversationMetadata> {
        let path = self.metadata_path(id);
        let contents = fs::read_to_string(path).ok()?;
        serde_json::from_str(&contents).ok()
    }

    pub fn load_conversations(&self) -> Result<Vec<Conversation>> {
        let mut conversations = Vec::new();
        let path = self.conversation_dir();
        if !path.exists() {
            return Ok(conversations);
        }
        for entry in walkdir::WalkDir::new(path)
            .into_iter()
            .filter_map(|entry| entry.ok())
        {
            if !entry.file_type().is_file() {
                continue;
            }
            let file_path = entry.into_path();
            if file_path.extension().and_then(|ext| ext.to_str()) != Some("jsonl") {
                continue;
            }
            let id = file_path
                .file_stem()
                .and_then(|stem| stem.to_str())
                .and_then(|s| Uuid::parse_str(s).ok())
                .unwrap_or_else(Uuid::new_v4);
            let file = File::open(&file_path)?;
            let reader = BufReader::new(file);
            let mut conversation = Conversation::with_id(id, "Restored conversation");
            for line in reader.lines() {
                let line = line?;
                if line.trim().is_empty() {
                    continue;
                }
                let message: ChatMessage = serde_json::from_str(&line)?;
                let _ = conversation.add_message(message);
            }
            if let Some(meta) = self.read_metadata(id) {
                conversation.title = meta.title;
            }
            conversations.push(conversation);
        }
        conversations.sort_by_key(|c| c.updated_at);
        conversations.reverse();
        Ok(conversations)
    }

    pub fn append_message(&self, conversation_id: Uuid, message: &ChatMessage) -> Result<()> {
        let path = self
            .conversation_dir()
            .join(format!("{}.jsonl", conversation_id));
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).ok();
        }
        let mut file = OpenOptions::new().create(true).append(true).open(&path)?;
        let serialized = serde_json::to_vec(message)?;
        file.write_all(&serialized)?;
        file.write_all(b"\n")?;
        Ok(())
    }

    pub fn persist_metadata(&self, conversation: &Conversation) -> Result<()> {
        let meta = ConversationMetadata {
            title: conversation.title.clone(),
        };
        let path = self.metadata_path(conversation.id);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).ok();
        }
        let serialized = serde_json::to_vec_pretty(&meta)?;
        fs::write(path, serialized)?;
        Ok(())
    }

    pub fn delete_conversation(&self, id: Uuid) -> Result<()> {
        let transcript_path = self.conversation_dir().join(format!("{}.jsonl", id));
        let _ = fs::remove_file(transcript_path);
        let _ = fs::remove_file(self.metadata_path(id));
        Ok(())
    }

    pub fn persist_secret(&self, key: &str, secret: &str) -> Result<()> {
        let path = self.root.join("secrets").join(format!("{}.txt", key));
        let mut file = File::create(path)?;
        file.write_all(secret.as_bytes())?;
        Ok(())
    }
}
