# Patina — Documentation Context Amendment

## Conceptual Background

Patina is inspired by Andrej Karpathy’s “LLM Wiki” concept: a file-based knowledge system where raw source material remains available, while an LLM helps maintain a structured, interlinked Markdown wiki over time.

The important idea is not a specific note-taking application or a particular AI vendor. The important idea is that knowledge should become a persistent, inspectable artifact rather than a temporary chat response.

In the LLM Wiki pattern, the knowledge base is maintained as ordinary files. New information can be added as source material. The wiki is then updated incrementally: pages are created, revised, cross-linked, and checked for contradictions. Agent instruction files define how AI tools should work with the repository.

Patina takes this idea and makes it more explicit as an engineering tool.

Patina does not assume Obsidian.

Patina does not assume Claude Code.

Patina does not assume a hosted vector database.

Patina does not assume MCP.

Instead, Patina defines a local-first, Git-compatible knowledge substrate:

```text
knowledge/   shared Markdown knowledge, reviewed through Git
.patina/     local generated index and cache, ignored by Git
patina       deterministic Rust CLI for init, lint, index, query, read, stale, and agent setup
```

The goal is to make the knowledge base usable by both humans and AI coding agents while preserving transparency, reviewability, and ownership.

## Why Patina Exists

Most AI-assisted knowledge workflows are ephemeral. A user asks a question, the model retrieves or guesses context, produces an answer, and the useful synthesis often disappears into chat history.

Patina is based on a different assumption: valuable synthesis should accumulate.

If a team repeatedly explains the same architecture, decisions, constraints, terminology, and project history to AI tools, that context should become a durable artifact. It should live in the repository, evolve through normal review, and remain readable without any special application.

Patina therefore treats the knowledge base as part of the working system.

The repository contains the durable knowledge.
The local index accelerates retrieval.
The CLI makes the workflow deterministic.
Agent instructions teach AI tools how to use the system safely.
Git remains the sharing and governance mechanism.

## Relationship to the LLM Wiki Pattern

Patina follows the spirit of the LLM Wiki pattern:

```text
raw sources remain available
synthesis is stored as Markdown
knowledge evolves incrementally
cross-links matter
contradictions should be surfaced
agents need explicit operating instructions
```

Patina adds engineering constraints for team and software-development use:

```text
Git is the source of truth
local indexes are disposable
all retrieval must expose file paths and snippets
write operations must be explicit
agent changes must be reviewable as diffs
the system must work across macOS, Windows, and Linux
the core tool must not depend on a specific editor, agent, or cloud service
```

This makes Patina less of a personal “second brain” product and more of a repository-native knowledge tool.

## Design Position

Patina should be understood as:

```text
a local-first Markdown knowledge manager
a deterministic retrieval and validation CLI
a Git-compatible knowledge substrate
an agent-readable project memory
```

Patina should not be understood as:

```text
a note-taking app
an Obsidian clone
a vector database
a chat system
an autonomous agent
a hosted knowledge platform
```

## Documentation References

The README should reference Karpathy’s LLM Wiki concept as prior art and inspiration.

Suggested README wording:

> Patina is inspired by Andrej Karpathy’s LLM Wiki pattern: a persistent Markdown knowledge base maintained with the help of LLM agents, where raw sources remain available and synthesized wiki pages evolve over time. Patina generalises that idea into a Rust-based, CLI-first, Git-governed tool for software teams and agent-assisted development workflows.

The README should make clear that Patina is an independent implementation with different constraints:

> Patina is not tied to Obsidian, Claude Code, MCP, ChromaDB, or any hosted AI service. Those tools may integrate with or use Patina, but they do not define Patina’s architecture. The stable contract is the file-based knowledge directory, the local generated index, and the deterministic CLI.

## Recommended README References

Use these as documentation references, not implementation dependencies:

```text
Andrej Karpathy — LLM Wiki
Primary conceptual inspiration for the file-based, LLM-maintained Markdown wiki pattern.

Andrej Karpathy — broader LLM workflow material
Useful background for the general idea that LLMs become more effective when given durable context, explicit instructions, and structured working memory.
```
