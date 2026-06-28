---
name: patina-check
description: Validate the Patina knowledge base before or after editing. Use when asked to audit knowledge health, check stale pages, validate metadata, or before changing files under the knowledge directory.
license: MIT
compatibility: Requires the patina CLI on PATH and a Patina knowledge directory in this repository.
---

<!-- PATINA GENERATED SKILL -->

# Patina Check

Use this skill before editing Patina knowledge files, after editing them, or when asked to audit the health of the knowledge base.

The shared Patina operating instructions are in:

```text
knowledge/AGENTS.md
```

## Workflow

1. Run:

   ```bash
   patina lint --json
   ```

2. Inspect the JSON response.

   - If `ok` is `false`, report each error with its `code`, `message`, and `path`.
   - Report warnings separately.
   - Do not proceed with unrelated knowledge edits while lint errors are present.

3. Run:

   ```bash
   patina stale --json
   ```

4. Inspect `data.stale_pages`.

   For each stale page, report:

   - page path;
   - reason code;
   - severity;
   - related source path if present.

5. If knowledge files were changed during the task, run:

   ```bash
   patina lint --json
   patina index
   ```

6. Summarise the result as one of:

   - clean;
   - warnings only;
   - errors found;
   - stale pages require review.

## Rules

- Do not ignore lint errors.
- Do not treat stale pages as necessarily wrong; they require review.
- Do not rewrite pages unless the user explicitly asks for fixes.
- Keep changes small and reviewable.
