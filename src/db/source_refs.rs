use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use rusqlite::{Connection, params};
use serde_yaml::Value;

use crate::index::sha256_hex;

pub fn replace_for_document(
    conn: &Connection,
    document_id: i64,
    knowledge_root: &Path,
    front_matter: &std::collections::BTreeMap<String, Value>,
) -> Result<()> {
    conn.execute(
        "DELETE FROM source_refs WHERE document_id = ?1",
        params![document_id],
    )
    .context("failed to delete old source references")?;

    for source_ref in source_refs(front_matter) {
        let resolved = resolve_source_ref(knowledge_root, &source_ref);
        let (hash, modified_at) = source_metadata(&resolved)?;
        conn.execute(
            "INSERT INTO source_refs (
                document_id, source_path, source_hash_at_index,
                source_modified_at_index, referenced_from
            ) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                document_id,
                source_ref,
                hash,
                modified_at,
                resolved.display().to_string()
            ],
        )
        .with_context(|| format!("failed to insert source reference {}", resolved.display()))?;
    }

    Ok(())
}

fn source_refs(front_matter: &std::collections::BTreeMap<String, Value>) -> Vec<String> {
    match front_matter.get("source_refs") {
        Some(Value::String(value)) => vec![value.clone()],
        Some(Value::Sequence(values)) => values
            .iter()
            .filter_map(Value::as_str)
            .map(ToOwned::to_owned)
            .collect(),
        _ => Vec::new(),
    }
}

fn source_metadata(path: &Path) -> Result<(Option<String>, Option<String>)> {
    if !path.exists() {
        return Ok((None, None));
    }

    let bytes = fs::read(path).with_context(|| format!("failed to read {}", path.display()))?;
    let modified_at = fs::metadata(path)
        .ok()
        .and_then(|metadata| metadata.modified().ok())
        .map(DateTime::<Utc>::from)
        .map(|datetime| datetime.to_rfc3339());

    Ok((Some(sha256_hex(&bytes)), modified_at))
}

fn resolve_source_ref(knowledge_root: &Path, source_ref: &str) -> PathBuf {
    let path = Path::new(source_ref);
    if path.is_absolute() {
        return path.to_path_buf();
    }

    if let Some(root_name) = knowledge_root.file_name() {
        if path.starts_with(root_name) {
            if let Some(parent) = knowledge_root.parent() {
                return parent.join(path);
            }
        }
    }

    knowledge_root.join(path)
}
