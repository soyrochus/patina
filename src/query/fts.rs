use anyhow::{Context, Result};
use rusqlite::{Connection, params};

use crate::query::plan::QueryPlan;

#[derive(Debug, Clone)]
pub struct RawResult {
    pub path: String,
    pub title: Option<String>,
    pub page_type: Option<String>,
    pub scope_classification: Option<String>,
    pub aliases: Option<String>,
    pub tags: Option<String>,
    pub modified_at: Option<String>,
    pub excerpt: String,
    pub raw_score: f64,
}

pub fn search(conn: &Connection, plan: &QueryPlan, limit: usize) -> Result<Vec<RawResult>> {
    let strict = search_expression(conn, plan, &plan.fts_all_expression(), limit)?;
    if !strict.is_empty() {
        return Ok(strict);
    }

    search_expression(conn, plan, &plan.fts_any_expression(), limit)
}

fn search_expression(
    conn: &Connection,
    plan: &QueryPlan,
    expression: &str,
    limit: usize,
) -> Result<Vec<RawResult>> {
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
        .query_map(params![expression, limit as i64], |row| {
            let text = row.get::<_, String>(7)?;
            Ok(RawResult {
                path: row.get(0)?,
                title: row.get(1)?,
                page_type: row.get(2)?,
                scope_classification: row.get(3)?,
                aliases: row.get(4)?,
                tags: row.get(5)?,
                modified_at: row.get(6)?,
                excerpt: plan.excerpt(&text),
                raw_score: row.get(8)?,
            })
        })
        .context("failed to execute FTS5 query")?;

    rows.collect::<std::result::Result<Vec<_>, _>>()
        .context("failed to collect FTS5 query results")
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    fn conn_with_fts_rows() -> Connection {
        let conn = Connection::open_in_memory().expect("in-memory db should open");
        conn.execute_batch(
            r#"
            CREATE TABLE documents (
                id INTEGER PRIMARY KEY,
                path TEXT NOT NULL,
                title TEXT,
                type TEXT,
                scope_classification TEXT,
                aliases TEXT,
                tags TEXT,
                modified_at TEXT
            );
            CREATE TABLE chunks (
                id INTEGER PRIMARY KEY,
                document_id INTEGER NOT NULL,
                text TEXT NOT NULL
            );
            CREATE VIRTUAL TABLE chunks_fts
            USING fts5(text, content='chunks', content_rowid='id');
            INSERT INTO documents (id, path, title) VALUES
                (1, 'one.md', 'One'),
                (2, 'two.md', 'Two');
            INSERT INTO chunks (id, document_id, text) VALUES
                (1, 1, 'Agents cite durable project knowledge.'),
                (2, 2, 'Unrelated text.');
            INSERT INTO chunks_fts(rowid, text) VALUES
                (1, 'Agents cite durable project knowledge.'),
                (2, 'Unrelated text.');
            "#,
        )
        .expect("schema should initialize");
        conn
    }

    #[test]
    fn retries_with_any_term_when_strict_fts_has_no_results() {
        let conn = conn_with_fts_rows();
        let plan = QueryPlan::new("agents durable context");

        let results = search(&conn, &plan, 10).expect("search should work");

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].path, "one.md");
    }
}
