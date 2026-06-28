---
title: Patina v0.1 CLI Contract
type: system
status: active
source_refs:
  - sources/spec-001-base-implementation.md
---

# Patina v0.1 CLI Contract

Patina v0.1 is a Rust CLI that turns a Markdown knowledgebase into a validated
and queryable project memory.

## Commands

- `init` creates the knowledgebase structure.
- `status` reports current configuration and index state.
- `lint` validates metadata, links, and source references.
- `index` builds the local SQLite and FTS index.
- `query` searches the indexed corpus.
- `read` returns page or chunk content in a stable form.
- `stale` checks whether referenced sources have changed.
- `install-agent` installs agent instructions.
- `doctor` checks environment and compatibility assumptions.

## Metadata Contract

Every page should include `title`, `type`, and `status`. The supported statuses
are `draft`, `active`, `deprecated`, and `archived`. Supported page types include
`concept`, `system`, `project`, `decision`, `person`, `process`, `glossary`,
`source`, `index`, and `open-question`.

## Output Contract

Agent-facing commands should return stable JSON envelopes. Human-readable output
can exist, but machine callers need deterministic paths, snippets, metadata,
and error structures.
