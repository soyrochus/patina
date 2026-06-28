---
title: Retrieval and Indexing
type: system
status: active
source_refs:
  - sources/spec-001-base-implementation.md
  - sources/datasciencedojo-llm-wiki-tutorial.md
---

# Retrieval and Indexing

Retrieval exists to help humans and agents find the right maintained knowledge
quickly. It should point back to pages and source-backed snippets rather than
becoming a separate knowledge layer.

## Chunking

Chunks should be deterministic and heading-aware. A chunk should preserve enough
heading context to make the snippet understandable without hiding the page it
came from. Stable content hashes make stale checks and incremental indexing
possible.

## Indexing

SQLite with FTS5 is the v0.1 retrieval target. The index should store page
metadata, chunk text, paths, heading context, and source hashes. If FTS5 is not
available, the CLI should degrade predictably rather than silently changing the
contract.

## Query Results

Query results should include a path, title, page type, status, snippet, heading
context, and score. Agents need enough information to decide whether to call
`read` for more context.
