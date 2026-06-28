## Why

AI-assisted knowledge workflows are ephemeral — synthesis disappears into chat history and teams repeatedly re-explain the same context to tools. Patina makes that synthesis durable: a Rust-based, local-first, Git-compatible Markdown knowledge tool with deterministic retrieval and thin agent instruction adapters, now specified precisely enough to implement.

## What Changes

- New Rust binary crate `patina` with all v0.1.0 CLI commands
- Knowledge directory structure initialized by `patina init`
- YAML front matter validation and Markdown link linting via `patina lint`
- SQLite-backed local index built by `patina index` with deterministic heading-aware chunking
- FTS5 lexical retrieval with transparent weighted scoring via `patina query`
- Path-safe file reading via `patina read`
- Stale source and review-date detection via `patina stale`
- Environment diagnostics via `patina doctor`
- Agent instruction file generation via `patina install-agent`
- Stable JSON output envelope across all commands
- Comprehensive test suite: unit, integration, fixture, and golden output tests
- Cross-platform binary distribution (macOS arm64/x64, Linux x64, Windows x64)

## Capabilities

### New Capabilities

- `cli-foundation`: CLI skeleton with `clap`, TOML config loading, JSON output envelope (`version`/`command`/`ok`/`data`/`warnings`/`errors`), cross-platform binary distribution
- `init-command`: `patina init` — scaffold knowledge directory, detect Git worktree, write `.gitignore` entry for `.patina/`, warn without `--no-git` if outside Git
- `knowledge-discovery`: Markdown file walking (Git-aware via `ignore` crate), YAML front matter parsing, `scope.yaml` handling, large-file and file-count limits
- `lint-command`: `patina lint` — required front matter keys, allowed status/type values, page-type-specific rules, internal link validation, alias uniqueness, source reference existence
- `sqlite-index`: SQLite schema (`meta`, `documents`, `chunks`, `source_refs`), FTS5 virtual table, schema versioning/migrations, WAL mode, advisory lock, atomic rebuild via tmp-file rename
- `index-command`: `patina index` — deterministic heading-aware chunking, token-count estimation, SHA-256 per chunk, incremental and full-rebuild modes
- `query-command`: `patina query` — FTS5 BM25 search with LIKE fallback, transparent weighted scoring (fts 0.70 + title 0.10 + alias 0.07 + tag 0.05 + page_type 0.03 + freshness 0.03 + provenance 0.02), `--explain` score components
- `read-command`: `patina read` — path canonicalization, knowledge-root boundary enforcement, symlink policy
- `stale-command`: `patina stale` — review_after expiry, source hash drift, deprecated pages still linked from active pages, draft age threshold
- `doctor-command`: `patina doctor` — Git worktree check, knowledge dir/README/AGENTS.md presence, `.patina/` permissions, SQLite integrity, FTS5 availability, schema version, scope.yaml validity
- `install-agent-command`: `patina install-agent` — write generic agent instruction files to `knowledge/AGENTS.md` and tool-specific locations
- `test-suite`: fixture knowledge repository, unit tests, integration tests, golden CLI output tests, JSON envelope tests, path-safety tests, SQLite migration tests, query ranking tests

### Modified Capabilities

## Impact

- New top-level Rust binary crate (`src/main.rs`, `src/lib.rs` and module tree)
- Dependencies: `clap`, `serde`/`serde_json`, `toml`, `anyhow`, `thiserror`, `rusqlite`, `walkdir`, `ignore`, `sha2`, `chrono`, `tracing`, `comrak` or `pulldown-cmark`, `gray_matter`
- `.patina/` local directory created at runtime (SQLite DB, lock file); must be `.gitignore`d
- `knowledge/` directory created by `patina init`; committed to Git
- No network dependencies, no MCP, no ChromaDB, no Python or Node.js runtime required
