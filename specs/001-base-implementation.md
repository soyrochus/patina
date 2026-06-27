# Patina — Specification Amendments for v0.1.1

## 1. Version Scope

These amendments refine the foundational Patina specification for a v0.1 implementation.

The purpose is not to change the product direction. The purpose is to remove ambiguity from the implementation contract.

Patina remains:

```text
local-first
Rust-based
CLI-first
Git-compatible
Markdown-based
SQLite-indexed
agent-usable
MCP-free in v0.1
Chroma-free in v0.1
Obsidian-independent
```

The v0.1 implementation must favour deterministic, inspectable behaviour over semantic sophistication.

## 2. Roadmap Boundaries

Patina shall use explicit version boundaries.

### v0.1.0 — Core Local Knowledge CLI

Required:

```text
patina init
patina status
patina lint
patina index
patina query
patina read
patina stale
patina install-agent
patina doctor
```

Required capabilities:

```text
Markdown discovery
YAML front matter parsing
Git-aware file walking
SQLite local index
SQLite FTS5 retrieval
heading-aware chunking
basic provenance validation
basic stale detection
JSON output envelope
agent instruction installation
cross-platform binaries
```

Not required:

```text
MCP
semantic vector retrieval
binary source extraction
web UI
multi-root federated retrieval
automatic synthesis rewriting
```

### v0.2.0 — Extraction and Semantic Retrieval

Candidate capabilities:

```text
PDF/DOCX/PPTX/HTML extraction
optional local embeddings
SQLite vector storage
hybrid lexical + semantic retrieval
improved stale-source detection
page-type-specific validation
```

### v0.3.0 — Agent Tooling and Governance

Candidate capabilities:

```text
MCP adapter
local read-only server mode
scope-aware export checks
patch proposal workflows
cross-root retrieval
knowledge graph visualisation
```

MCP must remain an adapter over the CLI contract, not a replacement for it.

## 3. Chunking Strategy

Patina shall use deterministic heading-aware chunking for Markdown files.

The default algorithm is:

```text
1. Parse the Markdown file.
2. Extract YAML front matter.
3. Build a heading tree from Markdown headings.
4. Treat each heading section as a logical chunk.
5. Include the heading path with each chunk.
6. If a section exceeds the configured chunk size, split it into smaller sub-chunks.
7. Preserve paragraph boundaries where possible.
8. Apply overlap only when a large section is split.
9. Compute a SHA-256 hash for each chunk.
10. Store the chunk ordinal, heading path, text, and hash.
```

Default chunk configuration:

```toml
[index]
chunk_size = 1200
chunk_overlap = 150
chunk_strategy = "heading-aware"
```

The default unit for `chunk_size` is an estimated token count, not characters.

Patina may estimate tokens using a simple deterministic approximation in v0.1. The algorithm must be documented and stable enough that the same input produces the same chunk boundaries.

Patina shall not randomly chunk content.

Patina shall not use model-dependent tokenisation in v0.1 unless that tokenizer is embedded, deterministic, and cross-platform.

## 4. Schema and Validation Model

Patina v0.1 shall not require full JSON Schema validation.

Initial validation shall use:

```text
front matter parsing
required key checks
allowed value checks
page-type checks
path checks
link checks
source-reference checks
custom structural rules
```

The schema files under `knowledge/schemas/` shall be generated as documentation and future extension points, but v0.1 validation shall be implemented through typed Rust structs and explicit validation rules.

Full JSON Schema enforcement may be introduced in v0.2 or later.

Default required front matter:

```yaml
title: string
type: string
status: string
```

Default allowed page statuses:

```text
draft
active
deprecated
archived
```

Default allowed page types:

```text
concept
system
project
decision
person
process
glossary
source
index
open-question
```

Page-type-specific rules may be configured.

Example:

```toml
[lint.page_types.decision]
required = ["title", "type", "status", "decided_on"]

[lint.page_types.source]
required = ["title", "type", "status", "source_kind"]
```

If no page-type-specific rule is configured, Patina shall apply the default required fields.

## 5. Provenance and Source Hashes

Patina shall validate that declared source references exist.

Patina shall also store source hashes and detect stale provenance.

The `source_refs` table shall include:

```sql
source_refs(
  id INTEGER PRIMARY KEY,
  document_id INTEGER NOT NULL,
  source_path TEXT NOT NULL,
  source_hash_at_index TEXT,
  source_modified_at_index TEXT,
  referenced_from TEXT,
  FOREIGN KEY(document_id) REFERENCES documents(id)
);
```

When `patina stale` or `patina lint` runs, Patina shall compare the current source hash with the stored hash.

If a source file has changed since the referencing page was last indexed, Patina shall emit a warning:

```text
warning: source changed after page synthesis

Page:
  knowledge/wiki/systems/control-plane.md

Source:
  knowledge/sources/extracted/workshop-notes.md

Suggested action:
  Review whether the synthesized page still reflects the source.
```

A stale source reference is a warning in v0.1, not an error.

Missing source references are errors when the page explicitly declares them.

## 6. Scope and Confidentiality Rules

`knowledge/scope.yaml` is optional.

If present, Patina shall parse and index it.

Example:

```yaml
scope: client-confidential
client: Example Client
allowed_exports:
  - anonymized-patterns
  - architectural-abstractions
forbidden_exports:
  - names
  - credentials
  - commercial terms
  - system identifiers
  - client-specific incidents
```

In v0.1, Patina shall not attempt semantic detection of copied confidential material.

It shall implement only deterministic checks:

```text
1. Warn if a page references a source outside the current knowledge root.
2. Warn if a page changes confidentiality classification compared with the indexed version.
3. Warn if a source reference points to a path whose root has incompatible scope metadata.
4. Display scope metadata in patina status and patina doctor.
```

Cross-root scope enforcement is reserved for a later version.

## 7. Multi-Root Forward Compatibility

Patina v0.1 shall operate on a single knowledge root.

The configuration model shall be forward-compatible with future multi-root workspaces.

Repository configuration may contain:

```toml
[knowledge]
dir = "knowledge"

[workspace]
enabled = false
roots = []
```

In v0.1, if `workspace.enabled = true`, Patina shall fail with:

```text
error: multi-root workspaces are not supported in this version
```

This prevents a breaking configuration change later.

## 8. SQLite and FTS5 Requirements

Patina shall prefer shipping with SQLite FTS5 enabled.

The v0.1 implementation should use a SQLite build configuration where FTS5 is available on all supported platforms.

If FTS5 is unavailable, Patina shall fall back to a deterministic but degraded lexical search:

```sql
SELECT ...
FROM chunks
WHERE lower(text) LIKE lower('%term%')
```

The fallback mode shall:

```text
support simple term matching
not claim BM25 ranking
emit a clear warning
appear in JSON output as mode = "lexical-fallback"
```

Example warning:

```text
warning: SQLite FTS5 is unavailable; using degraded LIKE-based search
```

The preferred implementation is FTS5. The fallback exists for resilience, not as the normal path.

## 9. Retrieval Scoring

Patina query scoring shall be transparent.

The default v0.1 score shall be a weighted combination of lexical relevance and deterministic metadata bonuses.

Default formula:

```text
score =
  normalized_fts_score * 0.70
+ title_match_bonus    * 0.10
+ alias_match_bonus    * 0.07
+ tag_match_bonus      * 0.05
+ page_type_bonus      * 0.03
+ freshness_bonus      * 0.03
+ provenance_bonus     * 0.02
```

All components shall be normalized to the range `0.0..1.0`.

If FTS5 returns BM25 values where lower is better, Patina shall normalize them before combining.

The JSON output shall include score components when requested:

```bash
patina query "controlled autonomy" --json --explain
```

Example:

```json
{
  "path": "knowledge/wiki/concepts/controlled-agent-autonomy.md",
  "score": 0.91,
  "score_components": {
    "fts": 0.86,
    "title": 1.0,
    "alias": 1.0,
    "tag": 0.5,
    "page_type": 0.3,
    "freshness": 0.8,
    "provenance": 0.6
  },
  "matches": ["fts", "title", "alias"]
}
```

The scoring formula may be configurable later, but v0.1 shall provide one stable default.

## 10. Optional Semantic Retrieval Storage

Semantic retrieval is not required in v0.1.

When introduced, embeddings shall be stored as generated local state in `.patina/`.

The preferred future storage model is a separate table:

```sql
chunk_vectors(
  chunk_id INTEGER PRIMARY KEY,
  model TEXT NOT NULL,
  dimensions INTEGER NOT NULL,
  vector BLOB NOT NULL,
  chunk_hash TEXT NOT NULL,
  embedded_at TEXT NOT NULL,
  FOREIGN KEY(chunk_id) REFERENCES chunks(id)
);
```

Embeddings shall be invalidated when:

```text
chunk hash changes
embedding model changes
embedding dimensions change
semantic index version changes
```

Patina shall never store embeddings as authoritative knowledge.

## 11. SQLite Schema Migrations

Patina shall include a schema metadata table.

```sql
meta(
  key TEXT PRIMARY KEY,
  value TEXT NOT NULL
);
```

Required keys:

```text
schema_version
patina_version
created_at
updated_at
```

On startup, Patina shall inspect `schema_version`.

If the schema version is unsupported, Patina shall fail with a clear error and suggest:

```bash
patina index --reset
```

For v0.1, destructive rebuild of `.patina/index.sqlite` is acceptable because the index is disposable.

Future versions may include incremental migrations.

## 12. Regeneration Contract

Deleting `.patina/` must never destroy shared knowledge.

This must always be valid:

```bash
rm -rf .patina
patina index --full
```

On Windows, the equivalent removal of `.patina/` followed by `patina index --full` must also recover a working local index.

Patina documentation shall state:

```text
If the local index becomes corrupt, delete .patina/ and run patina index --full.
```

## 13. Locking and Concurrency

Patina shall protect local index writes.

Any command that writes to `.patina/index.sqlite` shall acquire a lock before writing.

Required write-lock commands:

```text
patina index
patina index --reset
future semantic embedding commands
future extraction cache commands
```

Patina shall use:

```text
SQLite WAL mode
busy timeout
advisory file lock on .patina/index.lock
```

If another Patina process holds the lock, the command shall fail fast unless a wait option is provided.

Default behaviour:

```text
fail after a short timeout with a clear message
```

Example:

```text
error: Patina index is currently locked by another process

Lock file:
  .patina/index.lock

Suggested action:
  Wait for the other process to finish, or remove the lock only if no Patina process is running.
```

Patina shall avoid database corruption under concurrent invocations.

## 14. Atomic Local Writes

Patina shall make local index updates atomic where practical.

For full rebuilds:

```text
1. Build a new SQLite database at .patina/index.sqlite.tmp.
2. Validate database integrity.
3. Replace .patina/index.sqlite using an atomic rename where supported.
4. Remove temporary files.
```

For incremental updates, Patina shall use SQLite transactions.

A failed indexing operation shall not leave the index in a partially committed logical state.

## 15. `patina doctor`

`patina doctor` shall diagnose the local Patina environment.

It shall check:

```text
current directory is inside or near a Git worktree
knowledge directory exists
knowledge/README.md exists
knowledge/AGENTS.md exists
.patina/ exists or can be created
.patina/ is ignored by Git
SQLite database exists if indexed
SQLite integrity check passes
FTS5 is available
local index schema version is supported
file permissions allow reads and local writes
agent instruction files are present where expected
scope.yaml is valid if present
large-file limits are configured
```

It shall report:

```text
ok
warning
error
```

Example:

```bash
patina doctor
patina doctor --json
```

`patina doctor` shall not modify files unless an explicit future `--fix` option is added.

## 16. Path Safety and Symlink Policy

`patina read` and other file-reading commands shall canonicalize paths before access.

Required behaviour:

```text
1. Resolve the requested path.
2. Canonicalize the path.
3. Canonicalize the configured knowledge root.
4. Verify that the requested path remains inside the knowledge root.
5. Reject paths outside the root.
```

Patina shall prevent path traversal such as:

```text
../../secrets.txt
```

Default symlink policy:

```text
Do not follow symlinks that resolve outside the knowledge root.
Warn on symlinks during lint.
Allow internal symlinks only if configured.
```

Configuration:

```toml
[security]
allow_internal_symlinks = false
allow_external_symlinks = false
```

External symlinks shall be rejected by default.

## 17. Stale Detection Implementation

`patina stale` shall use indexed metadata and current file hashes.

A page is stale when one or more deterministic signals is present:

```text
review_after date has passed
page source reference hash changed
page source reference is missing
linked page hash changed since last index
linked decision page changed since last index
page is draft beyond configured age
page has deprecated status but is still linked from active pages
page has no source references where configured type requires them
```

The documents table shall store enough metadata to support this.

Additional fields:

```sql
documents(
  id INTEGER PRIMARY KEY,
  path TEXT NOT NULL UNIQUE,
  title TEXT,
  type TEXT,
  status TEXT,
  sha256 TEXT NOT NULL,
  modified_at TEXT,
  indexed_at TEXT,
  front_matter_updated TEXT,
  review_after TEXT
);
```

Stale results shall include reasons.

Example JSON:

```json
{
  "version": "0.1",
  "ok": true,
  "data": {
    "stale_pages": [
      {
        "path": "knowledge/wiki/systems/control-plane.md",
        "reasons": [
          {
            "code": "source_hash_changed",
            "severity": "warning",
            "source": "knowledge/sources/extracted/workshop-notes.md"
          }
        ]
      }
    ]
  }
}
```

## 18. JSON Output Envelope

All `--json` output shall use a stable envelope.

Default envelope:

```json
{
  "version": "0.1",
  "command": "query",
  "ok": true,
  "data": {},
  "warnings": [],
  "errors": []
}
```

On failure:

```json
{
  "version": "0.1",
  "command": "lint",
  "ok": false,
  "data": null,
  "warnings": [],
  "errors": [
    {
      "code": "missing_front_matter",
      "message": "Page is missing YAML front matter",
      "path": "knowledge/wiki/concepts/example.md",
      "severity": "error"
    }
  ]
}
```

All JSON-producing commands shall include:

```text
version
command
ok
data
warnings
errors
```

This envelope is required for stable agent integration.

## 19. Large File Handling

Patina shall define default limits.

Default limits:

```toml
[limits]
max_markdown_file_mb = 10
max_source_file_mb = 50
max_total_markdown_files = 50000
max_chunk_token_estimate = 1200
```

If a Markdown file exceeds the configured maximum, Patina shall skip it and emit a warning.

If the total file count exceeds the configured threshold, Patina shall warn that performance may degrade.

Patina shall not fail solely because a repository is large unless a hard configured limit is exceeded.

## 20. Git Integration

Patina shall detect whether it is running inside a Git worktree.

`patina init` shall warn if no Git repository is detected.

It shall still allow initialization outside Git if the user explicitly confirms or passes:

```bash
patina init --no-git
```

`patina status` shall report:

```text
Git worktree detected: yes/no
Uncommitted knowledge changes: yes/no
.patina ignored by Git: yes/no
```

Patina shall warn if `.patina/` or local index files are staged for commit.

Patina shall not automatically commit, push, pull, or merge.

## 21. Distribution

Patina shall be distributed as standalone cross-platform binaries where possible.

Required targets:

```text
macOS arm64
macOS x64
Windows x64
Linux x64
```

Preferred distribution channels:

```text
GitHub Releases
cargo install
Homebrew tap
Windows downloadable archive or installer
```

The initial implementation may use `cargo install` and GitHub Releases only.

Packaging shall avoid requiring users to install Python, Node.js, ChromaDB, or a database server.

## 22. Testing Requirements

Patina shall include automated tests from the first implementation.

Required test categories:

```text
unit tests
integration tests
fixture repository tests
golden output tests
cross-platform path tests
SQLite migration tests
lint rule tests
query ranking tests
```

A small fixture repository shall be included under test resources.

Fixture scenarios:

```text
valid knowledge directory
missing front matter
broken internal link
missing source reference
stale source hash
duplicate alias
large file warning
query returning expected ranked results
```

Golden tests shall verify stable CLI output for core commands.

JSON output tests shall verify the response envelope.

Path safety tests shall verify traversal rejection and symlink policy.

## 23. Dependency Clarifications

The first implementation shall prefer a single Rust binary crate with internal modules.

Suggested crates remain:

```text
clap
serde
serde_json
toml
anyhow
thiserror
rusqlite
walkdir
ignore
sha2
chrono
tracing
```

Recommended additions:

```text
camino or typed-path for safer UTF-8 path handling
comrak or pulldown-cmark for Markdown parsing
gray_matter or equivalent for front matter parsing
```

Full JSON Schema validation is not required in v0.1.

The `jsonschema` crate or equivalent may be considered in v0.2.

## 24. Updated Design Invariants

Patina shall preserve the original invariants and add the following:

```text
Chunking is deterministic.
Retrieval scoring is explainable.
JSON output is versioned.
Local index writes are locked.
Full index rebuild is always possible.
Schema version is explicit.
SQLite FTS5 is the normal lexical retrieval engine.
Fallback search is visibly degraded.
Agent instructions are thin wrappers over the CLI.
The CLI contract comes before any MCP adapter.
```

## 25. Updated Acceptance Criteria

Patina v0.1 is acceptable when all of the following are true:

```text
A repository can be initialized with patina init.
The generated structure is usable without Obsidian.
.patina/ is ignored by Git.
patina doctor reports environment state.
Markdown pages can be linted.
Lint detects missing front matter, broken links, duplicate aliases, and missing source refs.
patina index builds a SQLite database.
The SQLite database includes schema_version metadata.
patina index uses locking during writes.
patina index --reset rebuilds from Git-tracked files.
Deleting .patina/ and running patina index --full recovers the local state.
patina query returns ranked results.
patina query --json returns the standard JSON envelope.
patina query --explain reports scoring signals.
patina read rejects paths outside the knowledge root.
patina stale detects expired review_after dates and changed source hashes.
patina install-agent writes at least generic agent instructions.
No v0.1 command requires MCP.
No v0.1 command requires ChromaDB.
No v0.1 command requires a network connection.
The binary runs on macOS, Windows, and Linux.
Golden tests cover init, lint, index, query, read, stale, and doctor.
```

## 26. Implementation Priority

The recommended implementation sequence is:

```text
1. CLI skeleton and configuration loading
2. patina init
3. Git-aware knowledge directory discovery
4. Markdown/front matter parser
5. patina lint
6. SQLite schema and migrations
7. patina index with deterministic chunking
8. SQLite FTS5 search
9. patina query with transparent scoring
10. patina read with path safety
11. patina stale
12. patina doctor
13. patina install-agent
14. JSON envelope support across commands
15. fixture and golden tests
16. packaging for initial platforms
```

This order minimizes architectural risk. It establishes the deterministic substrate before adding agent-facing convenience.

## 27. Summary

These amendments convert Patina from a strong conceptual foundation into a more precise implementation contract.

The essential tightening points are:

```text
heading-aware deterministic chunking
manual v0.1 validation rules
explicit FTS5-first retrieval
transparent scoring
schema versioning
index locking
atomic local writes
stable JSON envelopes
safe path canonicalization
defined stale-source detection
doctor command definition
test and packaging requirements
```

The product direction remains unchanged: Patina is a Rust-based, local-first, Git-compatible Markdown knowledge tool with deterministic retrieval and thin agent instruction adapters.
