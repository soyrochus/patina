use crate::store::TranscriptStore;
use anyhow::{anyhow, Result};
use chrono::{Duration, Utc};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

/// Authentication modes advertised by an MCP endpoint.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum AuthMode {
    ServerManaged,
    ClientManaged,
}

/// Stored authentication state for a given MCP endpoint or provider.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthState {
    pub mode: AuthMode,
    pub access_token: Option<String>,
    pub refresh_token: Option<String>,
    pub expires_at: Option<chrono::DateTime<Utc>>,
}

impl AuthState {
    pub fn new(mode: AuthMode) -> Self {
        Self {
            mode,
            access_token: None,
            refresh_token: None,
            expires_at: None,
        }
    }

    pub fn needs_refresh(&self) -> bool {
        match (self.access_token.as_ref(), self.expires_at) {
            (Some(_), Some(exp)) => Utc::now() + Duration::minutes(1) >= exp,
            (Some(_), None) => false,
            _ => true,
        }
    }

    pub fn hydrate_for_testing(mode: AuthMode) -> Self {
        let mut state = Self::new(mode.clone());
        state.refresh(mode).expect("refresh for testing");
        state
    }

    pub fn refresh(&mut self, mode: AuthMode) -> Result<()> {
        match mode {
            AuthMode::ServerManaged => {
                self.access_token = Some(format!("server-token-{}", Uuid::new_v4()));
                self.refresh_token = None;
                self.expires_at = Some(Utc::now() + Duration::hours(1));
            }
            AuthMode::ClientManaged => {
                self.access_token = Some(format!("client-token-{}", Uuid::new_v4()));
                self.refresh_token = Some(format!("refresh-{}", Uuid::new_v4()));
                self.expires_at = Some(Utc::now() + Duration::minutes(30));
            }
        }
        Ok(())
    }
}

/// Coordinates authentication state across MCP endpoints.
#[derive(Clone)]
pub struct AuthCoordinator {
    store: Arc<RwLock<HashMap<String, AuthState>>>,
    transcript_store: TranscriptStore,
}

impl AuthCoordinator {
    pub fn new(transcript_store: TranscriptStore) -> Self {
        Self {
            store: Arc::new(RwLock::new(HashMap::new())),
            transcript_store,
        }
    }

    pub fn state_for(&self, key: &str) -> Option<AuthState> {
        self.store.read().get(key).cloned()
    }

    pub fn upsert(&self, key: impl Into<String>, state: AuthState) {
        self.store.write().insert(key.into(), state);
    }

    /// Negotiate auth with an MCP endpoint based on the advertised mode.
    pub async fn negotiate(&self, key: &str, mode: AuthMode) -> Result<AuthState> {
        let mut state = self
            .state_for(key)
            .unwrap_or_else(|| AuthState::new(mode.clone()));
        if state.needs_refresh() {
            state.refresh(mode.clone())?;
            // Persisting auth tokens to the same directory used for transcripts keeps the
            // implementation simple while allowing tests to exercise credential reuse.
            self.transcript_store
                .persist_secret(key, &state.access_token.clone().unwrap_or_default())?;
        }
        self.upsert(key.to_owned(), state.clone());
        Ok(state)
    }

    pub fn require(&self, key: &str) -> Result<AuthState> {
        self.state_for(key)
            .ok_or_else(|| anyhow!("no auth state registered for {key}"))
    }
}
