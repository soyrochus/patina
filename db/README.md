---
title: Patina Knowledgebase
type: index
status: active
source_refs:
  - sources/spec-000-documentation-context.md
  - sources/spec-001-base-implementation.md
---

# Patina Knowledgebase

This directory is a local-first knowledgebase for Patina. It follows the LLM
Wiki pattern: durable source notes live in `sources/`, curated synthesis pages
live in `wiki/`, and both are intended to be readable, reviewable, and tracked
with Git.

Start at [[index]].

## Layout

- `sources/` contains source notes for specs, articles, and other materials.
- `wiki/` contains maintained knowledge pages with cross-links and provenance.

## Maintenance Rules

- Keep source notes close to the original material and avoid speculative claims.
- Keep wiki pages concise, linked, and grounded in `source_refs`.
- Prefer updating existing pages before adding overlapping pages.
- Treat generated indexes under `.patina/` as disposable.
