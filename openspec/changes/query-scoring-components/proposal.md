## Why

The `query-command` scoring formula is specified as seven weighted components, but `alias_match_bonus`, `tag_match_bonus`, and `freshness_bonus` are currently placeholders. This causes query results to ignore aliases, tags, and document age even when `--explain` reports those components.

## What Changes

- Compute `alias_match_bonus` from indexed page aliases using case-insensitive substring matching.
- Compute `tag_match_bonus` from indexed page tags using case-insensitive substring matching.
- Compute `freshness_bonus` as a deterministic linear decay from `1.0` to `0.0` over 365 days.
- Persist `aliases` and `tags` as JSON-encoded TEXT columns on indexed documents.
- **BREAKING**: Bump the SQLite index schema version from `"1"` to `"2"`; users with an existing v1 index must run `patina index --reset`.
- Preserve the existing CLI flags, query JSON envelope shape, scoring weights, and fallback behavior.

## Capabilities

### New Capabilities

### Modified Capabilities

- `query-command`: complete the alias, tag, and freshness scoring requirements for `patina query` and `--explain`.
- `sqlite-index`: add indexed document metadata for aliases and tags, and bump the supported index schema version to `"2"`.

## Impact

- Affected code: `src/db/schema.rs`, `src/db/documents.rs`, `src/cli/index.rs`, `src/query/fts.rs`, `src/query/fallback.rs`, `src/cli/query.rs`, and `src/query/scorer.rs`.
- Affected data: `.patina/index.sqlite` schema changes from version 1 to version 2; no migration is required because the index is disposable.
- Dependencies: no new crates; existing `serde_json` and `chrono` are sufficient.
- Tests: scorer unit tests plus schema-version and query explanation coverage for the new components.
