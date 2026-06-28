---
title: Spec 001 - Base Implementation
type: source
status: active
original_path: specs/001-base-implementation.md
---

# Spec 001 - Base Implementation

This source note summarizes the local file `specs/001-base-implementation.md`.

## Core Points

- Version 0.1 is a Rust CLI with commands for `init`, `status`, `lint`,
  `index`, `query`, `read`, `stale`, `install-agent`, and `doctor`.
- Markdown discovery, YAML front matter parsing, Git-aware walking, SQLite
  indexing, FTS5 retrieval, and heading-aware chunking are core behavior.
- Pages require `title`, `type`, and `status`.
- Supported statuses are `draft`, `active`, `deprecated`, and `archived`.
- Supported page types include `concept`, `system`, `project`, `decision`,
  `person`, `process`, `glossary`, `source`, `index`, and `open-question`.
- Chunking must be deterministic and heading-aware, with stable hashes.
- Retrieval should use a stable JSON envelope.
- Source references and stale source detection are part of the validation model.
- `scope.yaml` is optional but should support deterministic scope and
  confidentiality checks.
- Multiple roots should be guarded against accidental indexing.

## Design Implication

The wiki should use valid page metadata, local `source_refs`, and short,
heading-oriented sections that can be indexed and retrieved predictably.
