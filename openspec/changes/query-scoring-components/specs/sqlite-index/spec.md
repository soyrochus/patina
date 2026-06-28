## MODIFIED Requirements

### Requirement: documents table
Patina SHALL maintain a `documents` table with the following columns: `id INTEGER PRIMARY KEY`, `path TEXT NOT NULL UNIQUE`, `title TEXT`, `type TEXT`, `status TEXT`, `sha256 TEXT NOT NULL`, `modified_at TEXT`, `indexed_at TEXT`, `front_matter_updated TEXT`, `review_after TEXT`, `scope_classification TEXT`, `aliases TEXT`, `tags TEXT`.

The `scope_classification` column SHALL store the value of the `scope` field from `knowledge/scope.yaml` at the time the document was last indexed. If no `scope.yaml` exists or no `scope` field is present, the column SHALL be NULL.

The `aliases` column SHALL store the page front matter `aliases` sequence as a JSON-encoded array string when present, and SHALL be NULL when absent or when no string alias values are present.

The `tags` column SHALL store the page front matter `tags` sequence as a JSON-encoded array string when present, and SHALL be NULL when absent or when no string tag values are present.

#### Scenario: Document is indexed
- **WHEN** a Markdown file is indexed
- **THEN** a row is inserted or updated in `documents` with the correct `path`, `sha256`, and front matter fields

#### Scenario: scope_classification is recorded at index time

- **WHEN** `knowledge/scope.yaml` contains `scope: client-confidential` and a document is indexed
- **THEN** `documents.scope_classification` for that document is `"client-confidential"`

#### Scenario: scope_classification is NULL when scope.yaml is absent

- **WHEN** no `knowledge/scope.yaml` exists
- **THEN** `documents.scope_classification` is NULL for all documents

#### Scenario: aliases are recorded as JSON
- **WHEN** a Markdown file with front matter `aliases: ["controlled autonomy", "agent autonomy"]` is indexed
- **THEN** `documents.aliases` for that document contains the JSON array `["controlled autonomy","agent autonomy"]`

#### Scenario: tags are recorded as JSON
- **WHEN** a Markdown file with front matter `tags: ["agents", "architecture"]` is indexed
- **THEN** `documents.tags` for that document contains the JSON array `["agents","architecture"]`

#### Scenario: absent aliases and tags are NULL
- **WHEN** a Markdown file without `aliases` or `tags` front matter is indexed
- **THEN** `documents.aliases` and `documents.tags` are NULL for that document

### Requirement: Schema version validation on startup
On startup, if the SQLite database exists, Patina SHALL read `schema_version` from `meta`. If the version is not supported, Patina SHALL exit with a clear error and suggest `patina index --reset`.

#### Scenario: Supported schema version
- **WHEN** the database has `schema_version = "2"`
- **THEN** Patina proceeds normally

#### Scenario: Version 1 schema is unsupported
- **WHEN** the database has `schema_version = "1"`
- **THEN** Patina exits with an error message and suggests running `patina index --reset`

#### Scenario: Unsupported schema version
- **WHEN** the database has an unrecognised `schema_version`
- **THEN** Patina exits with an error message and suggests running `patina index --reset`
