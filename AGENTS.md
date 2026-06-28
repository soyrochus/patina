<!-- BEGIN PATINA CODEX CONTEXT -->

## Patina Knowledge Base

This repository uses Patina for local-first Markdown knowledge management.

Real Patina Agent Skills are installed under:

```text
.agents/skills/
```

Use the `patina-query` skill when answering questions about project context, architecture, decisions, domain knowledge, or prior repository knowledge.

Use the `patina-check` skill when validating or editing knowledge files.

Shared operating instructions:

```text
knowledge/AGENTS.md
```

Core commands:

```bash
patina query "<terms>" --json --limit 5
patina read <path> --json
patina lint --json
patina stale --json
```

<!-- END PATINA CODEX CONTEXT -->
