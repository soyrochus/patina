## 1. Schema And Document Metadata

- [ ] 1.1 Add nullable `aliases TEXT` and `tags TEXT` columns to the `documents` table DDL in `src/db/schema.rs`
- [ ] 1.2 Bump `SCHEMA_VERSION` from `"1"` to `"2"` and keep unsupported-version errors pointing users to `patina index --reset`
- [ ] 1.3 Add `aliases` and `tags` fields to `DocumentRecord` in `src/db/documents.rs`
- [ ] 1.4 Update the document upsert SQL, conflict update clause, and positional parameters to persist `aliases` and `tags`

## 2. Index-Time Front Matter Capture

- [ ] 2.1 Add a helper in `src/cli/index.rs` that serializes YAML string sequences to JSON array strings
- [ ] 2.2 Populate `DocumentRecord.aliases` from front matter `aliases` and `DocumentRecord.tags` from front matter `tags`
- [ ] 2.3 Ensure absent fields or sequences with no string values store SQL NULL rather than `[]`

## 3. Query Result Metadata

- [ ] 3.1 Extend `RawResult` in `src/query/fts.rs` with `aliases`, `tags`, and `modified_at`
- [ ] 3.2 Update the FTS5 SELECT and row mapping in `src/query/fts.rs` to return aliases, tags, and modified_at
- [ ] 3.3 Update the fallback SELECT and row mapping in `src/query/fallback.rs` to return aliases, tags, and modified_at
- [ ] 3.4 Compute a single `Utc::now()` timestamp per query run in `src/cli/query.rs` and pass it with raw metadata to the scorer

## 4. Scoring Components

- [ ] 4.1 Update `score_components()` in `src/query/scorer.rs` to accept aliases JSON, tags JSON, modified_at, and `now: DateTime<Utc>`
- [ ] 4.2 Implement `alias_match_bonus` as case-insensitive substring matching over parsed alias strings
- [ ] 4.3 Implement `tag_match_bonus` as case-insensitive substring matching over parsed tag strings
- [ ] 4.4 Implement `freshness_bonus` as linear decay from `1.0` to `0.0` over 365 days, with missing or invalid timestamps scoring `0.0`
- [ ] 4.5 Preserve the existing component weights and clamp normalized FTS values to `[0.0, 1.0]`

## 5. Tests And Verification

- [ ] 5.1 Add scorer unit tests for alias substring match and alias no-match behavior
- [ ] 5.2 Add scorer unit tests for tag match behavior
- [ ] 5.3 Add scorer unit tests for freshness today, freshness older than 365 days, and missing modified_at
- [ ] 5.4 Update schema-version tests and expectations from `"1"` to `"2"`
- [ ] 5.5 Add or update coverage proving schema version `"1"` is rejected with the reset guidance
- [ ] 5.6 Run `cargo test -p patina scorer`
- [ ] 5.7 Run `cargo check`
- [ ] 5.8 Run `cargo test`
