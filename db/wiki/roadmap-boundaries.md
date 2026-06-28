---
title: Roadmap and Boundaries
type: decision
status: active
source_refs:
  - sources/spec-000-documentation-context.md
  - sources/spec-001-base-implementation.md
  - sources/towardsai-llm-wiki-twice.md
---

# Roadmap and Boundaries

Patina should grow from a deterministic local CLI into richer extraction and
agent tooling without losing the Markdown-first source of truth.

## v0.1

Version 0.1 establishes the local CLI, Markdown validation, source references,
SQLite or FTS indexing, query and read commands, stale checks, and agent
instruction installation.

## v0.2

Version 0.2 can add extraction workflows and semantic retrieval. These features
should enrich the wiki but should not make generated state canonical.

## v0.3

Version 0.3 can add MCP integration, richer governance, and more advanced agent
workflows. These should remain layered on top of the local knowledgebase.

## Boundary

Patina should not become a hosted platform or an opaque memory database. Its
advantage is that humans and agents can inspect the same durable files.
