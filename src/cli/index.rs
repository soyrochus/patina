use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use clap::Args;
use rusqlite::Connection;
use serde::Serialize;
use serde_yaml::Value;

use crate::config::PatinaConfig;
use crate::db::{chunks, documents, init as db_init, lock::IndexLock, source_refs};
use crate::discovery::{frontmatter, scope, walker};
use crate::index::{chunker, sha256_hex};
use crate::output::{ErrorEntry, JsonEnvelope, print_json};

#[derive(Debug, Args)]
pub struct IndexArgs {
    #[arg(long)]
    pub json: bool,
    #[arg(long)]
    pub full: bool,
    #[arg(long)]
    pub reset: bool,
}

#[derive(Debug, Serialize)]
struct IndexData {
    files_processed: usize,
    chunk_count: usize,
    skipped_count: usize,
}

struct IndexStats {
    files_processed: usize,
    chunk_count: usize,
    skipped_count: usize,
    errors: Vec<ErrorEntry>,
}

pub fn run(args: IndexArgs, config: &PatinaConfig) -> Result<()> {
    let stats = run_index(args.full || args.reset, args.reset, config)?;
    let ok = stats.errors.is_empty();
    let data = IndexData {
        files_processed: stats.files_processed,
        chunk_count: stats.chunk_count,
        skipped_count: stats.skipped_count,
    };

    if args.json {
        print_json(&JsonEnvelope::new(
            "index",
            ok,
            Some(data),
            Vec::new(),
            stats.errors,
        ))?;
    } else {
        println!(
            "indexed {} file(s), {} chunk(s), skipped {} unchanged file(s)",
            data.files_processed, data.chunk_count, data.skipped_count
        );
    }
    Ok(())
}

fn run_index(full: bool, reset: bool, config: &PatinaConfig) -> Result<IndexStats> {
    let patina_dir = PathBuf::from(".patina");
    let live_db = patina_dir.join("index.sqlite");
    let _lock = IndexLock::acquire(&patina_dir)?;

    if full || reset {
        let tmp_db = patina_dir.join("index.sqlite.tmp");
        if tmp_db.exists() {
            fs::remove_file(&tmp_db)
                .with_context(|| format!("failed to remove {}", tmp_db.display()))?;
        }

        let db = db_init::open(&tmp_db, env!("CARGO_PKG_VERSION"))?;
        let stats = build_full_index(&db.conn, db.fts5_available, config, false)?;
        validate_integrity(&db.conn)?;
        drop(db);
        replace_database(&tmp_db, &live_db)?;
        return Ok(stats);
    }

    let db = db_init::open(&live_db, env!("CARGO_PKG_VERSION"))?;
    build_full_index(&db.conn, db.fts5_available, config, true)
}

fn build_full_index(
    conn: &Connection,
    fts5_available: bool,
    config: &PatinaConfig,
    incremental: bool,
) -> Result<IndexStats> {
    let knowledge_root = config.knowledge_dir();
    let scope = scope::load(&knowledge_root)?;
    let scope_classification = scope.classification();
    let walk = walker::walk_markdown(&knowledge_root, config)?;
    let mut stats = IndexStats {
        files_processed: 0,
        chunk_count: 0,
        skipped_count: 0,
        errors: Vec::new(),
    };

    for file in walk.files {
        let contents = fs::read_to_string(&file.path)
            .with_context(|| format!("failed to read {}", file.path.display()))?;
        let document_hash = sha256_hex(contents.as_bytes());
        let path_string = file.path.display().to_string();

        if incremental
            && documents::stored_sha256(conn, &path_string)?
                .as_deref()
                .is_some_and(|stored| stored == document_hash)
        {
            stats.skipped_count += 1;
            continue;
        }

        let parsed = match frontmatter::parse_file(&file.path, &contents) {
            Ok(parsed) => parsed,
            Err(error) => {
                stats.errors.push(error.to_error_entry());
                continue;
            }
        };

        let tx = conn
            .unchecked_transaction()
            .context("failed to begin index transaction")?;
        let chunks_for_file = chunker::chunk_markdown(&parsed.body, &config.index);
        let record = documents::DocumentRecord {
            path: path_string,
            title: string_field(&parsed.front_matter, "title"),
            page_type: string_field(&parsed.front_matter, "type"),
            status: string_field(&parsed.front_matter, "status"),
            sha256: document_hash,
            modified_at: modified_at(&file.path),
            indexed_at: Utc::now().to_rfc3339(),
            front_matter_updated: string_field(&parsed.front_matter, "updated"),
            review_after: string_field(&parsed.front_matter, "review_after"),
            scope_classification: scope_classification.clone(),
            aliases: yaml_sequence_as_json(&parsed.front_matter, "aliases"),
            tags: yaml_sequence_as_json(&parsed.front_matter, "tags"),
        };
        let document_id = documents::upsert(&tx, &record)?;
        chunks::replace_for_document(&tx, document_id, &chunks_for_file, fts5_available)?;
        source_refs::replace_for_document(&tx, document_id, &knowledge_root, &parsed.front_matter)?;
        tx.commit().context("failed to commit index transaction")?;

        stats.files_processed += 1;
        stats.chunk_count += chunks_for_file.len();
    }

    Ok(stats)
}

fn string_field(
    front_matter: &std::collections::BTreeMap<String, Value>,
    field: &str,
) -> Option<String> {
    front_matter
        .get(field)
        .and_then(Value::as_str)
        .map(ToOwned::to_owned)
}

fn yaml_sequence_as_json(
    front_matter: &std::collections::BTreeMap<String, Value>,
    field: &str,
) -> Option<String> {
    let strings = front_matter
        .get(field)
        .and_then(Value::as_sequence)?
        .iter()
        .filter_map(Value::as_str)
        .collect::<Vec<_>>();

    if strings.is_empty() {
        None
    } else {
        serde_json::to_string(&strings).ok()
    }
}

fn modified_at(path: &Path) -> Option<String> {
    fs::metadata(path)
        .ok()
        .and_then(|metadata| metadata.modified().ok())
        .map(DateTime::<Utc>::from)
        .map(|datetime| datetime.to_rfc3339())
}

fn validate_integrity(conn: &Connection) -> Result<()> {
    let result: String = conn
        .query_row("PRAGMA integrity_check", [], |row| row.get(0))
        .context("failed to run SQLite integrity_check")?;
    anyhow::ensure!(result == "ok", "SQLite integrity_check failed: {result}");
    Ok(())
}

fn replace_database(tmp_db: &Path, live_db: &Path) -> Result<()> {
    match fs::rename(tmp_db, live_db) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
            fs::remove_file(live_db)
                .with_context(|| format!("failed to remove {}", live_db.display()))?;
            fs::rename(tmp_db, live_db).with_context(|| {
                format!(
                    "failed to replace {} with {}",
                    live_db.display(),
                    tmp_db.display()
                )
            })
        }
        Err(error) => Err(error).with_context(|| {
            format!(
                "failed to replace {} with {}",
                live_db.display(),
                tmp_db.display()
            )
        }),
    }
}
