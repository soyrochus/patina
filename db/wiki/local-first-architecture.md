---
title: Local-First Architecture
type: system
status: active
source_refs:
  - sources/spec-000-documentation-context.md
  - sources/spec-001-base-implementation.md
  - sources/towardsai-llm-wiki-twice.md
---

# Local-First Architecture

Patina's durable state lives in ordinary files. Git tracks the source notes,
curated wiki pages, scope configuration, and agent instructions. Generated
indexes belong outside the canonical knowledge layer.

## Canonical Files

- `sources/` stores source notes and local summaries.
- `wiki/` stores synthesized pages.
- `scope.yaml` stores local policy and validation scope.
- `AGENTS.md` stores operating instructions for agents.

## Disposable Files

The `.patina/` directory may contain SQLite indexes, FTS tables, caches, source
hashes, or other generated state. These files are implementation details and can
be rebuilt from the canonical Markdown files.

## Design Boundary

The system should work without network access after sources have been captured.
Remote services may help create or update pages, but the resulting knowledgebase
must remain useful as a local repo artifact.
