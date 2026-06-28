use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result};
use chrono::{DateTime, Duration, NaiveDate, Utc};
use clap::Args;
use rusqlite::{Connection, OpenFlags};
use serde::Serialize;

use crate::config::PatinaConfig;
use crate::index::sha256_hex;
use crate::output::{JsonEnvelope, print_json};

#[derive(Debug, Args)]
pub struct StaleArgs {
    #[arg(long)]
    pub json: bool,
}

#[derive(Debug, Clone, Serialize)]
struct StaleReason {
    code: String,
    severity: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    source: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
struct StalePage {
    path: String,
    reasons: Vec<StaleReason>,
}

#[derive(Debug, Serialize)]
struct StaleData {
    stale_pages: Vec<StalePage>,
}

#[derive(Debug, Clone)]
struct Document {
    path: String,
    status: Option<String>,
    modified_at: Option<String>,
    indexed_at: Option<String>,
    review_after: Option<String>,
}

pub fn run(args: StaleArgs, _config: &PatinaConfig) -> Result<()> {
    let stale_pages = collect_stale_pages()?;
    let ok = stale_pages
        .iter()
        .flat_map(|page| &page.reasons)
        .all(|reason| reason.severity != "error");
    let data = StaleData { stale_pages };

    if args.json {
        print_json(&JsonEnvelope::new(
            "stale",
            ok,
            Some(data),
            Vec::new(),
            Vec::new(),
        ))?;
    } else {
        for page in data.stale_pages {
            let reasons = page
                .reasons
                .iter()
                .map(|reason| reason.code.as_str())
                .collect::<Vec<_>>()
                .join(", ");
            println!("{}\t{}", page.path, reasons);
        }
    }
    Ok(())
}

fn collect_stale_pages() -> Result<Vec<StalePage>> {
    let db_path = PathBuf::from(".patina/index.sqlite");
    let conn = Connection::open_with_flags(&db_path, OpenFlags::SQLITE_OPEN_READ_ONLY)
        .with_context(|| format!("failed to open Patina index {}", db_path.display()))?;
    let documents = load_documents(&conn)?;
    let mut reasons: BTreeMap<String, Vec<StaleReason>> = BTreeMap::new();

    for document in &documents {
        if review_after_passed(document.review_after.as_deref()) {
            reasons
                .entry(document.path.clone())
                .or_default()
                .push(reason("review_after_passed", "warning", None));
        }

        if document.status.as_deref() == Some("draft") && draft_too_old(document) {
            reasons
                .entry(document.path.clone())
                .or_default()
                .push(reason("draft_too_old", "warning", None));
        }
    }

    collect_source_ref_reasons(&conn, &mut reasons)?;
    collect_deprecated_link_reasons(&documents, &mut reasons);

    Ok(reasons
        .into_iter()
        .map(|(path, reasons)| StalePage { path, reasons })
        .collect())
}

fn load_documents(conn: &Connection) -> Result<Vec<Document>> {
    let mut statement = conn
        .prepare("SELECT path, status, modified_at, indexed_at, review_after FROM documents")
        .context("failed to prepare stale documents query")?;
    let rows = statement
        .query_map([], |row| {
            Ok(Document {
                path: row.get(0)?,
                status: row.get(1)?,
                modified_at: row.get(2)?,
                indexed_at: row.get(3)?,
                review_after: row.get(4)?,
            })
        })
        .context("failed to query stale documents")?;

    rows.collect::<std::result::Result<Vec<_>, _>>()
        .context("failed to collect stale documents")
}

fn collect_source_ref_reasons(
    conn: &Connection,
    reasons: &mut BTreeMap<String, Vec<StaleReason>>,
) -> Result<()> {
    let mut statement = conn
        .prepare(
            "SELECT d.path, sr.source_path, sr.source_hash_at_index, sr.referenced_from
             FROM source_refs sr
             JOIN documents d ON d.id = sr.document_id",
        )
        .context("failed to prepare source reference stale query")?;
    let rows = statement
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, Option<String>>(2)?,
                row.get::<_, Option<String>>(3)?,
            ))
        })
        .context("failed to query source references")?;

    for row in rows {
        let (document_path, source_path, indexed_hash, referenced_from) =
            row.context("failed to read source reference row")?;
        let resolved = PathBuf::from(referenced_from.as_deref().unwrap_or(&source_path));
        if !resolved.exists() {
            reasons.entry(document_path).or_default().push(reason(
                "missing_source_ref",
                "error",
                Some(source_path),
            ));
            continue;
        }

        let current_hash = sha256_hex(
            &fs::read(&resolved)
                .with_context(|| format!("failed to read source {}", resolved.display()))?,
        );
        if indexed_hash.as_deref() != Some(current_hash.as_str()) {
            reasons.entry(document_path).or_default().push(reason(
                "source_hash_changed",
                "warning",
                Some(source_path),
            ));
        }
    }

    Ok(())
}

fn collect_deprecated_link_reasons(
    documents: &[Document],
    reasons: &mut BTreeMap<String, Vec<StaleReason>>,
) {
    let deprecated = documents
        .iter()
        .filter(|document| document.status.as_deref() == Some("deprecated"))
        .collect::<Vec<_>>();
    let active = documents
        .iter()
        .filter(|document| document.status.as_deref() == Some("active"))
        .collect::<Vec<_>>();

    for deprecated_doc in deprecated {
        let target = PathBuf::from(&deprecated_doc.path);
        let stem = target
            .file_stem()
            .and_then(|stem| stem.to_str())
            .unwrap_or("");
        for active_doc in &active {
            let Ok(body) = fs::read_to_string(&active_doc.path) else {
                continue;
            };
            if body.contains(&deprecated_doc.path)
                || (!stem.is_empty() && body.contains(&format!("[[{stem}]]")))
            {
                reasons
                    .entry(deprecated_doc.path.clone())
                    .or_default()
                    .push(reason(
                        "deprecated_but_linked",
                        "warning",
                        Some(active_doc.path.clone()),
                    ));
                break;
            }
        }
    }
}

fn review_after_passed(value: Option<&str>) -> bool {
    let Some(value) = value else {
        return false;
    };
    let Ok(date) = NaiveDate::parse_from_str(value, "%Y-%m-%d") else {
        return false;
    };
    date < Utc::now().date_naive()
}

fn draft_too_old(document: &Document) -> bool {
    let timestamp = document
        .modified_at
        .as_deref()
        .or(document.indexed_at.as_deref());
    let Some(timestamp) = timestamp else {
        return false;
    };
    let Ok(datetime) = DateTime::parse_from_rfc3339(timestamp) else {
        return false;
    };
    Utc::now().signed_duration_since(datetime.with_timezone(&Utc)) > Duration::days(90)
}

fn reason(code: &str, severity: &str, source: Option<String>) -> StaleReason {
    StaleReason {
        code: code.to_string(),
        severity: severity.to_string(),
        source,
    }
}
