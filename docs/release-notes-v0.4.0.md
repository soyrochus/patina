# Release Notes - v0.4.0

Patina v0.4.0 improves query behavior for real agent and human usage.

## Highlights

- Added natural-language query normalization for `patina query`.
  Questions such as `why is the CLI the stable integration boundary for Patina?`
  are normalized into meaningful lexical terms before search.
- FTS5 queries now use quoted, generated expressions instead of passing raw user
  text directly to SQLite `MATCH`.
- FTS5 search first tries all normalized terms, then falls back to any-term
  matching when strict search returns no results.
- LIKE-based fallback search now matches normalized terms independently instead
  of requiring the full raw sentence as one substring.
- `--explain` now reports the normalized search terms in `matches`.
- Title, alias, and tag score bonuses now use normalized terms.

## Compatibility

- No database schema migration is required for v0.4.0.
- Existing short keyword queries continue to work.
- Generated Patina skills remain thin wrappers over the CLI. The v0.4.0 skill
  spec allows agents to pass either concise terms or natural-language questions
  to `patina query`.

## Known Limitations

- Index pages can occasionally rank above canonical concept, system, or decision
  pages when they contain a compact summary with the exact query terms.
- Wikilinks are still primarily stem-based; path-qualified wikilink semantics
  may need future refinement.
- Pages created from conversational context may omit `source_refs`; add source
  notes later if stronger provenance is needed.
