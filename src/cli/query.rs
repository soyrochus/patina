use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::Args;
use rusqlite::Connection;
use serde::Serialize;

use crate::db::init as db_init;
use crate::output::{JsonEnvelope, WarningEntry, print_json};
use crate::query::{fallback, fts, scorer};

#[derive(Debug, Args)]
pub struct QueryArgs {
    pub terms: String,
    #[arg(long)]
    pub json: bool,
    #[arg(long, default_value_t = 10)]
    pub limit: usize,
    #[arg(long)]
    pub explain: bool,
}

#[derive(Debug, Serialize)]
struct QueryData {
    mode: String,
    results: Vec<QueryResult>,
}

#[derive(Debug, Serialize)]
struct QueryResult {
    path: String,
    score: f64,
    excerpt: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    score_components: Option<scorer::ScoreComponents>,
    #[serde(skip_serializing_if = "Option::is_none")]
    matches: Option<Vec<String>>,
}

pub fn run(args: QueryArgs) -> Result<()> {
    let (data, warnings) = run_query(&args)?;
    if args.json {
        print_json(&JsonEnvelope::success("query", data, warnings))?;
    } else {
        for result in data.results {
            println!("{:.3}\t{}\t{}", result.score, result.path, result.excerpt);
        }
    }
    Ok(())
}

fn run_query(args: &QueryArgs) -> Result<(QueryData, Vec<WarningEntry>)> {
    let db_path = PathBuf::from(".patina/index.sqlite");
    let conn = Connection::open(&db_path)
        .with_context(|| format!("failed to open Patina index {}", db_path.display()))?;
    db_init::validate_schema_version(&conn)?;
    let mut warnings = Vec::new();

    let (mode, raw_results) = match fts::search(&conn, &args.terms, args.limit) {
        Ok(results) => ("fts5".to_string(), results),
        Err(error) => {
            warnings.push(WarningEntry::new(
                "fts5_unavailable",
                format!("SQLite FTS5 is unavailable; using degraded LIKE-based search: {error}"),
            ));
            (
                "lexical-fallback".to_string(),
                fallback::search(&conn, &args.terms, args.limit)?,
            )
        }
    };

    let normalized = if mode == "fts5" {
        scorer::normalize_bm25(
            &raw_results
                .iter()
                .map(|result| result.raw_score)
                .collect::<Vec<_>>(),
        )
    } else {
        vec![1.0; raw_results.len()]
    };

    let mut results = raw_results
        .into_iter()
        .zip(normalized)
        .map(|(raw, normalized_fts)| {
            let components = scorer::score_components(
                &args.terms,
                normalized_fts,
                raw.title.as_deref(),
                raw.page_type.as_deref(),
                raw.scope_classification.as_deref(),
            );
            QueryResult {
                path: raw.path,
                score: components.combined(),
                excerpt: raw.excerpt,
                score_components: args.explain.then_some(components),
                matches: args.explain.then(|| vec![args.terms.clone()]),
            }
        })
        .collect::<Vec<_>>();

    results.sort_by(|left, right| {
        right
            .score
            .partial_cmp(&left.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    results.truncate(args.limit);

    Ok((QueryData { mode, results }, warnings))
}
