use anyhow::{Context, Result};
use rusqlite::{Connection, params};

#[derive(Debug, Clone)]
pub struct RawResult {
    pub path: String,
    pub title: Option<String>,
    pub page_type: Option<String>,
    pub scope_classification: Option<String>,
    pub excerpt: String,
    pub raw_score: f64,
}

pub fn search(conn: &Connection, query: &str, limit: usize) -> Result<Vec<RawResult>> {
    let mut statement = conn
        .prepare(
            "SELECT
                d.path,
                d.title,
                d.type,
                d.scope_classification,
                c.text,
                bm25(chunks_fts) AS score
             FROM chunks_fts
             JOIN chunks c ON c.id = chunks_fts.rowid
             JOIN documents d ON d.id = c.document_id
             WHERE chunks_fts MATCH ?1
             ORDER BY score
             LIMIT ?2",
        )
        .context("failed to prepare FTS5 query")?;

    let rows = statement
        .query_map(params![query, limit as i64], |row| {
            Ok(RawResult {
                path: row.get(0)?,
                title: row.get(1)?,
                page_type: row.get(2)?,
                scope_classification: row.get(3)?,
                excerpt: excerpt(row.get::<_, String>(4)?.as_str(), query),
                raw_score: row.get(5)?,
            })
        })
        .context("failed to execute FTS5 query")?;

    rows.collect::<std::result::Result<Vec<_>, _>>()
        .context("failed to collect FTS5 query results")
}

fn excerpt(text: &str, query: &str) -> String {
    let lower = text.to_lowercase();
    let query_lower = query.to_lowercase();
    let start = lower
        .find(&query_lower)
        .map(|index| index.saturating_sub(80))
        .unwrap_or(0);
    text.chars().skip(start).take(240).collect()
}
