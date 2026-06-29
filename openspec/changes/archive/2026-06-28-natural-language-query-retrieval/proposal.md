## Why

`patina query` currently treats user input as an exact retrieval expression, which makes natural-language questions brittle even when the indexed knowledge contains the meaningful terms. For v0.4.0, query retrieval should keep Patina's deterministic lexical model while preparing natural-language input into an inspectable query plan.

## What Changes

- Normalize query text into deterministic lexical terms before retrieval.
- Treat punctuation and FTS5 syntax characters as separators rather than executable search syntax.
- Remove common English stop words, de-duplicate terms, and filter short terms except for numeric terms and an explicit allow-list.
- Build safe FTS5 expressions from normalized terms, first requiring all terms and then retrying with any-term matching when strict search returns no results.
- Update the LIKE fallback path to search normalized terms instead of the full raw sentence, with the same strict-first then relaxed retry behavior.
- Center excerpts and `--explain` matches around normalized terms.
- Use normalized terms for title, alias, and tag score component matching.
- Preserve the existing JSON envelope shape, output modes, FTS5-first model, and deterministic lexical retrieval behavior.

## Capabilities

### New Capabilities

### Modified Capabilities

- `query-command`: normalize natural-language input into deterministic lexical terms and use those terms for FTS5 retrieval, fallback retrieval, excerpts, explain matches, and query-related score components.

## Impact

- Affected code: `src/query/fts.rs`, `src/query/fallback.rs`, `src/query/scorer.rs`, `src/query/mod.rs`, and `src/cli/query.rs`.
- Affected behavior: natural-language questions should find pages that contain the same meaningful terms as equivalent keyword queries; unsupported FTS5 syntax in user input is treated as text.
- Affected output: `--explain` may report normalized terms in `matches`; no required JSON envelope shape change.
- Dependencies: no new crates; use Rust standard library and existing SQLite support.
- Tests: unit and CLI coverage for normalization, FTS5 strict/relaxed planning, fallback strict/relaxed planning, excerpt positioning, explain matches, and normalized score component matching.
