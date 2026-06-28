---
title: Patina Product Definition
type: project
status: active
source_refs:
  - sources/spec-000-documentation-context.md
  - sources/spec-001-base-implementation.md
---

# Patina Product Definition

Patina is a local-first knowledge substrate for agents and humans working in a
repository. It provides a structured Markdown knowledgebase, validation,
retrieval, source tracking, and agent-facing commands.

## What Patina Is

- A Git-compatible knowledge layer.
- A Markdown-first wiki with source provenance.
- A CLI for indexing, querying, reading, linting, and checking staleness.
- A way to make agent memory explicit and reviewable.

## What Patina Is Not

- Not a hosted memory service.
- Not an Obsidian-specific vault format.
- Not an MCP server as its core identity.
- Not a vector database as its source of truth.
- Not hidden chat memory.

## Product Principle

The user should be able to inspect every durable knowledge artifact in the repo.
Any generated database, cache, embedding index, or search index must be
rebuildable from the Markdown and source files.
