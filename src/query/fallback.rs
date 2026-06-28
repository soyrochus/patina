use anyhow::{Context, Result};
use rusqlite::{Connection, ToSql};

use crate::query::fts::RawResult;
use crate::query::plan::QueryPlan;

pub fn search(conn: &Connection, plan: &QueryPlan, limit: usize) -> Result<Vec<RawResult>> {
    let strict = search_with_operator(conn, plan, "AND", limit)?;
    if !strict.is_empty() {
        return Ok(strict);
    }

    search_with_operator(conn, plan, "OR", limit)
}

fn search_with_operator(
    conn: &Connection,
    plan: &QueryPlan,
    operator: &str,
    limit: usize,
) -> Result<Vec<RawResult>> {
    let patterns = plan.like_patterns();
    let predicates = (0..patterns.len())
        .map(|index| format!("lower(c.text) LIKE lower(?{})", index + 1))
        .collect::<Vec<_>>()
        .join(&format!(" {operator} "));
    let limit_index = patterns.len() + 1;
    let sql = format!(
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
         WHERE {predicates}
         LIMIT ?{limit_index}"
    );

    let mut statement = conn
        .prepare(&sql)
        .context("failed to prepare fallback query")?;
    let mut values = patterns
        .iter()
        .map(|pattern| pattern as &dyn ToSql)
        .collect::<Vec<_>>();
    let limit_value = limit as i64;
    values.push(&limit_value);

    let rows = statement
        .query_map(values.as_slice(), |row| {
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
                raw_score: plan.matched_term_count(&text) as f64,
            })
        })
        .context("failed to execute fallback query")?;

    rows.collect::<std::result::Result<Vec<_>, _>>()
        .context("failed to collect fallback query results")
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    fn conn_with_rows() -> Connection {
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
            INSERT INTO documents (id, path, title) VALUES
                (1, 'agent-boundaries.md', 'Agent Boundaries'),
                (2, 'other.md', 'Other');
            INSERT INTO chunks (id, document_id, text) VALUES
                (1, 1, 'Agents use bounded tools and cite durable project knowledge.'),
                (2, 2, 'Agents can run tools.');
            "#,
        )
        .expect("schema should initialize");
        conn
    }

    #[test]
    fn matches_normalized_natural_language_terms_instead_of_raw_sentence() {
        let conn = conn_with_rows();
        let plan = QueryPlan::new("why do agents need durable context");

        let results = search(&conn, &plan, 10).expect("fallback search should work");

        assert_eq!(results.len(), 2);
        assert_eq!(results[0].path, "agent-boundaries.md");
        assert_eq!(results[0].raw_score, 2.0);
        assert_eq!(results[1].raw_score, 1.0);
    }
}
