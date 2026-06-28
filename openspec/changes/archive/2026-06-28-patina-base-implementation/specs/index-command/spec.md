## ADDED Requirements

### Requirement: Heading-aware deterministic chunking
`patina index` SHALL chunk each Markdown file by its heading tree. Each heading section becomes a logical chunk. The algorithm SHALL be:
1. Parse the Markdown file.
2. Extract YAML front matter.
3. Build a heading tree from Markdown headings.
4. Treat each heading section as a logical chunk.
5. Include the heading path with each chunk.
6. If a section exceeds `chunk_size` (estimated tokens), split at paragraph boundaries.
7. Apply overlap (`chunk_overlap`) only when a large section is split.
8. Compute a SHA-256 hash for each chunk.
9. Store ordinal, heading path, text, token estimate, and hash.

#### Scenario: Document with multiple headings
- **WHEN** a Markdown file contains `# H1`, `## H2a`, and `## H2b` sections
- **THEN** three chunks are produced with heading paths `["H1"]`, `["H1", "H2a"]`, `["H1", "H2b"]`

#### Scenario: Section exceeding chunk_size is split
- **WHEN** a heading section's token estimate exceeds `chunk_size`
- **THEN** the section is split into sub-chunks at paragraph boundaries; each sub-chunk has the same heading path

#### Scenario: Same input always produces same chunks
- **WHEN** the same Markdown file is indexed twice
- **THEN** chunk count, ordinals, heading paths, text, and SHA-256 hashes are identical on both runs

### Requirement: Token count estimation
Patina SHALL estimate token count using `ceil(char_count / 4)`. This approximation SHALL be the same on all platforms.

#### Scenario: Token estimate is deterministic
- **WHEN** a chunk has 400 characters
- **THEN** the token estimate stored in the database is 100

### Requirement: SHA-256 per chunk
Each chunk SHALL have a SHA-256 hash of its text content stored in the `chunks.sha256` column.

#### Scenario: Chunk hash is computed
- **WHEN** a chunk with known text is indexed
- **THEN** the stored `sha256` matches the SHA-256 of the chunk text

### Requirement: Full index rebuild mode
`patina index --full` SHALL process all Markdown files regardless of whether they appear changed since the last index run.

#### Scenario: --full re-indexes all files
- **WHEN** `patina index --full` is run
- **THEN** every Markdown file in the knowledge directory is re-parsed and its chunks are replaced

### Requirement: Reset mode
`patina index --reset` SHALL delete all existing index data and perform a fresh full rebuild using an atomic temporary file.

#### Scenario: --reset rebuilds from scratch
- **WHEN** `patina index --reset` is run
- **THEN** the old index data is discarded and a new complete index is built

### Requirement: Source reference hash capture
When a document with `source_refs` is indexed, Patina SHALL record the current SHA-256 hash and modification time of each referenced source file in the `source_refs` table.

#### Scenario: Source file hash is recorded
- **WHEN** a page with `source_refs: ["knowledge/sources/notes.md"]` is indexed
- **THEN** `source_refs.source_hash_at_index` contains the SHA-256 of `notes.md` at index time

### Requirement: Scope classification capture at index time
When a document is indexed, Patina SHALL read the `scope` field from `knowledge/scope.yaml` (if present) and store it in `documents.scope_classification`. If no `scope.yaml` exists or the field is absent, `scope_classification` SHALL be stored as NULL.

#### Scenario: Scope classification stored during indexing

- **WHEN** `knowledge/scope.yaml` has `scope: client-confidential` and `patina index` runs
- **THEN** every indexed document row has `scope_classification = "client-confidential"`

#### Scenario: No scope.yaml — classification stored as NULL

- **WHEN** no `knowledge/scope.yaml` exists
- **THEN** every indexed document row has `scope_classification = NULL`

### Requirement: Index JSON output
`patina index --json` SHALL return the standard JSON envelope with indexing statistics in `data` (files processed, chunks created, errors encountered).

#### Scenario: Successful index run with --json
- **WHEN** `patina index --json` completes successfully
- **THEN** `ok` is `true` and `data` contains file and chunk counts

#### Scenario: Index run with errors with --json
- **WHEN** some files fail to parse during `patina index --json`
- **THEN** `ok` is `false` and `errors` lists the failed files with their error codes
