## 1. Query Normalization And Planning

- [x] 1.1 Add a shared query planning module under `src/query/` with a `QueryPlan` type containing the raw query, trimmed raw query, normalized terms, and empty-query state
- [x] 1.2 Implement deterministic normalization: Unicode-aware lowercasing, punctuation/FTS syntax separator replacement, ASCII whitespace splitting, stop-word removal, duplicate removal preserving first-seen order, and short-term filtering
- [x] 1.3 Add the required initial stop-word list and short-term allow-list with unit coverage for representative terms
- [x] 1.4 Implement the no-normalized-terms fallback to the trimmed raw query as one escaped term, and return an empty successful result set for empty trimmed queries
- [x] 1.5 Add helpers for strict and relaxed FTS5 expressions that quote or escape each normalized term
- [x] 1.6 Add helpers for fallback `%term%` patterns and term matching against chunk text

## 2. FTS5 Retrieval

- [x] 2.1 Update `src/query/fts.rs` to accept a `QueryPlan` instead of a raw query string
- [x] 2.2 Execute a strict all-term FTS5 search first and return those results when non-empty
- [x] 2.3 Retry with a relaxed any-term FTS5 search only when strict search returns no results
- [x] 2.4 Preserve `data.mode = "fts5"` for relaxed retry results
- [x] 2.5 Update FTS5 excerpts to center around the first normalized term found in the chunk, falling back to the start of the chunk when no term is found

## 3. LIKE Fallback Retrieval

- [x] 3.1 Update `src/query/fallback.rs` to accept the shared `QueryPlan`
- [x] 3.2 Build all-term fallback SQL using one `LIKE` predicate per normalized term joined by `AND`
- [x] 3.3 Retry fallback SQL with predicates joined by `OR` only when all-term fallback returns no results
- [x] 3.4 Compute deterministic fallback raw scores from the number of normalized terms matched in each chunk
- [x] 3.5 Update fallback excerpts to use the same normalized-term excerpt behavior as FTS5

## 4. CLI Integration And Scoring

- [x] 4.1 Build a `QueryPlan` once in `src/cli/query.rs` before selecting FTS5 or fallback search
- [x] 4.2 Return an empty successful query response immediately when the plan represents an empty trimmed query
- [x] 4.3 Pass normalized terms into score component calculation instead of the raw query string
- [x] 4.4 Update title, alias, and tag bonuses to use case-insensitive any-term matching over normalized terms
- [x] 4.5 Return normalized terms in `matches` when `--json --explain` is used
- [x] 4.6 Normalize fallback raw scores so chunks matching more normalized terms do not score lower solely because of fallback scoring

## 5. Tests And Verification

- [x] 5.1 Add unit tests for normalization, stop words, short-term filtering, duplicate removal, punctuation/FTS syntax handling, fallback-to-trimmed-term behavior, and empty-query behavior
- [x] 5.2 Add unit tests for strict and relaxed FTS5 expression generation with escaped or quoted terms
- [x] 5.3 Add unit or integration coverage proving FTS5 strict search is tried before relaxed search and relaxed results keep mode `"fts5"`
- [x] 5.4 Add unit or integration coverage proving fallback search matches normalized natural-language terms instead of the raw sentence
- [x] 5.5 Add scorer tests proving title, alias, and tag bonuses match any normalized term
- [x] 5.6 Add CLI JSON explain coverage proving `matches` contains normalized terms
- [x] 5.7 Run `cargo test -p patina query`
- [x] 5.8 Run `cargo check`
- [x] 5.9 Run `cargo test`
