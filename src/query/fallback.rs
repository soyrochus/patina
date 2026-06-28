use anyhow::{Context, Result};
use rusqlite::{Connection, params};

use crate::query::fts::RawResult;

pub fn search(conn: &Connection, query: &str, limit: usize) -> Result<Vec<RawResult>> {
    let pattern = format!("%{query}%");
    let mut statement = conn
        .prepare(
            "SELECT
                d.path,
                d.title,
                d.type,
                d.scope_classification,
                d.aliases,
                d.tags,
                d.modified_at,
                c.text
             FROM chunks c
             JOIN documents d ON d.id = c.document_id
             WHERE lower(c.text) LIKE lower(?1)
             LIMIT ?2",
        )
        .context("failed to prepare fallback query")?;

    let rows = statement
        .query_map(params![pattern, limit as i64], |row| {
            Ok(RawResult {
                path: row.get(0)?,
                title: row.get(1)?,
                page_type: row.get(2)?,
                scope_classification: row.get(3)?,
                aliases: row.get(4)?,
                tags: row.get(5)?,
                modified_at: row.get(6)?,
                excerpt: row.get::<_, String>(7)?.chars().take(240).collect(),
                raw_score: 1.0,
            })
        })
        .context("failed to execute fallback query")?;

    rows.collect::<std::result::Result<Vec<_>, _>>()
        .context("failed to collect fallback query results")
}
