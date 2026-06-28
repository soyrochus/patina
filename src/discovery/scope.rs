use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde_yaml::Value;

use crate::output::WarningEntry;

#[derive(Debug, Clone)]
pub struct ScopeLoad {
    pub path: PathBuf,
    pub metadata: Option<Value>,
    pub warnings: Vec<WarningEntry>,
}

pub fn load(knowledge_root: &Path) -> Result<ScopeLoad> {
    let path = knowledge_root.join("scope.yaml");
    if !path.exists() {
        return Ok(ScopeLoad {
            path,
            metadata: None,
            warnings: Vec::new(),
        });
    }

    let contents =
        fs::read_to_string(&path).with_context(|| format!("failed to read {}", path.display()))?;
    match serde_yaml::from_str::<Value>(&contents) {
        Ok(value) => Ok(ScopeLoad {
            path,
            metadata: Some(value),
            warnings: Vec::new(),
        }),
        Err(error) => Ok(ScopeLoad {
            path: path.clone(),
            metadata: None,
            warnings: vec![
                WarningEntry::new(
                    "malformed_scope_yaml",
                    format!("malformed scope.yaml at {}: {error}", path.display()),
                )
                .with_path(&path),
            ],
        }),
    }
}

impl ScopeLoad {
    pub fn classification(&self) -> Option<String> {
        self.metadata
            .as_ref()
            .and_then(|metadata| metadata.get("scope"))
            .and_then(Value::as_str)
            .map(ToOwned::to_owned)
    }
}
