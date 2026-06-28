## ADDED Requirements

### Requirement: SQLite schema with meta table
Patina SHALL create a `meta` table in the SQLite database with at minimum the keys: `schema_version`, `patina_version`, `created_at`, `updated_at`.

#### Scenario: Fresh database creation
- **WHEN** `patina index` runs on a new repository
- **THEN** the `meta` table contains `schema_version`, `patina_version`, `created_at`, and `updated_at`

#### Scenario: schema_version is readable
- **WHEN** the SQLite database exists
- **THEN** `SELECT value FROM meta WHERE key = 'schema_version'` returns the version string

### Requirement: documents table
Patina SHALL maintain a `documents` table with the following columns: `id INTEGER PRIMARY KEY`, `path TEXT NOT NULL UNIQUE`, `title TEXT`, `type TEXT`, `status TEXT`, `sha256 TEXT NOT NULL`, `modified_at TEXT`, `indexed_at TEXT`, `front_matter_updated TEXT`, `review_after TEXT`, `scope_classification TEXT`.

The `scope_classification` column SHALL store the value of the `scope` field from `knowledge/scope.yaml` at the time the document was last indexed. If no `scope.yaml` exists or no `scope` field is present, the column SHALL be NULL.

#### Scenario: Document is indexed
- **WHEN** a Markdown file is indexed
- **THEN** a row is inserted or updated in `documents` with the correct `path`, `sha256`, and front matter fields

#### Scenario: scope_classification is recorded at index time

- **WHEN** `knowledge/scope.yaml` contains `scope: client-confidential` and a document is indexed
- **THEN** `documents.scope_classification` for that document is `"client-confidential"`

#### Scenario: scope_classification is NULL when scope.yaml is absent

- **WHEN** no `knowledge/scope.yaml` exists
- **THEN** `documents.scope_classification` is NULL for all documents

### Requirement: chunks table
Patina SHALL maintain a `chunks` table with columns: `id INTEGER PRIMARY KEY`, `document_id INTEGER NOT NULL`, `ordinal INTEGER NOT NULL`, `heading_path TEXT`, `text TEXT NOT NULL`, `token_estimate INTEGER`, `sha256 TEXT NOT NULL`, and a foreign key to `documents`.

#### Scenario: Chunk is stored
- **WHEN** a document is chunked
- **THEN** each chunk row in `chunks` has a non-null `sha256`, correct `ordinal`, and the `heading_path` reflecting the Markdown heading hierarchy

### Requirement: source_refs table
Patina SHALL maintain a `source_refs` table with columns: `id INTEGER PRIMARY KEY`, `document_id INTEGER NOT NULL`, `source_path TEXT NOT NULL`, `source_hash_at_index TEXT`, `source_modified_at_index TEXT`, `referenced_from TEXT`, and a foreign key to `documents`.

#### Scenario: Source reference is recorded
- **WHEN** a document with `source_refs` is indexed
- **THEN** a row is inserted in `source_refs` for each declared source path with its hash and modification time

### Requirement: FTS5 virtual table
Patina SHALL create a FTS5 virtual table over the `chunks` text content. If FTS5 is unavailable, Patina SHALL fall back to LIKE-based search and emit a clear warning.

#### Scenario: FTS5 is available
- **WHEN** SQLite is built with FTS5 support
- **THEN** `patina query` uses BM25 ranking and does not emit a degraded-search warning

#### Scenario: FTS5 is unavailable
- **WHEN** SQLite is built without FTS5
- **THEN** `patina query` uses LIKE-based search, emits `warning: SQLite FTS5 is unavailable; using degraded LIKE-based search`, and JSON output includes `mode = "lexical-fallback"`

### Requirement: Schema version validation on startup
On startup, if the SQLite database exists, Patina SHALL read `schema_version` from `meta`. If the version is not supported, Patina SHALL exit with a clear error and suggest `patina index --reset`.

#### Scenario: Supported schema version
- **WHEN** the database has `schema_version = "1"`
- **THEN** Patina proceeds normally

#### Scenario: Unsupported schema version
- **WHEN** the database has an unrecognised `schema_version`
- **THEN** Patina exits with an error message and suggests running `patina index --reset`

### Requirement: WAL mode and busy timeout
Patina SHALL open the SQLite database in WAL mode and set a busy timeout before acquiring any write lock.

#### Scenario: SQLite opened in WAL mode
- **WHEN** Patina opens `index.sqlite`
- **THEN** `PRAGMA journal_mode` returns `wal`

### Requirement: Advisory file lock on index operations
Any command that writes to the SQLite index SHALL acquire an advisory lock on `.patina/index.lock` before writing. If another Patina process holds the lock, the command SHALL fail with a descriptive error after a short timeout.

#### Scenario: Lock is acquired during indexing
- **WHEN** `patina index` runs
- **THEN** `.patina/index.lock` is created and held for the duration of the write

#### Scenario: Concurrent indexing attempt
- **WHEN** a second `patina index` runs while the first holds the lock
- **THEN** the second process fails with `error: Patina index is currently locked by another process` and the lock file path

### Requirement: Atomic full rebuild via temporary file
Full index rebuilds (`patina index --full` or `patina index --reset`) SHALL write to `.patina/index.sqlite.tmp`, run `PRAGMA integrity_check`, then rename to `.patina/index.sqlite`.

#### Scenario: Full rebuild completes successfully
- **WHEN** `patina index --full` completes without error
- **THEN** `.patina/index.sqlite` reflects the rebuilt content and no `.patina/index.sqlite.tmp` file remains

#### Scenario: Full rebuild fails midway
- **WHEN** `patina index --full` is interrupted during build
- **THEN** the existing `.patina/index.sqlite` is not modified; `.patina/index.sqlite.tmp` may remain but does not replace the live index

### Requirement: Incremental update transactions
Incremental index updates SHALL use SQLite transactions so that a failed update does not leave the index in a partially committed state.

#### Scenario: Incremental update succeeds
- **WHEN** a changed file is re-indexed incrementally
- **THEN** the document and chunk rows are updated atomically within a single transaction

#### Scenario: Incremental update fails
- **WHEN** an error occurs during an incremental index update
- **THEN** the transaction is rolled back and the previous index state is preserved

### Requirement: Disposable local index contract
Deleting `.patina/` and running `patina index --full` SHALL always produce a fully working index. The knowledge directory itself contains no data that must be derived from the index.

#### Scenario: Index rebuilt after .patina/ deletion
- **WHEN** `.patina/` is deleted and `patina index --full` is run
- **THEN** a valid index is created; `patina query`, `patina lint`, and `patina stale` operate correctly
