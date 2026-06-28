---
title: Agent Operating Model
type: process
status: active
source_refs:
  - sources/spec-000-documentation-context.md
  - sources/spec-001-base-implementation.md
  - sources/datasciencedojo-llm-wiki-tutorial.md
---

# Agent Operating Model

Agents should use Patina as durable context, not as an opaque memory store.

## Read Flow

1. Run `query` to find relevant pages.
2. Run `read` on the strongest results.
3. Inspect source notes when provenance matters.
4. Use page links to navigate related context.

## Write Flow

1. Add or update source notes when new source material appears.
2. Update existing wiki pages when the knowledge belongs to an existing topic.
3. Add a new page only when it introduces a distinct durable concept.
4. Run `lint`, `index`, and `stale` before considering the knowledgebase clean.

## Agent Instructions

The `install-agent` command should place concise instructions where local agents
can find them. Those instructions should emphasize local files, source
references, reviewable edits, and generated index boundaries.
