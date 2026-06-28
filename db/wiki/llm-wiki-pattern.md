---
title: LLM Wiki Pattern
type: concept
status: active
source_refs:
  - sources/spec-000-documentation-context.md
  - sources/towardsai-llm-wiki-twice.md
  - sources/datasciencedojo-llm-wiki-tutorial.md
  - sources/mindstudio-llm-wiki-architecture.md
---

# LLM Wiki Pattern

An LLM Wiki is a durable knowledgebase maintained with help from an LLM or
agent. The key shift is from transient chat memory to inspectable project
memory: sources are captured, pages are synthesized, cross-links are maintained,
and future agents can retrieve the same knowledge repeatedly.

## Layers

- Raw or summarized sources establish provenance.
- Curated Markdown pages synthesize and organize the knowledge.
- Retrieval indexes help agents find relevant pages quickly.
- Agent instructions define how updates should be made and reviewed.

## Patina Interpretation

Patina keeps the Markdown wiki as the canonical artifact. The database or index
is useful for search, ranking, and snippets, but it is not the source of truth.
This preserves the main advantage of a Markdown wiki: it is readable in ordinary
tools, diffable in Git, and easy for humans to review.

## Tradeoff

A code-first or database-first implementation can provide stronger mechanics and
automation. A Markdown-first implementation gives up some control in exchange
for portability, auditability, and low-friction editing. Patina should combine
these by keeping Markdown canonical and making generated indexes disposable.
