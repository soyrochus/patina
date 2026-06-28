---
title: Patina Wiki Index
type: index
status: active
source_refs:
  - sources/spec-000-documentation-context.md
  - sources/spec-001-base-implementation.md
  - sources/towardsai-llm-wiki-twice.md
  - sources/datasciencedojo-llm-wiki-tutorial.md
  - sources/mindstudio-llm-wiki-architecture.md
---

# Patina Wiki Index

Patina is a local-first, Git-compatible knowledge substrate for projects and
agents. It keeps Markdown as the durable knowledge artifact, uses source notes
for provenance, and treats indexes as rebuildable support infrastructure.

## Core Pages

- [[llm-wiki-pattern]] defines the architectural pattern Patina adopts.
- [[patina-product-definition]] defines what Patina is and is not.
- [[patina-v0-1-cli-contract]] captures the v0.1 command and behavior contract.
- [[local-first-architecture]] describes filesystem, Git, and index boundaries.
- [[retrieval-indexing]] explains indexing, chunking, and retrieval.
- [[provenance-staleness]] explains source references and stale detection.
- [[agent-operating-model]] explains how agents should use and update Patina.
- [[roadmap-boundaries]] records implementation phases and non-goals.

## Knowledgebase Shape

The canonical memory is the `wiki/` Markdown layer. The `sources/` layer stores
source notes derived from specs and requested articles. A local database or FTS
index can accelerate retrieval, but it should always be disposable and
rebuildable from the Markdown corpus.
