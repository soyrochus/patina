# Patina - Natural-Language Query Retrieval

## 1. Status

Pending implementation for v0.4.0.

This specification defines a fix for brittle `patina query` behaviour when users
enter natural-language questions instead of short keyword phrases.

It does not replace the FTS5-first retrieval model from
`specs/001-base-implementation.md`. It refines how user query text is prepared
for both the FTS5 path and the LIKE-based fallback path.

## 2. Problem

`patina query` currently treats the user's input as an exact retrieval
expression.

In the FTS5 path, the raw user input is passed directly to SQLite `MATCH`.

In the fallback path, the raw user input is wrapped as one `%query%` pattern and
matched with `LIKE`.

This works for concise keyword queries such as:

```bash
patina query "agents durable context"
```

It is brittle for natural-language questions such as:

```bash
patina query "why should agents use Patina as durable context"
```

The natural-language form may contain stop words, punctuation, wording that does
not appear verbatim in the indexed chunk, or FTS5 syntax characters. A page can
contain the relevant durable terms and still be missed because the query was not
normalized before retrieval.

The fix is not semantic retrieval. The v0.1 retrieval model remains deterministic
lexical search. The fix is to turn natural-language input into a deterministic
lexical query plan that behaves like users expect from a knowledge-base search.

## 3. Goals

- Natural-language questions should find the same relevant pages as equivalent
  keyword queries when they share meaningful terms.
- FTS5 search should remain the preferred retrieval engine.
- LIKE-based fallback search should be degraded but useful, not exact-phrase
  only.
- Query handling must be deterministic, inspectable, and testable.
- Query handling must avoid treating user punctuation as executable FTS5 query
  syntax unless Patina explicitly supports advanced query syntax later.
- Existing short keyword queries must continue to work.

## 4. Non-goals

- No embeddings or semantic vector search.
- No model-dependent query rewriting.
- No network calls.
- No user-specific search history or adaptive ranking.
- No advanced FTS5 query language support in this change.
- No change to the JSON envelope shape except for optional additional
  explanation fields described below.

## 5. Query normalization

Patina shall normalize user query text into meaningful lexical terms before
running retrieval.

The normalization algorithm shall be deterministic and shall apply equally to
FTS5 search, fallback search, highlighting, and score component matching unless
a section below states otherwise.

Required steps:

1. Lowercase the input using Unicode-aware lowercasing where available in the
   standard library.
2. Treat punctuation and FTS5 syntax characters as separators.
3. Split on ASCII whitespace after separator replacement.
4. Trim empty terms.
5. Remove stop words.
6. Remove duplicate terms while preserving first-seen order.
7. Keep terms shorter than three characters only when they contain a digit or
   are present in an explicit allow-list.

The initial English stop-word list shall include at least:

```text
a
an
and
are
as
at
be
but
by
can
does
for
from
how
if
in
is
it
of
on
or
should
that
the
this
to
use
what
when
where
who
why
with
```

The initial short-term allow-list shall include:

```text
ai
cli
db
fts
ui
v0
v1
v2
v3
```

If normalization produces no terms, Patina shall fall back to the trimmed raw
query as a single escaped term. If the trimmed raw query is empty, Patina shall
return an empty result list with `ok = true`.

## 6. FTS5 query planning

For FTS5 search, Patina shall build a safe FTS5 expression from normalized
terms, not from the raw user string.

Each normalized term shall be escaped or quoted so that user input cannot be
interpreted as FTS5 operators, column filters, phrase syntax, or prefix syntax
unless Patina intentionally adds such a feature later.

The default FTS5 expression shall require all normalized terms:

```text
term1 AND term2 AND term3
```

If this strict all-term search returns no results, Patina shall retry with a
relaxed any-term expression:

```text
term1 OR term2 OR term3
```

The retry is part of one `patina query` invocation. It must not change the
reported mode from `fts5`.

Result ranking shall still use SQLite BM25 for FTS5 results. Results from the
strict all-term query should be preferred over results found only by the relaxed
any-term query. The implementation may do this by:

- running strict search first and returning it when non-empty; or
- combining both result sets with an explicit strict-match bonus.

The simpler strict-first approach is acceptable for the initial implementation.

## 7. LIKE fallback query planning

When FTS5 is unavailable, Patina shall use the same normalized terms for
LIKE-based fallback search.

The fallback query shall not require the full raw user sentence to appear as a
single substring.

The fallback path shall first search for chunks containing all normalized terms:

```sql
lower(c.text) LIKE lower(?1)
AND lower(c.text) LIKE lower(?2)
AND ...
```

where each parameter is a `%term%` pattern.

If the all-term fallback returns no results, Patina shall retry with an any-term
fallback:

```sql
lower(c.text) LIKE lower(?1)
OR lower(c.text) LIKE lower(?2)
OR ...
```

Fallback results shall receive a deterministic raw lexical score based on the
number of normalized terms matched in the chunk. Exact details may be simple in
the initial implementation, but a chunk matching more terms must not score lower
than a chunk matching fewer terms solely because of fallback scoring.

The JSON output mode shall remain:

```json
"mode": "lexical-fallback"
```

when FTS5 is unavailable.

## 8. Excerpts and matches

Excerpts should be centered around the first normalized term found in the chunk.

If no normalized term is found, the excerpt may start at the beginning of the
chunk.

When `--explain` is provided, the `matches` array should contain the normalized
terms used for retrieval, not the unprocessed raw query string.

Example:

```json
"matches": ["agents", "patina", "durable", "context"]
```

## 9. Score components

Title, alias, and tag matching shall use normalized terms instead of requiring
the full raw query string to appear as a substring.

A title, alias, or tag match shall be considered present when any normalized
term appears in the candidate field.

For example, this query:

```bash
patina query "why is controlled autonomy important"
```

shall be eligible for an alias bonus if a page has this alias:

```yaml
aliases:
  - controlled autonomy
```

The exact component value may remain binary for now:

```text
1.0 if any normalized term matches
0.0 otherwise
```

Future specs may define proportional component scoring.

## 10. User-visible warnings

Patina shall continue to warn when FTS5 is unavailable and fallback mode is used.

Patina should not warn merely because a natural-language query was normalized.
Normalization is expected behaviour.

`patina doctor` may include more detailed FTS5 diagnostic information, such as
SQLite version and FTS5 availability, but this is optional. The core fix is query
planning, not doctor output.

## 11. Compatibility

Existing commands remain valid:

```bash
patina query "controlled autonomy"
patina query "agents" --json
patina query "durable context" --json --explain
```

The output envelope remains compatible:

```json
{
  "version": "0.1",
  "command": "query",
  "ok": true,
  "data": {
    "mode": "fts5",
    "results": []
  },
  "warnings": [],
  "errors": []
}
```

The contents of `data.results[*].matches` under `--explain` may change from the
raw query string to normalized terms as specified above.

## 12. Acceptance criteria

Given a knowledge page containing:

```markdown
# Agent Operating Model

Agents should use Patina as durable context, not as an opaque memory store.
```

After indexing, both commands shall return that page:

```bash
patina query "agents durable context" --json
patina query "why should agents use Patina as durable context" --json
```

The second command shall not require the exact sentence to appear in the page.

Given a page with the title:

```text
Patina CLI as Stable Agent Contract
```

and body text containing:

```text
The Patina CLI is the stable integration boundary between Patina internals and
external agents or tooling.
```

both commands shall return the page:

```bash
patina query "CLI stable integration boundary" --json
patina query "why is the CLI the stable integration boundary" --json
```

When FTS5 is unavailable, the same natural-language queries shall use
`lexical-fallback` mode and still return pages matching the meaningful terms.

## 13. Test requirements

The implementation shall include tests for:

- query normalization removes stop words and punctuation;
- query normalization preserves important short terms such as `CLI`;
- FTS5 search uses normalized terms rather than raw natural-language input;
- FTS5 strict all-term search is attempted before relaxed any-term search;
- fallback search matches terms independently rather than matching the full raw
  sentence;
- fallback results rank chunks with more matched terms at least as high as
  chunks with fewer matched terms;
- `--explain` reports normalized `matches`;
- title, alias, and tag bonuses use normalized terms;
- empty or stop-word-only queries return `ok = true` and an empty result list,
  or a documented single escaped raw-term fallback when applicable;
- existing short keyword query tests continue to pass.

## 14. Implementation notes

A small shared query module is recommended so FTS5 search, fallback search,
excerpt generation, and scorer components all use the same normalized query
representation.

Suggested internal shape:

```rust
struct PreparedQuery {
    raw: String,
    terms: Vec<String>,
}
```

The prepared query object should be created once in the CLI query path and
passed into retrieval and scoring functions.

The implementation should keep the first version intentionally simple. The goal
is robust lexical retrieval for common natural-language questions, not a full
search language.
