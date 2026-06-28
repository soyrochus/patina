---
title: Spec 000 - Non-Spec Documentation Base
type: source
status: active
original_path: specs/000-non-spec-documentaion-base.md
---

# Spec 000 - Non-Spec Documentation Base

This source note summarizes the local file
`specs/000-non-spec-documentaion-base.md`.

## Core Points

- Patina is inspired by Karpathy's LLM Wiki pattern: raw sources plus an
  LLM-maintained Markdown wiki that becomes a durable artifact.
- Patina is not Obsidian, Claude, MCP, or ChromaDB. It is a local-first,
  Git-compatible knowledge substrate.
- The repository should make knowledge inspectable, diffable, and reviewable.
- Git is the durable source of truth. Local indexes are disposable.
- Retrieval should return JSON with paths, snippets, and provenance.
- Writes by agents should be explicit and reviewable, not hidden chat memory.

## Design Implication

The knowledgebase should store curated Markdown pages alongside local source
notes and should avoid depending on a hosted service or opaque database as the
canonical memory.
