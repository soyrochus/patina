## 1. Project Scaffold

- [x] 1.1 Run `cargo new --bin patina` and commit the initial Cargo.toml with binary crate structure
- [x] 1.2 Add all required dependencies to Cargo.toml: `clap` (derive feature), `serde`, `serde_json`, `toml`, `anyhow`, `thiserror`, `rusqlite` (bundled-full feature for FTS5), `walkdir`, `ignore`, `sha2`, `chrono`, `tracing`, `tracing-subscriber`, `pulldown-cmark`, `serde_yaml`, `camino`
- [x] 1.3 Create top-level module structure: `src/main.rs`, `src/lib.rs`, `src/cli/`, `src/config/`, `src/discovery/`, `src/lint/`, `src/index/`, `src/query/`, `src/read/`, `src/stale/`, `src/doctor/`, `src/agent/`, `src/db/`, `src/output/`
- [x] 1.4 Create `src/output/mod.rs` with the `JsonEnvelope` struct and serialization logic (`version`, `command`, `ok`, `data`, `warnings`, `errors`)
- [x] 1.5 Create `src/output/error.rs` with `ErrorEntry` struct (`code`, `message`, `severity`, `path` optional) and `WarningEntry` struct

## 2. Configuration Loading

- [x] 2.1 Define `PatinaConfig` struct in `src/config/mod.rs` covering all config sections: `[knowledge]`, `[index]`, `[limits]`, `[security]`, `[workspace]`, `[lint.page_types.*]`
- [x] 2.2 Implement config loading: search for `patina.toml` in the current directory and repository root; merge with compiled-in defaults if absent or partial
- [x] 2.3 Add TOML parse error handling that includes file path in the error message
- [x] 2.4 Implement `workspace.enabled` guard: fail fast with `error: multi-root workspaces are not supported in this version` if enabled

## 3. CLI Skeleton and Subcommand Dispatch

- [x] 3.1 Define the `Cli` struct and `Commands` enum in `src/cli/mod.rs` using `clap` derive macros with subcommands: `init`, `status`, `lint`, `index`, `query`, `read`, `stale`, `doctor`, `install-agent`
- [x] 3.2 Implement `main.rs` entry point: parse args, load config, dispatch to subcommand handler, map errors to JSON envelope if `--json` is set
- [x] 3.3 Add `--json` flag to all subcommands
- [x] 3.4 Ensure no-arg invocation prints help text and exits 0; unknown subcommands exit with non-zero code
- [x] 3.5 Implement global error handler that catches `anyhow::Error` and prints either human-readable or JSON envelope depending on `--json` flag

## 4. Git Worktree Detection

- [x] 4.1 Implement `git_worktree::detect()` in `src/discovery/git.rs` by checking for `.git` directory or running `git rev-parse --is-inside-work-tree`
- [x] 4.2 Implement `gitignore::is_ignored(path)` to check if a given path is listed in `.gitignore`
- [x] 4.3 Implement `gitignore::has_entry(file, entry)` to check for a specific entry in a `.gitignore` file

## 5. patina init Command

- [x] 5.1 Implement `src/cli/init.rs` handler
- [x] 5.2 Create knowledge directory scaffold: `knowledge/`, `knowledge/wiki/`, `knowledge/sources/`, `knowledge/schemas/`
- [x] 5.3 Write `knowledge/README.md` and `knowledge/AGENTS.md` stubs if they do not exist; skip without overwriting if they do
- [x] 5.4 Detect Git worktree; warn and require `--no-git` if outside Git (prompt or flag)
- [x] 5.5 Add `.patina/` to `.gitignore` (create the file if absent; append if entry is missing; skip if already present)
- [x] 5.6 Make init idempotent: second run on initialized repo preserves existing files and exits 0

## 6. Markdown and Front Matter Parser

- [x] 6.1 Implement `src/discovery/frontmatter.rs`: detect `---` fence at file start, extract YAML content, parse with `serde_yaml`, return front matter map and remaining Markdown body
- [x] 6.2 Handle files with no front matter (return empty map, no error at parse stage)
- [x] 6.3 Handle invalid YAML in front matter: return a structured parse error with file path
- [x] 6.4 Implement `src/discovery/walker.rs`: use `ignore::WalkBuilder` to walk the knowledge directory, respecting `.gitignore` and `.ignore` files
- [x] 6.5 Enforce `limits.max_markdown_file_mb`: skip files exceeding the limit and add a warning entry
- [x] 6.6 Enforce `limits.max_total_markdown_files`: emit warning if total count exceeds threshold but continue
- [x] 6.7 Implement `scope.yaml` loader in `src/discovery/scope.rs`: parse optional file, warn on malformed YAML, return `None` if absent

## 7. patina lint Command

- [x] 7.1 Implement `src/cli/lint.rs` handler; walk knowledge directory and run all lint rules on each file
- [x] 7.2 Implement required field check: validate `title`, `type`, `status` are present in front matter; emit `missing_required_field` errors
- [x] 7.3 Implement allowed value check: validate `status` against `[draft, active, deprecated, archived]` and `type` against the 10 allowed page types; emit `invalid_field_value` errors
- [x] 7.4 Implement page-type-specific required field rules from `[lint.page_types.*]` config section
- [x] 7.5 Implement internal link validator: parse Markdown body for `[[wikilinks]]` and `[text](path.md)` links; verify each resolves to an existing file within the knowledge root; emit `broken_link` errors
- [x] 7.6 Implement alias uniqueness check: collect all `aliases` values across all pages; detect duplicates; emit `duplicate_alias` errors with both file paths
- [x] 7.7 Implement source reference existence check: verify each path in `source_refs` front matter field exists on the filesystem; emit `missing_source_ref` errors
- [x] 7.8 Implement symlink warning during file walk: detect symlinks and emit a warning with the path
- [x] 7.9a Implement scope-based provenance warning (path check): warn if a `source_refs` path, when resolved, does not have the knowledge root as a prefix — this check is purely path-based and requires no index
- [x] 7.9b Implement scope-based provenance warning (classification drift): after the index exists, compare the current `scope.yaml` `scope` field against `documents.scope_classification` for each document; warn on mismatch — requires `scope_classification` column (added in schema update, recorded by task 9.7)
- [x] 7.10 Implement lint JSON output: `ok` is false if any errors; `errors` and `warnings` arrays populated from collected entries

## 8. SQLite Schema and Migrations

- [x] 8.1 Implement `src/db/schema.rs`: define SQL for `meta`, `documents`, `chunks`, `source_refs` tables and the FTS5 virtual table over `chunks.text`
- [x] 8.2 Implement database initialisation in `src/db/init.rs`: create tables if they don't exist; insert initial `meta` keys (`schema_version = "1"`, `patina_version`, `created_at`, `updated_at`)
- [x] 8.3 Implement schema version check on database open: read `schema_version` from `meta`; if unrecognised, return a structured error suggesting `patina index --reset`
- [x] 8.4 Enable WAL mode (`PRAGMA journal_mode = WAL`) and set busy timeout on every database connection
- [x] 8.5 Implement advisory file lock in `src/db/lock.rs`: create/acquire `.patina/index.lock`, fail fast with descriptive error if already locked by another process
- [x] 8.6 Implement FTS5 availability check: attempt to create or query the FTS5 table; fall back to LIKE-based search mode and set a flag if FTS5 is unavailable

## 9. patina index Command

- [x] 9.1 Implement `src/cli/index.rs` handler with `--full` and `--reset` flags
- [x] 9.2 Implement `--reset` mode: build new database at `.patina/index.sqlite.tmp`, validate with `PRAGMA integrity_check`, rename to `.patina/index.sqlite`
- [x] 9.3 Implement `--full` mode: re-process all files (same rebuild path, or truncate + re-insert within a transaction)
- [x] 9.4 Implement heading-aware chunker in `src/index/chunker.rs`: use `pulldown-cmark` events to build heading tree; split text into heading sections; split oversized sections at paragraph boundaries with overlap
- [x] 9.5 Implement token estimator: `ceil(char_count / 4)` — deterministic, platform-independent
- [x] 9.6 Implement SHA-256 per chunk using the `sha2` crate
- [x] 9.7 Implement document upsert in `src/db/documents.rs`: insert or update `documents` row with path, title, type, status, sha256, modified_at, indexed_at, front_matter_updated, review_after, and `scope_classification` (read from the parsed `scope.yaml`; NULL if absent)
- [x] 9.8 Implement chunk upsert in `src/db/chunks.rs`: delete old chunks for a document, insert new chunks with ordinal, heading_path, text, token_estimate, sha256
- [x] 9.9 Implement source reference recording in `src/db/source_refs.rs`: for each path in `source_refs` front matter, compute SHA-256 and modification time, insert into `source_refs` table
- [x] 9.10 Implement incremental indexing: compare current file SHA-256 against stored value; skip re-parsing if unchanged
- [x] 9.11 Acquire advisory lock before any write; release on completion or error
- [x] 9.12 Implement index JSON output: file count, chunk count, skipped count, errors list

## 10. patina query Command

- [x] 10.1 Implement `src/cli/query.rs` handler with `--limit` (default 10), `--json`, and `--explain` flags
- [x] 10.2 Implement FTS5 query in `src/query/fts.rs`: parameterise the query string, execute against the FTS5 virtual table, retrieve BM25 scores and matching chunk rows
- [x] 10.3 Implement BM25 score normalisation to `[0.0, 1.0]` using per-result-set min/max
- [x] 10.4 Implement LIKE-based fallback search in `src/query/fallback.rs` for when FTS5 is unavailable; set `mode = "lexical-fallback"` in output
- [x] 10.5 Implement scoring components in `src/query/scorer.rs`: compute `title_match_bonus`, `alias_match_bonus`, `tag_match_bonus`, `page_type_bonus`, `freshness_bonus`, `provenance_bonus` — all normalised to `[0.0, 1.0]`
- [x] 10.6 Implement weighted score combination: `fts*0.70 + title*0.10 + alias*0.07 + tag*0.05 + page_type*0.03 + freshness*0.03 + provenance*0.02`
- [x] 10.7 Sort results by descending combined score; apply `--limit`
- [x] 10.8 Implement `--explain` output: include `score_components` object and `matches` array in each result
- [x] 10.9 Implement query JSON output: `data.results` array with `path`, `score`, `excerpt` per result; `data.mode` field

## 11. patina status Command

- [x] 11.1 Implement `src/cli/status.rs` handler
- [x] 11.2 Report: Git worktree detected (yes/no), uncommitted knowledge changes (yes/no), `.patina/` ignored by Git (yes/no), index last built timestamp, scope metadata if present
- [x] 11.3 Warn if `.patina/` is not listed in `.gitignore`
- [x] 11.4 Warn if any file under `.patina/` is staged for Git commit
- [x] 11.5 Implement status JSON output with the standard envelope

## 12. patina read Command

- [x] 12.1 Implement `src/cli/read.rs` handler
- [x] 12.2 Implement path canonicalization in `src/read/path.rs`: resolve the requested path via `std::fs::canonicalize`; canonicalize the knowledge root; verify the resolved path has the root as a prefix; reject if not
- [x] 12.3 Reject path traversal with error `path traversal rejected: path resolves outside the knowledge root`
- [x] 12.4 Implement symlink policy: detect symlinks; reject external symlinks always; reject internal symlinks unless `security.allow_internal_symlinks = true`
- [x] 12.5 Return clear error if the file does not exist
- [x] 12.6 Print file contents to stdout on success; implement `--json` output with `data.content` and `data.front_matter`

## 13. patina stale Command

- [x] 13.1 Implement `src/cli/stale.rs` handler
- [x] 13.2 Implement `review_after_passed` check: compare `documents.review_after` against current date; emit reason with severity `warning`
- [x] 13.3 Implement `source_hash_changed` check: read current SHA-256 of each source file referenced in `source_refs` table; compare against `source_hash_at_index`; emit reason with severity `warning`
- [x] 13.4 Implement `missing_source_ref` check: detect referenced source files that no longer exist on disk; emit reason with severity `error`
- [x] 13.5 Implement `deprecated_but_linked` check: find pages with `status = 'deprecated'` that are linked from pages with `status = 'active'`; emit reason with severity `warning`
- [x] 13.6 Implement `draft_too_old` check: find pages with `status = 'draft'` whose `modified_at` is older than configured threshold (default 90 days); emit severity `warning`
- [x] 13.7 Implement stale JSON output: `data.stale_pages` array; each entry has `path` and `reasons` array with `code`, `severity`, `source` (optional)

## 14. patina doctor Command

- [x] 14.1 Implement `src/cli/doctor.rs` handler; run all checks in sequence; collect results
- [x] 14.2 Implement Git worktree check (ok/warning)
- [x] 14.3 Implement knowledge directory existence check (ok/error)
- [x] 14.4 Implement `knowledge/README.md` and `knowledge/AGENTS.md` existence checks (ok/warning)
- [x] 14.5 Implement `.patina/` existence and writability check (ok/warning/error)
- [x] 14.6 Implement `.patina/` Git-ignore check (ok/warning)
- [x] 14.7 Implement SQLite database existence and `PRAGMA integrity_check` (ok/error); suggest `patina index --reset` on failure
- [x] 14.8 Implement FTS5 availability check (ok/warning)
- [x] 14.9 Implement schema version check (ok/error)
- [x] 14.10 Implement file permissions check for reads and writes to `.patina/` (ok/error)
- [x] 14.11 Implement agent instruction files check (ok/warning)
- [x] 14.12 Implement `scope.yaml` validity check (ok/warning)
- [x] 14.13 Implement large-file limit configuration check (ok/warning)
- [x] 14.14 Implement doctor JSON output: `data.checks` array with `name`, `status`, `message` per check; `ok` is false if any check is `error`
- [x] 14.15 Ensure doctor never modifies files

## 15. patina install-agent Command

- [x] 15.1 Implement `src/cli/agent.rs` handler with `--force` and `--agent <name>` flags
- [x] 15.2 Write generic `knowledge/AGENTS.md` with instructions referencing `patina query`, `patina read`, `patina lint`, `patina stale` — skip if exists unless `--force`
- [x] 15.3 Implement `--agent claude-code` target: write or append Claude Code snippet to `.claude/CLAUDE.md`
- [x] 15.4 Emit error for unrecognised `--agent` values; list supported agent types
- [x] 15.5 Implement install-agent JSON output: `data.files_written` array of paths created

## 16. Test Suite

- [x] 16.1 Create fixture knowledge directory at `tests/fixtures/valid_repo/` with: 3+ valid Markdown pages (correct front matter, internal links), 1 page with missing front matter, 1 page with a broken internal link, 1 page with a missing source reference, 1 page with `review_after` set to a past date, 1 source file for hash-change testing
- [x] 16.2 Create `tests/fixtures/duplicate_alias/` fixture with two pages sharing an alias value
- [x] 16.3 Create `tests/fixtures/large_file/` fixture with a Markdown file exceeding `max_markdown_file_mb` (use a stub that the test populates programmatically)
- [x] 16.4 Write unit tests for the front matter parser: valid, no front matter, invalid YAML cases
- [x] 16.5 Write unit tests for the heading-aware chunker: single heading, multiple headings, oversized section splitting, overlap application
- [x] 16.6 Write unit test for the token estimator: verify `ceil(char_count / 4)` for specific inputs
- [x] 16.7 Write unit test for path canonicalization: valid path, traversal attempt (`../../`), absolute path outside root
- [x] 16.8 Write unit test for the scoring formula: verify weights sum to 1.0; verify normalisation edge case (single result)
- [x] 16.9 Write unit test for JSON envelope serialization: verify all six top-level fields are present and typed correctly
- [x] 16.10 Write integration test: `patina init` on a temp directory — verify directory structure created, `.gitignore` updated
- [x] 16.11 Write integration test: `patina lint --json` on valid fixture — verify `ok: true`, empty errors
- [x] 16.12 Write integration test: `patina lint --json` on fixture with missing front matter — verify `ok: false`, `missing_required_field` in errors
- [x] 16.13 Write integration test: `patina lint --json` on fixture with broken link — verify `broken_link` error
- [x] 16.14 Write integration test: `patina lint --json` on fixture with duplicate alias — verify `duplicate_alias` error
- [x] 16.15 Write integration test: `patina index --reset --json` on valid fixture — verify `ok: true`, non-zero file and chunk counts
- [x] 16.16 Write integration test: delete `.patina/`, run `patina index --full`, verify `patina query` works
- [x] 16.17 Write integration test: `patina query --json` returns results with correct envelope structure
- [x] 16.18 Write integration test: `patina stale --json` on fixture with past `review_after` — verify `review_after_passed` reason
- [x] 16.19 Write integration test: `patina doctor --json` on valid initialized repo — verify all checks ok
- [x] 16.20 Write golden output test: capture `patina init` stdout and compare against `tests/golden/init.txt`
- [x] 16.21 Write golden output test: capture `patina lint --json` on valid fixture and compare against `tests/golden/lint_clean.json`
- [x] 16.22 Write path safety test: verify `patina read knowledge/../../etc/passwd` exits non-zero and does not read the file
- [x] 16.23 Write symlink rejection test: create a symlink outside the knowledge root; verify `patina read` rejects it
- [x] 16.24 Write SQLite migration test: inject unsupported `schema_version` into `meta` table; verify Patina exits with the expected error message
- [x] 16.25 Write SQLite migration test: `patina index --reset` produces `schema_version = "1"` in meta table
- [x] 16.26 Write query ranking test: index fixture with one title-matching page and one body-only-matching page; verify title match ranks first

## 17. JSON Envelope Hardening

- [x] 17.1 Audit all `--json` command handlers; verify every handler produces the standard envelope for both success and failure paths
- [x] 17.2 Ensure `command` field matches the subcommand name in all handlers
- [x] 17.3 Ensure `warnings` and `errors` are always arrays (never null) in the envelope
- [x] 17.4 Implement `mode` field in query output for both FTS5 and fallback search modes

## 18. Binary Packaging and Distribution

- [x] 18.1 Add CI workflow (`.github/workflows/release.yml`) with matrix builds for `x86_64-unknown-linux-gnu`, `x86_64-apple-darwin`, `aarch64-apple-darwin`, `x86_64-pc-windows-msvc`
- [x] 18.2 Configure `cargo build --release` with `lto = true` and `codegen-units = 1` in `[profile.release]` for binary size optimisation
- [x] 18.3 Add `cargo install` instructions to README
- [x] 18.4 Create GitHub Release on version tag; attach compiled binaries as release assets
- [x] 18.5 Write README with: project description, Karpathy LLM Wiki attribution, quick-start (`patina init`, `patina index`, `patina query`), independence statement (not tied to Obsidian/Claude/MCP/ChromaDB)
