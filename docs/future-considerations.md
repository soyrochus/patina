# Future Considerations

This document collects non-blocking follow-up ideas discovered while testing
Patina with the knowledge exercise in `docs/patina-knowledge-exercise.md`.

These items do not block test-user rollout. The current implementation is good
enough for practical exercise use: natural-language queries return relevant
results, lint and stale checks pass, and the generated knowledge pages are
retrievable through the CLI.

## Retrieval Ranking

The query:

```bash
patina query "why is the CLI the stable integration boundary for Patina?" --json --explain
```

can rank `knowledge/wiki/index.md` above the canonical decision page
`knowledge/wiki/decisions/patina-cli-as-stable-agent-contract.md`.

This happens because the index page contains a compact summary with the exact
query terms, which can receive a stronger normalized FTS score than the longer
canonical page.

This is a ranking issue, not a recall issue. The canonical page is still
returned.

Possible future improvements:

- Down-rank `type: index` pages for ordinary content queries.
- Boost canonical `decision`, `concept`, and `system` pages when title or page
  type matches the query intent.
- De-duplicate or group query results by document so multiple chunks from the
  same page do not dominate the result list.
- Add an explainable "canonical page" component to query scoring.

## Wikilink Resolution

Current wikilinks work when using bare page stems such as:

```markdown
[[patina-release-workflow]]
```

Lint resolves these by searching for a matching Markdown filename under the
knowledge root.

Path-qualified wikilinks may need clearer future semantics, especially as the
knowledge tree grows into directories such as:

```text
knowledge/wiki/concepts/
knowledge/wiki/decisions/
knowledge/wiki/systems/
```

Possible future improvements:

- Define whether wikilinks are always stem-based, path-based, or both.
- Support unambiguous path-qualified wikilinks such as
  `[[wiki/systems/patina-release-workflow]]`.
- Emit a warning when a bare-stem wikilink becomes ambiguous.
- Document the preferred wikilink style in `knowledge/AGENTS.md`.

## Provenance for Conversational Knowledge

The exercise creates knowledge from a conversational prompt rather than from an
existing source note under `knowledge/sources/`.

That is acceptable for now. Pages without `source_refs` are valid when the
knowledge is authored directly and no durable source note exists.

If stronger provenance becomes important, add a source note summarizing the
exercise/session and reference it from the created pages.

Possible future improvements:

- Add `knowledge/sources/patina-release-agent-integration-exercise.md`.
- Reference that source from:
  - `knowledge/wiki/decisions/patina-cli-as-stable-agent-contract.md`
  - `knowledge/wiki/systems/patina-release-workflow.md`
  - `knowledge/wiki/concepts/agent-skills-as-thin-adapters.md`
- Add guidance for when conversationally supplied knowledge should be converted
  into a source note.

## Test-User Guidance

For test users, the current behavior is acceptable with these caveats:

- A summary/index page may occasionally rank above the canonical page.
- Missing `source_refs` are not automatically an error.
- The generated `.patina/` directory remains disposable local state and should
  not be committed or included in release artifacts.

These caveats should be presented as known limitations, not failures.
