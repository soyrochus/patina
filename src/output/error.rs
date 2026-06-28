use std::path::Path;

use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct ErrorEntry {
    pub code: String,
    pub message: String,
    pub severity: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
}

impl ErrorEntry {
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            severity: "error".to_string(),
            path: None,
        }
    }

    pub fn with_path(mut self, path: &Path) -> Self {
        self.path = Some(path.display().to_string());
        self
    }

    pub fn from_anyhow(error: &anyhow::Error) -> Self {
        Self::new("command_failed", error.to_string())
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct WarningEntry {
    pub code: String,
    pub message: String,
    pub severity: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
}

impl WarningEntry {
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            severity: "warning".to_string(),
            path: None,
        }
    }

    pub fn with_path(mut self, path: &Path) -> Self {
        self.path = Some(path.display().to_string());
        self
    }
}
