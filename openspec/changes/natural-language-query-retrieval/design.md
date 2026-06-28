## Context

`patina query` currently passes the raw user string into SQLite FTS5 `MATCH`, wraps the same raw string in one fallback `LIKE` pattern, and uses that raw string for excerpts, explain matches, and title/alias/tag score components. This works for short keyword queries but fails for natural-language questions that include stop words, punctuation, wording that does not appear verbatim in a chunk, or characters that FTS5 treats as syntax.

The v0.4.0 requirement is still deterministic lexical retrieval. This change should make natural-language input behave like a useful knowledge-base search without introducing embeddings, network calls, model-dependent rewriting, user history, or advanced FTS5 query language support.

## Goals / Non-Goals

**Goals:**

- Normalize raw query text into deterministic lexical terms shared by FTS5, fallback search, excerpts, explain matches, and query-related score components.
- Build safe FTS5 expressions from normalized terms so user punctuation is treated as text, not executable FTS5 syntax.
- Use strict all-term retrieval first and relaxed any-term retrieval only when strict retrieval returns no results.
- Make fallback search degraded but useful by matching normalized terms instead of exact raw sentences.
- Keep the JSON envelope shape, `mode` values, scoring weights, and FTS5-first retrieval model unchanged.
- Keep implementation inspectable and covered by focused unit and CLI tests.

**Non-Goals:**

- No semantic/vector retrieval.
- No model-generated query rewriting.
- No network calls or adaptive ranking.
- No support for user-authored FTS5 syntax, phrases, column filters, prefix queries, or boolean operators.
- No index schema change.
- No required output-shape change beyond normalized values in existing `matches`.

## Decisions

1. Add a shared query planning module under `src/query/`.

   The normalization rules need to apply consistently to FTS5, fallback, excerpts, explain matches, and scorer matching. A shared `QueryPlan` type can hold the raw query, trimmed raw query, normalized terms, and helper methods for strict/relaxed FTS expressions and fallback patterns. Keeping it under `src/query/` avoids spreading normalization logic across CLI, FTS, fallback, and scorer modules.

   Alternative considered: normalize independently in each module. That would be simpler locally but would quickly drift, especially around stop words, short-term filtering, and the empty-normalization fallback.

2. Represent normalized terms as ordered, de-duplicated strings.

   The source spec requires first-seen order and deterministic behavior. A `Vec<String>` plus a small `HashSet` during construction is enough. There is no need for stemming, locale-specific tokenization, or a parser dependency.

   Alternative considered: use SQLite tokenizer behavior directly. That would leave fallback, excerpts, and scorer matching with different semantics and would not protect FTS5 from raw syntax characters.

3. Quote each FTS5 term and join quoted terms with explicit `AND` or `OR`.

   Each normalized term should be treated as literal text. Quoting terms and escaping embedded quote characters prevents user input from becoming operators, column filters, phrase syntax, or prefix syntax. Since punctuation is already converted to separators before terms are built, most syntax characters never reach expression building.

   Alternative considered: pass terms as bare FTS5 tokens. That is shorter but risks operator interpretation for terms such as `AND`, `OR`, or values containing special characters if the normalization rules evolve.

4. Use strict-first retrieval for both FTS5 and fallback.

   FTS5 search will run an all-term expression first and return those results when non-empty. Only an empty strict result set triggers an any-term retry. Fallback search follows the same rule with `AND` and `OR` `LIKE` predicates. This satisfies the requirement that strict matches rank ahead of relaxed-only matches without adding a combined ranking layer in v0.4.0.

   Alternative considered: combine strict and relaxed result sets with a bonus. That gives more ranking nuance but adds duplicate handling and extra score math that the initial requirement does not need.

5. Score fallback results by matched-term count.

   Fallback `raw_score` can be the number of normalized terms found in the chunk text, with empty-normalization fallback queries scoring as one matched term when returned. The CLI can normalize fallback scores with the same existing scorer path or a simple maximum-based normalization, as long as a chunk matching more normalized terms does not score lower solely because of fallback scoring.

   Alternative considered: keep every fallback result at `1.0`. That preserves current behavior but violates the requirement that fallback ranking account for how many normalized terms matched.

6. Match title, alias, and tag bonuses against normalized terms.

   The scorer should accept normalized terms instead of the raw query string for fields whose bonus depends on query text. A field receives the binary bonus when any normalized term appears in that field case-insensitively. This preserves the current binary component model while making natural-language questions eligible for metadata bonuses.

   Alternative considered: require all normalized terms in metadata fields. That would be too strict for titles, aliases, and tags, which are often shorter than a natural-language question.

## Risks / Trade-offs

- Natural-language normalization may remove a term a user intended to search for -> Keep the initial stop-word list conservative and covered by tests.
- Relaxed any-term search may return broader results -> Run relaxed search only after strict search returns no results, preserve the result limit, and rely on term-count/FTS scoring for ordering.
- Quoted FTS terms may differ from current advanced-user expectations -> Advanced FTS syntax is explicitly out of scope for v0.4.0, and treating punctuation as text is the safer default.
- Unicode lowercasing without full Unicode tokenization may be imperfect -> Use standard-library lowercasing and the specified ASCII whitespace split after separator replacement; avoid adding a dependency until real use requires it.
- FTS5 and fallback could diverge -> Drive both paths from the same `QueryPlan` terms and add tests for both strict and relaxed behavior.

## Migration Plan

1. Add the shared query normalization and planning module with unit tests.
2. Update FTS5 search to accept a `QueryPlan`, execute strict search first, and retry relaxed search only on no results.
3. Update fallback search to accept the same plan, build dynamic all-term and any-term `LIKE` predicates, and compute deterministic matched-term scores.
4. Update excerpt creation, explain matches, and score component matching to use normalized terms.
5. Add CLI tests using natural-language questions that should retrieve the same relevant pages as equivalent keyword queries.
6. Run `cargo test -p patina query`, `cargo check`, and the full test suite.

Rollback is a code revert only. The SQLite index schema and user-authored knowledge files are unchanged.
