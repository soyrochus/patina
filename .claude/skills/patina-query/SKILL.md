---
name: patina-query
description: Search the Patina knowledge base and read the most relevant pages. Use when answering questions about project context, architecture, decisions, domain knowledge, prior repository knowledge, or anything that may already be documented under the knowledge directory.
license: MIT
compatibility: Requires the patina CLI on PATH and a Patina knowledge directory in this repository.
---

<!-- PATINA GENERATED SKILL -->

# Patina Query

Use this skill when the user asks about project knowledge, architecture, decisions, domain concepts, previous notes, repository context, or anything likely to be documented in the Patina knowledge base.

The shared Patina operating instructions are in:

```text
knowledge/AGENTS.md
```

## Workflow

1. Convert the user's request into a concise search query.

2. Run:

   ```bash
   patina query "<terms>" --json --limit 5
   ```

3. Inspect the JSON response.

   - If `ok` is `false`, report the errors.
   - If no results are returned, try one broader query.
   - If results are returned, read the highest-scoring pages.

4. For each relevant result, run:

   ```bash
   patina read <path> --json
   ```

5. Answer from the Git-tracked Markdown content returned by `patina read`.

6. Cite repository-relative page paths in the answer so the user can inspect the source.

## Rules

- Do not answer from `.patina/` generated index files.
- Do not cite generated local cache data.
- Prefer reading actual Markdown pages before answering.
- Use `patina query "<terms>" --json --limit 5 --explain` if ranking looks unexpected and the installed Patina version supports `--explain`.
- If the knowledge base does not contain the answer, say so clearly.
