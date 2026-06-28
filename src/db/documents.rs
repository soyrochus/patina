use anyhow::{Context, Result};
use rusqlite::{Connection, OptionalExtension, params};

#[derive(Debug, Clone)]
pub struct DocumentRecord {
    pub path: String,
    pub title: Option<String>,
    pub page_type: Option<String>,
    pub status: Option<String>,
    pub sha256: String,
    pub modified_at: Option<String>,
    pub indexed_at: String,
    pub front_matter_updated: Option<String>,
    pub review_after: Option<String>,
    pub scope_classification: Option<String>,
    pub aliases: Option<String>,
    pub tags: Option<String>,
}

pub fn stored_sha256(conn: &Connection, path: &str) -> Result<Option<String>> {
    conn.query_row(
        "SELECT sha256 FROM documents WHERE path = ?1",
        params![path],
        |row| row.get(0),
    )
    .optional()
    .with_context(|| format!("failed to read stored hash for {path}"))
}

pub fn upsert(conn: &Connection, record: &DocumentRecord) -> Result<i64> {
    conn.execute(
        "INSERT INTO documents (
            path, title, type, status, sha256, modified_at, indexed_at,
            front_matter_updated, review_after, scope_classification, aliases, tags
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)
        ON CONFLICT(path) DO UPDATE SET
            title = excluded.title,
            type = excluded.type,
            status = excluded.status,
            sha256 = excluded.sha256,
            modified_at = excluded.modified_at,
            indexed_at = excluded.indexed_at,
            front_matter_updated = excluded.front_matter_updated,
            review_after = excluded.review_after,
            scope_classification = excluded.scope_classification,
            aliases = excluded.aliases,
            tags = excluded.tags",
        params![
            record.path,
            record.title,
            record.page_type,
            record.status,
            record.sha256,
            record.modified_at,
            record.indexed_at,
            record.front_matter_updated,
            record.review_after,
            record.scope_classification,
            record.aliases,
            record.tags,
        ],
    )
    .with_context(|| format!("failed to upsert document {}", record.path))?;

    conn.query_row(
        "SELECT id FROM documents WHERE path = ?1",
        params![record.path],
        |row| row.get(0),
    )
    .with_context(|| format!("failed to read document id for {}", record.path))
}
