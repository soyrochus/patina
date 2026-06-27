## Context

Patina v0.1.0 is a net-new Rust binary crate. No prior implementation exists; all decisions are greenfield within the constraints defined by `specs/001-base-implementation.md`. The codebase must be cross-platform (macOS arm64/x64, Linux x64, Windows x64), dependency-free at runtime (no Python, Node, ChromaDB, or network), and produce a single statically-linked or minimally-linked binary via `cargo build --release`.

The knowledge directory (`knowledge/`) lives inside a Git repository and is shared across a team. The local index (`.patina/`) is machine-local, Git-ignored, and fully disposable — deleting it and running `patina index --full` must always reconstruct a working index.

## Goals / Non-Goals

**Goals:**
- Implement all nine v0.1.0 commands: `init`, `status`, `lint`, `index`, `query`, `read`, `stale`, `doctor`, `install-agent`
- Deterministic behaviour: same inputs always produce identical outputs (chunking, scoring, hashes)
- Stable JSON output envelope usable by AI agents
- SQLite FTS5 as primary retrieval engine; LIKE-based fallback if FTS5 is unavailable
- Transparent, explainable query scoring
- Path-safe file reading (no traversal outside knowledge root)
- Atomic local index writes with advisory file locking
- Comprehensive automated test suite (unit, integration, fixture, golden)

**Non-Goals:**
- MCP adapter (v0.3+)
- Semantic/vector retrieval (v0.2+)
- PDF/DOCX/PPTX extraction (v0.2+)
- Multi-root workspaces (v0.3+)
- Web UI or hosted service
- Automatic Git commits or pushes
- Full JSON Schema enforcement (v0.2+)

## Decisions

### Single binary crate with internal modules

**Decision**: One `patina` binary crate; feature areas split into internal modules (`cli`, `config`, `discovery`, `lint`, `index`, `query`, `read`, `stale`, `doctor`, `agent`, `db`, `output`).

**Rationale**: The spec explicitly requires a single binary. Internal module boundaries provide testability and separation without the overhead of a workspace with multiple library crates at this stage. A library crate (`patina-core`) can be extracted later if an MCP adapter or public API needs it.

**Alternative rejected**: Workspace with `patina-core` lib + `patina` bin — premature abstraction for v0.1.

### `clap` for CLI, `toml` for config

**Decision**: Use `clap` (derive API) for argument parsing. Config loaded from `.patina.toml` or `patina.toml` at the knowledge root; defaults applied if absent.

**Rationale**: Clap is the de-facto standard in the Rust ecosystem, supports `--json` flags cleanly, and generates help text automatically. TOML config matches the spec's config examples verbatim.

### `rusqlite` with bundled SQLite and FTS5

**Decision**: Use `rusqlite` with the `bundled` feature to ship SQLite as part of the binary. Enable `bundled-full` for FTS5.

**Rationale**: Eliminates system SQLite version variance across platforms. FTS5 availability becomes guaranteed rather than environment-dependent. The spec states FTS5 is the preferred engine and fallback exists only for resilience.

**Alternative rejected**: System SQLite — would require testing matrix across OS SQLite versions, and FTS5 may be absent on some Linux distributions.

### Heading-aware chunking with token-count estimation

**Decision**: Chunk Markdown by heading tree. Sections exceeding `chunk_size` (default 1200 estimated tokens) are split at paragraph boundaries with `chunk_overlap` (default 150) applied only to oversized sections. Token estimate: `ceil(char_count / 4)`.

**Rationale**: The spec mandates heading-aware deterministic chunking. The `/ 4` character-to-token approximation is simple, cross-platform, and stable. Any future tokenizer change would be a breaking schema version bump.

**Alternative rejected**: Fixed-size character chunks — loses semantic structure. Model-dependent tokenization — violates cross-platform determinism constraint.

### Markdown parser: `pulldown-cmark`

**Decision**: Use `pulldown-cmark` for Markdown event-stream parsing. Use `gray_matter` (or a thin YAML-first front matter splitter) for YAML front matter extraction.

**Rationale**: `pulldown-cmark` is maintained, fast, event-driven (suitable for streaming heading detection), and has no C dependencies. `comrak` is an alternative but heavier; its AST is useful if we need rendering, which v0.1 does not.

### Scoring formula is hardcoded, not runtime-configurable

**Decision**: The weighted scoring formula (`fts=0.70, title=0.10, alias=0.07, tag=0.05, page_type=0.03, freshness=0.03, provenance=0.02`) is compiled into the binary for v0.1. `--explain` exposes all components.

**Rationale**: Spec says "one stable default" for v0.1 with configurability deferred. Hardcoding avoids a config surface area that hasn't been validated in practice yet.

### Advisory file lock + SQLite WAL for concurrency

**Decision**: Acquire an advisory lock on `.patina/index.lock` before any write operation. SQLite opened in WAL mode with a busy timeout.

**Rationale**: WAL mode allows concurrent readers. The advisory lock prevents two `patina index` processes from racing; it is not a substitute for WAL but a guard against partial builds corrupting the index.

### Atomic full rebuild via tmp file + rename

**Decision**: Full index rebuilds write to `.patina/index.sqlite.tmp`, validate with `PRAGMA integrity_check`, then atomically rename to `.patina/index.sqlite`.

**Rationale**: If the rebuild crashes midway, the existing index remains intact. On Windows, rename semantics differ; use `std::fs::rename` which is documented to fail if destination is open rather than corrupt it.

### Path canonicalization for `patina read`

**Decision**: Resolve requested path via `std::fs::canonicalize`, then verify the canonical path has the canonical knowledge root as a prefix. Symlinks that resolve outside the root are rejected.

**Rationale**: Prevents path traversal attacks. The spec mandates this behaviour explicitly. Symlinks inside the root are allowed only if `security.allow_internal_symlinks = true` (default false).

### Schema versioning and migration

**Decision**: `meta` table stores `schema_version` as a plain integer string. v0.1 schema is version `1`. On startup, if `schema_version` is unrecognised, fail with a clear error suggesting `patina index --reset`.

**Rationale**: Disposable local index means destructive rebuild is acceptable for v0.1. Future incremental migrations can be layered on top of the `meta` table pattern without changing the table structure.

## Risks / Trade-offs

- **FTS5 BM25 score polarity** — SQLite FTS5 returns BM25 as a negative number (lower = better). Normalising across result sets requires finding the min/max per query. If only one result is returned, normalisation collapses to 1.0. → Normalize to range [0,1] using per-result-set min/max; document the edge case.

- **Windows atomic rename** — `std::fs::rename` on Windows fails if the destination file is open by another process (e.g., a reader holds the DB). → Ensure the old DB is closed before rename; document that the index file should not be kept open by external tools.

- **`camino` vs `std::path`** — UTF-8 path enforcement is desirable (`camino`) but adds a dependency and requires wrapping all path operations. → Use `camino::Utf8Path` internally for knowledge paths; fall back to `std::path` for OS interactions. This tradeoff is acceptable for v0.1.

- **Token estimation drift** — The `ceil(char_count / 4)` approximation produces different chunk boundaries from future model-aware tokenizers. → Treat the approximation as part of the schema contract; changing it bumps `schema_version`.

- **Large knowledge repositories** — The `max_total_markdown_files = 50000` limit is a warning, not a hard stop. FTS5 can handle this volume but query latency may increase. → Emit the warning and document recommended indexing frequency.

## Migration Plan

This is a new project; no migration from an existing system is required. The rollout sequence follows the implementation priority in the spec:

1. CLI skeleton + config loading
2. `patina init`
3. Knowledge directory discovery
4. Markdown + front matter parser
5. `patina lint`
6. SQLite schema + migrations
7. `patina index` + chunking
8. FTS5 search
9. `patina query` + scoring
10. `patina read`
11. `patina stale`
12. `patina doctor`
13. `patina install-agent`
14. JSON envelope hardening
15. Test suite (fixture + golden)
16. Binary packaging

## Open Questions

- Should `patina status` be its own subcommand or output from `patina doctor`? The spec lists both. Current decision: separate `patina status` for quick Git/index state; `patina doctor` for full environment check.
- Front matter parser library: `gray_matter` crate vs. a hand-rolled YAML fence splitter + `serde_yaml`. The `gray_matter` crate has not been updated recently; evaluate at implementation time and fall back to manual splitting + `serde_yaml` if needed.
