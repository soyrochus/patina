use std::path::Path;
use std::time::Duration;

use anyhow::{Context, Result, bail};
use chrono::Utc;
use rusqlite::{Connection, OptionalExtension, params};

use crate::db::schema;

pub struct PatinaDb {
    pub conn: Connection,
    pub fts5_available: bool,
}

pub fn open(path: &Path, patina_version: &str) -> Result<PatinaDb> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }

    let conn = Connection::open(path)
        .with_context(|| format!("failed to open SQLite database {}", path.display()))?;
    configure_connection(&conn)?;
    let fts5_available = check_fts5_available(&conn);
    initialize_schema(&conn, patina_version, fts5_available)?;
    validate_schema_version(&conn)?;

    Ok(PatinaDb {
        conn,
        fts5_available,
    })
}

pub fn configure_connection(conn: &Connection) -> Result<()> {
    conn.busy_timeout(Duration::from_secs(5))
        .context("failed to set SQLite busy timeout")?;
    conn.query_row("PRAGMA journal_mode = WAL", [], |row| {
        row.get::<_, String>(0)
    })
    .map(|_| ())
    .context("failed to enable SQLite WAL mode")?;
    Ok(())
}

pub fn initialize_schema(
    conn: &Connection,
    patina_version: &str,
    fts5_available: bool,
) -> Result<()> {
    conn.execute_batch(schema::CREATE_META_TABLE)
        .context("failed to create meta table")?;
    conn.execute_batch(schema::CREATE_DOCUMENTS_TABLE)
        .context("failed to create documents table")?;
    ensure_documents_scope_classification_column(conn)?;
    conn.execute_batch(schema::CREATE_CHUNKS_TABLE)
        .context("failed to create chunks table")?;
    conn.execute_batch(schema::CREATE_SOURCE_REFS_TABLE)
        .context("failed to create source_refs table")?;

    if fts5_available {
        conn.execute_batch(schema::CREATE_CHUNKS_FTS_TABLE)
            .context("failed to create chunks FTS5 table")?;
    }

    let now = Utc::now().to_rfc3339();
    insert_meta_if_absent(conn, "schema_version", schema::SCHEMA_VERSION)?;
    insert_meta_if_absent(conn, "patina_version", patina_version)?;
    insert_meta_if_absent(conn, "created_at", &now)?;
    upsert_meta(conn, "updated_at", &now)?;

    Ok(())
}

pub fn validate_schema_version(conn: &Connection) -> Result<()> {
    let version: Option<String> = conn
        .query_row(
            "SELECT value FROM meta WHERE key = 'schema_version'",
            [],
            |row| row.get(0),
        )
        .optional()
        .context("failed to read schema_version from meta table")?;

    match version.as_deref() {
        Some(schema::SCHEMA_VERSION) => Ok(()),
        Some(version) => {
            bail!("unsupported Patina index schema_version `{version}`; run `patina index --reset`")
        }
        None => bail!("Patina index is missing schema_version; run `patina index --reset`"),
    }
}

pub fn check_fts5_available(conn: &Connection) -> bool {
    conn.execute_batch(
        "CREATE VIRTUAL TABLE IF NOT EXISTS patina_fts5_probe USING fts5(value);
         DROP TABLE IF EXISTS patina_fts5_probe;",
    )
    .is_ok()
}

fn insert_meta_if_absent(conn: &Connection, key: &str, value: &str) -> Result<()> {
    conn.execute(
        "INSERT OR IGNORE INTO meta (key, value) VALUES (?1, ?2)",
        params![key, value],
    )
    .with_context(|| format!("failed to initialize meta key {key}"))?;
    Ok(())
}

fn ensure_documents_scope_classification_column(conn: &Connection) -> Result<()> {
    let mut statement = conn
        .prepare("PRAGMA table_info(documents)")
        .context("failed to inspect documents schema")?;
    let columns = statement
        .query_map([], |row| row.get::<_, String>(1))
        .context("failed to read documents columns")?
        .collect::<std::result::Result<Vec<_>, _>>()
        .context("failed to collect documents columns")?;

    if !columns
        .iter()
        .any(|column| column == "scope_classification")
    {
        conn.execute(
            "ALTER TABLE documents ADD COLUMN scope_classification TEXT",
            [],
        )
        .context("failed to add documents.scope_classification column")?;
    }

    Ok(())
}

fn upsert_meta(conn: &Connection, key: &str, value: &str) -> Result<()> {
    conn.execute(
        "INSERT INTO meta (key, value) VALUES (?1, ?2)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        params![key, value],
    )
    .with_context(|| format!("failed to update meta key {key}"))?;
    Ok(())
}
