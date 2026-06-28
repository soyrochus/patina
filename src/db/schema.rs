pub const SCHEMA_VERSION: &str = "2";

pub const CREATE_META_TABLE: &str = r#"
CREATE TABLE IF NOT EXISTS meta (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
);
"#;

pub const CREATE_DOCUMENTS_TABLE: &str = r#"
CREATE TABLE IF NOT EXISTS documents (
    id INTEGER PRIMARY KEY,
    path TEXT NOT NULL UNIQUE,
    title TEXT,
    type TEXT,
    status TEXT,
    sha256 TEXT NOT NULL,
    modified_at TEXT,
    indexed_at TEXT,
    front_matter_updated TEXT,
    review_after TEXT,
    scope_classification TEXT,
    aliases TEXT,
    tags TEXT
);
"#;

pub const CREATE_CHUNKS_TABLE: &str = r#"
CREATE TABLE IF NOT EXISTS chunks (
    id INTEGER PRIMARY KEY,
    document_id INTEGER NOT NULL,
    ordinal INTEGER NOT NULL,
    heading_path TEXT,
    text TEXT NOT NULL,
    token_estimate INTEGER,
    sha256 TEXT NOT NULL,
    FOREIGN KEY(document_id) REFERENCES documents(id) ON DELETE CASCADE
);
"#;

pub const CREATE_SOURCE_REFS_TABLE: &str = r#"
CREATE TABLE IF NOT EXISTS source_refs (
    id INTEGER PRIMARY KEY,
    document_id INTEGER NOT NULL,
    source_path TEXT NOT NULL,
    source_hash_at_index TEXT,
    source_modified_at_index TEXT,
    referenced_from TEXT,
    FOREIGN KEY(document_id) REFERENCES documents(id) ON DELETE CASCADE
);
"#;

pub const CREATE_CHUNKS_FTS_TABLE: &str = r#"
CREATE VIRTUAL TABLE IF NOT EXISTS chunks_fts
USING fts5(text, content='chunks', content_rowid='id');
"#;
