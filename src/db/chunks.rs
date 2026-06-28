use anyhow::{Context, Result};
use rusqlite::{Connection, params};

use crate::index::chunker::Chunk;

pub fn replace_for_document(
    conn: &Connection,
    document_id: i64,
    chunks: &[Chunk],
    fts5_available: bool,
) -> Result<()> {
    if fts5_available {
        conn.execute(
            "DELETE FROM chunks_fts WHERE rowid IN (SELECT id FROM chunks WHERE document_id = ?1)",
            params![document_id],
        )
        .context("failed to delete old FTS chunks")?;
    }

    conn.execute(
        "DELETE FROM chunks WHERE document_id = ?1",
        params![document_id],
    )
    .context("failed to delete old chunks")?;

    for chunk in chunks {
        conn.execute(
            "INSERT INTO chunks (
                document_id, ordinal, heading_path, text, token_estimate, sha256
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                document_id,
                chunk.ordinal as i64,
                serde_json::to_string(&chunk.heading_path)?,
                chunk.text,
                chunk.token_estimate as i64,
                chunk.sha256,
            ],
        )
        .context("failed to insert chunk")?;

        if fts5_available {
            let rowid = conn.last_insert_rowid();
            conn.execute(
                "INSERT INTO chunks_fts(rowid, text) VALUES (?1, ?2)",
                params![rowid, chunk.text],
            )
            .context("failed to insert FTS chunk")?;
        }
    }

    Ok(())
}
