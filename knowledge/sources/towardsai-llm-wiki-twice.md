---
title: Towards AI - I Built Karpathy's LLM Wiki Twice
type: source
status: active
url: https://pub.towardsai.net/i-built-karpathys-llm-wiki-twice-once-as-code-once-as-a-md-heres-what-each-one-gives-up-08b31170999a
accessed: 2026-06-28
---

# Towards AI - I Built Karpathy's LLM Wiki Twice

This source note records the article requested by the user:
<https://pub.towardsai.net/i-built-karpathys-llm-wiki-twice-once-as-code-once-as-a-md-heres-what-each-one-gives-up-08b31170999a>.

## Relevant Takeaways

- The article frames the LLM Wiki idea through an implementation tradeoff:
  structured code or database-backed systems can offer automation and stronger
  mechanics, while plain Markdown offers simpler inspection, editing, and Git
  review.
- Patina should preserve Markdown as the canonical knowledge layer even when it
  builds indexes or helper commands around that layer.
- Generated or computed state should support the wiki rather than replace it.

## Design Implication

Use local Markdown pages as the durable artifact and treat any database or index
as a rebuildable acceleration layer.
