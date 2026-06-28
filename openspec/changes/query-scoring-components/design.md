## Context

`patina query` already combines seven normalized score components and exposes them through `--explain`, but three components are placeholders: aliases always score `0.0`, tags always score `0.0`, and freshness always scores `0.5`. The missing alias and tag data is not currently persisted to the SQLite index, while `modified_at` is already stored but not fetched by the query path.

The index is explicitly disposable, so this change can bump the SQLite schema version and require users to rebuild with `patina index --reset` instead of adding a migration.

## Goals / Non-Goals

**Goals:**

- Persist front matter `aliases` and `tags` on each indexed document as JSON-encoded arrays.
- Fetch aliases, tags, and `modified_at` for FTS5 and fallback query results.
- Compute alias, tag, and freshness score components deterministically.
- Keep the scoring weights, output envelope, CLI flags, and fallback mode unchanged.
- Reject stale v1 indexes through the existing schema-version validation path.

**Non-Goals:**

- No migration from schema version 1 to 2.
- No changes to ranking weights or BM25 normalization.
- No new front matter validation rules for aliases or tags.
- No new CLI flags or output fields beyond corrected `score_components` values.

## Decisions

1. Store `aliases` and `tags` as nullable JSON TEXT columns on `documents`.

   SQLite has no native array type, and `serde_json` is already available. Storing `NULL` for absent fields keeps missing metadata cheap and distinct from an explicit empty list. Alternatives considered were separate join tables and comma-delimited strings. Join tables add unnecessary complexity for query-time scoring, and comma-delimited strings are ambiguous for values containing punctuation.

2. Bump `SCHEMA_VERSION` from `"1"` to `"2"` without a migration.

   Patina treats `.patina/index.sqlite` as a disposable local cache. Requiring `patina index --reset` is simpler and avoids silent wrong scores from an index lacking aliases and tags. The existing unsupported-version error already gives the correct recovery path.

3. Pass one `now: DateTime<Utc>` from `src/cli/query.rs` into the scorer.

   Computing `now` once gives every result in a query run the same freshness reference and keeps scorer unit tests deterministic. Calling `Utc::now()` inside the scorer would make tests and result ordering harder to reason about.

4. Use case-insensitive substring matching for alias and tag bonuses.

   This matches the source spec and supports multi-word aliases where the query may be a significant phrase or term within a larger alias. Exact matching was considered but would miss useful alias matches such as query `autonomy` against alias `controlled autonomy`.

5. Use linear freshness decay over 365 days.

   A document modified today receives `1.0`, a document 365 or more days old receives `0.0`, and missing or unparsable timestamps receive `0.0`. Negative ages are clamped to zero days so future timestamps do not exceed `1.0`.

## Risks / Trade-offs

- Existing v1 index fails after upgrade -> The error should clearly suggest `patina index --reset`.
- Malformed JSON in stored aliases or tags loses the component bonus -> Treat parse failure as `0.0`, matching conservative behavior for absent metadata.
- Fixture databases or tests may assume schema version 1 -> Update tests and fixture setup to expect schema version 2.
- Query paths may diverge between FTS5 and fallback -> Add the same selected metadata columns to both paths and keep `RawResult` as the shared contract.

## Migration Plan

1. Update schema DDL and `SCHEMA_VERSION` to `"2"`.
2. Update document indexing to serialize `aliases` and `tags` from YAML sequences.
3. Update FTS5 and fallback query SELECTs to return aliases, tags, and `modified_at`.
4. Update the scorer API and call site to pass metadata plus a single query timestamp.
5. Add unit tests for alias, tag, and freshness components.
6. Update schema-version tests to expect version `"2"` and verify v1 indexes are rejected.

Rollback is to revert the code and rebuild the local index. No user-authored knowledge files are modified by this change.
