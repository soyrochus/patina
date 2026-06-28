---
title: Provenance and Staleness
type: process
status: active
source_refs:
  - sources/spec-000-documentation-context.md
  - sources/spec-001-base-implementation.md
  - sources/mindstudio-llm-wiki-architecture.md
---

# Provenance and Staleness

Patina should make knowledge provenance visible. A wiki page should show which
source notes support it, and tooling should warn when those sources are missing
or have changed.

## Source References

Use `source_refs` in YAML front matter to point at local source notes. External
URLs can be recorded in source notes, but wiki pages should prefer local
references so validation and stale checks can operate without network access.

## Stale Detection

The index should record source hashes. If a referenced source changes after a
wiki page was last reviewed, the CLI should surface a stale warning. Missing
source references should be treated as validation errors.

## Review Model

Agents may propose source note updates and wiki edits, but those changes should
be visible as normal file diffs. This keeps project memory auditable.
