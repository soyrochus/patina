# Patina

> *Knowledge that improves with age.*

[![CI](https://github.com/soyrochus/patina/actions/workflows/release.yml/badge.svg)](https://github.com/soyrochus/patina/actions/workflows/release.yml)
[![Crates.io](https://img.shields.io/crates/v/patina.svg)](https://crates.io/crates/patina)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](https://choosealicense.com/licenses/mit/)
[![Rust: 2024 edition](https://img.shields.io/badge/rust-2024%20edition-orange.svg)](https://www.rust-lang.org)

Patina is a local-first, Git-compatible knowledge tool for software teams and AI-assisted development workflows. It keeps project knowledge in plain Markdown files, builds a disposable local SQLite index, and exposes deterministic CLI commands that both humans and coding agents can rely on.

---

![Knowledge](./images/knowledge.png)

## The Idea Behind Patina

### The LLM Wiki pattern

The concept originates from Andrej Karpathy's **LLM Wiki** idea: instead of letting AI-generated synthesis disappear into chat history, you accumulate it as an evolving Markdown wiki. Raw source material — meeting notes, specs, articles, decisions — lives in one layer. Curated, cross-linked wiki pages synthesizing that material live in another. An LLM or coding agent helps maintain the wiki over time: creating pages, updating them when sources change, surfacing contradictions, and filling gaps.

The appeal is that the knowledge base becomes a durable team artifact rather than a personal chat transcript. It can be read without a special application, reviewed as normal Git diffs, and queried repeatedly by any tool that can run a CLI command.

### What Patina adds

Patina takes the LLM Wiki idea and makes it an engineering-grade tool:

- **Git is the source of truth.** Knowledge files are committed, reviewed, and versioned like code. The local index is generated state — always rebuildable, never committed.
- **Provenance is explicit.** Every wiki page can declare which source files it was synthesized from. Patina tracks source hashes and warns when a referenced source has changed since the page was last reviewed.
- **Retrieval is deterministic.** Chunking, indexing, and scoring use stable algorithms. The same query against the same knowledge base always produces the same result ranking.
- **The CLI is the contract.** Agent integrations are generated Agent Skills that tell tools how to call `patina query`, `patina read`, `patina lint`, and `patina stale`. There is no MCP server, no hosted service, and no vendor lock-in in the core.

The project is intentionally *not*:

- an Obsidian-specific vault format (works with any Markdown editor)
- a vector database (lexical FTS5 first; semantic retrieval is a planned future layer)
- a chat memory system (everything is inspectable files)
- tied to a particular AI provider or agent framework

This design comes from the repository's own knowledge base. The `knowledge/` directory at the root of this repository is a live Patina knowledge base about Patina itself — the `knowledge/wiki/` pages were synthesized from the specification notes in `knowledge/sources/` and are cross-linked, front-matter-tagged, and tracked with the same provenance model described below.

---

## Directory Layout

A Patina-managed repository has two layers:

```text
knowledge/          ← committed to Git; the durable knowledge base
  AGENTS.md         ← instructions for AI agents using this repo
  README.md         ← human-readable index
  scope.yaml        ← optional confidentiality and export policy
  wiki/             ← synthesized, curated Markdown pages
  sources/          ← raw source notes, summaries, extracted content
  schemas/          ← page-type schemas (documentation / future enforcement)

.patina/            ← Git-ignored; generated local state; always disposable
  index.sqlite      ← SQLite FTS5 index of all knowledge pages
  index.lock        ← advisory lock file for concurrent safety
```

The design boundary is strict: **everything under `knowledge/` is canonical**; everything under `.patina/` is a cache. If `.patina/` is deleted, `patina index --full` reconstructs it completely from the Markdown files.

---

## Page Format

Every wiki page is a Markdown file with a YAML front matter block:

```markdown
---
title: Controlled Agent Autonomy
type: concept
status: active
aliases:
  - controlled autonomy
  - agent autonomy
tags:
  - agents
  - architecture
review_after: 2026-12-01
source_refs:
  - sources/workshop-notes.md
  - sources/spec-001-base-implementation.md
---

# Controlled Agent Autonomy

...page content...
```

**Required fields:** `title`, `type`, `status`

**Supported `type` values:** `concept`, `system`, `project`, `decision`, `person`, `process`, `glossary`, `source`, `index`, `open-question`

**Supported `status` values:** `draft`, `active`, `deprecated`, `archived`

`source_refs` lists the local source files the page was synthesized from. Patina records the SHA-256 of each source at index time and warns via `patina stale` when a source has changed since the page was last reviewed. `review_after` triggers a staleness warning when the date passes.

---

## Install

**From this repository:**

```bash
cargo install --path .
```

**From crates.io** (when published):

```bash
cargo install patina
```

**From a Git checkout:**

```bash
cargo install --git https://github.com/soyrochus/patina.git
```

Pre-built binaries for Linux x64, macOS x64, macOS arm64, and Windows x64 are attached to each [GitHub Release](https://github.com/soyrochus/patina/releases). Download the archive for your platform and place the `patina` binary on your `PATH`.

No Python, Node.js, ChromaDB, or database server is required. Patina ships with SQLite bundled in the binary.

---

## Configuration File

Patina reads an optional `patina.toml` from the repository root. If the file is absent, all defaults apply. A minimal annotated file is included in this repository:

```toml
# patina.toml

[knowledge]
# dir = "knowledge"   # default; change to use a different directory name

[index]
# chunk_size = 1200
# chunk_overlap = 150
# chunk_strategy = "heading-aware"

[limits]
# max_markdown_file_mb = 10
# max_source_file_mb = 50
# max_total_markdown_files = 50000

[security]
# allow_internal_symlinks = false
# allow_external_symlinks = false
```

### Naming the knowledge directory

`knowledge/` is the default, but the name is not mandatory. If your project already uses a different convention, set `knowledge.dir` in `patina.toml`:

```toml
[knowledge]
dir = "docs"       # or "wiki", "notes", "context", anything you prefer
```

Every Patina command resolves file paths relative to the configured directory. The name shown in `patina status`, `patina doctor`, and all JSON output reflects whatever you set here. The only fixed rule is that the directory must live inside the repository root.

---

## Getting Started

```bash
# Initialise a new knowledge base in the current Git repository
patina init

# Add some Markdown pages to knowledge/wiki/ ...

# Build the local index
patina index --reset

# Search
patina query "controlled autonomy"

# Read a specific page
patina read knowledge/wiki/concepts/controlled-agent-autonomy.md

# Check for broken links, missing front matter, and duplicate aliases
patina lint

# Check whether any referenced sources have changed
patina stale

# Diagnose the local environment
patina doctor
```

---

## Command Reference

### `patina init`

Scaffolds the knowledge directory structure, adds `.patina/` to `.gitignore`, and optionally detects Git worktree presence.

```bash
patina init           # warn if outside Git
patina init --no-git  # skip the Git check
```

### `patina status`

Reports the current state of the repository and index: Git worktree presence, uncommitted knowledge changes, index age, and scope metadata.

```bash
patina status
patina status --json
```

### `patina lint`

Validates every Markdown page in the knowledge directory:

- Required front matter fields (`title`, `type`, `status`)
- Allowed `type` and `status` values
- Page-type-specific required fields (configurable)
- Internal link targets
- Alias uniqueness across all pages
- Source reference existence
- Scope provenance

```bash
patina lint
patina lint --json
```

### `patina index`

Builds (or rebuilds) the local SQLite index with deterministic heading-aware chunking and FTS5 full-text search.

```bash
patina index           # incremental: only changed files
patina index --full    # re-index all files
patina index --reset   # drop and rebuild from scratch (atomic)
patina index --json
```

Chunking splits each Markdown file by its heading tree. Each heading section becomes one logical chunk. Sections exceeding the configured token limit are split at paragraph boundaries with configurable overlap. Every chunk receives a SHA-256 hash, making re-index detection exact.

### `patina query`

Searches the index using SQLite FTS5 BM25 ranking. Results are scored with a transparent seven-component formula:

| Component    | Weight | Signal                              |
| ------------ | ------ | ----------------------------------- |
| FTS5 BM25    | 70%    | Full-text relevance                 |
| Title match  | 10%    | Query term in page title            |
| Alias match  | 7%     | Query term in declared aliases      |
| Tag match    | 5%     | Query term in declared tags         |
| Page type    | 3%     | Preferred types (concept, decision) |
| Freshness    | 3%     | Linear decay over 365 days          |
| Provenance   | 2%     | Page has source references          |

```bash
patina query "controlled autonomy"
patina query "agent loop" --limit 5
patina query "decision" --json
patina query "architecture" --json --explain   # include score breakdown
```

If FTS5 is unavailable, Patina falls back to LIKE-based search and notes the degraded mode in the output.

### `patina read`

Returns the content of a knowledge page. Canonicalizes paths and enforces a strict knowledge-root boundary — no path traversal, no symlinks resolving outside the root.

```bash
patina read knowledge/wiki/concepts/controlled-agent-autonomy.md
patina read knowledge/wiki/concepts/controlled-agent-autonomy.md --json
```

### `patina stale`

Checks for pages that may need review:

- `review_after` date has passed
- A referenced source file has changed since the page was last indexed
- A referenced source file is missing
- A `deprecated` page is still linked from `active` pages
- A `draft` page is older than the configured age threshold (default: 90 days)

```bash
patina stale
patina stale --json
```

### `patina doctor`

Diagnoses the local environment: Git worktree, knowledge directory presence, SQLite integrity, FTS5 availability, schema version, file permissions, agent instruction files, and scope configuration validity.

```bash
patina doctor
patina doctor --json
```

### `patina install-skills`

Installs Patina Agent Skills for coding tools that support `SKILL.md`-based skills. Every run ensures the shared operating instructions exist at `<knowledge_dir>/AGENTS.md`. Host-specific skill files are written only when `--for` is provided.

```bash
patina install-skills
patina install-skills --for github-copilot
patina install-skills --for codex
patina install-skills --for claude-code
patina install-skills --for github-copilot --for codex
patina install-skills --for all
patina install-skills --for github-copilot --force
patina install-skills --for codex --json
```

Supported targets:

| Target | Files written |
| ------ | ------------- |
| no `--for` | `knowledge/AGENTS.md` only |
| `github-copilot` | `.github/skills/patina-query/SKILL.md`, `.github/skills/patina-check/SKILL.md` |
| `codex` | `.agents/skills/patina-query/SKILL.md`, `.agents/skills/patina-check/SKILL.md`, root `AGENTS.md` |
| `claude-code` | `.claude/skills/patina-query/SKILL.md`, `.claude/skills/patina-check/SKILL.md` |
| `all` | all host-specific skill targets |

Patina installs two skills for each selected host:

- `patina-query`: search the Patina knowledge base, read the most relevant pages, and answer from Git-tracked Markdown content.
- `patina-check`: run lint and stale checks before or after editing knowledge files.

Generated `SKILL.md` files include the marker `<!-- PATINA GENERATED SKILL -->`. Patina replaces files with that marker on later runs. Existing non-managed skill files are skipped by default and overwritten only with `--force`.

Generated skills do not include `allowed-tools`; the agent host keeps its normal command approval behavior.

---

## JSON Output

Every command accepts `--json` and returns a stable envelope:

```json
{
  "version": "0.1",
  "command": "query",
  "ok": true,
  "data": { ... },
  "warnings": [],
  "errors": []
}
```

On failure, `ok` is `false`, `data` is `null`, and `errors` contains structured entries with `code`, `message`, `severity`, and optionally `path`. This envelope is designed for agent consumption: a coding agent can call `patina lint --json` and parse `errors` without screen-scraping.

---

## Configuration

Patina reads `patina.toml` from the repository root. All keys are optional; defaults are applied when the file is absent or incomplete.

```toml
[knowledge]
dir = "knowledge"

[index]
chunk_size = 1200          # estimated tokens per chunk
chunk_overlap = 150        # token overlap when splitting large sections
chunk_strategy = "heading-aware"

[limits]
max_markdown_file_mb = 10
max_source_file_mb = 50
max_total_markdown_files = 50000

[security]
allow_internal_symlinks = false
allow_external_symlinks = false

[lint.page_types.decision]
required = ["title", "type", "status", "decided_on"]

[lint.page_types.source]
required = ["title", "type", "status", "source_kind"]
```

---

## Skill Usage

Agents should treat Patina as read-mostly durable context.

Install shared instructions and optional host-specific skills with:

```bash
patina install-skills
patina install-skills --for codex
patina install-skills --for github-copilot
patina install-skills --for claude-code
```

The generated skills are task-scoped wrappers around the CLI:

- Use `patina-query` when answering questions about project context, architecture, decisions, domain knowledge, or prior repository knowledge.
- Use `patina-check` when validating knowledge health or before changing files under the knowledge directory.

**Finding relevant knowledge:**

```bash
patina query "topic of interest" --json --limit 5
```

**Reading a specific page:**

```bash
patina read knowledge/wiki/systems/control-plane.md --json
```

**Checking knowledge health before writing:**

```bash
patina lint --json
patina stale --json
```

**After adding or updating a page:**

```bash
patina lint --json     # verify the new page is valid
patina index           # update the index incrementally
```

The shared `knowledge/AGENTS.md` file remains the full operating reference for humans and agents. Host-specific `SKILL.md` files point back to that shared file and do not require MCP or a hosted service.


## Try the Knowledge Workflow

Patina is easiest to understand by using it with an AI coding agent.

The repository includes a guided exercise that asks an agent to add useful project knowledge, validate it, index it, and query it back through Patina:

[Exercise: Let Patina Teach Itself Something Useful](docs/patina-knowledge-exercise.md)

The exercise demonstrates the intended workflow:

```bash
patina query
patina read
patina lint
patina index
patina stale
```

It also shows the main design principle in practice: Git-tracked Markdown is the durable knowledge base, while `.patina/` is only generated local state.

---

## Contributing & Principles of Participation

Pull requests are welcome. For major changes, open an issue first to discuss the approach.

Please update tests when changing behaviour. The test suite includes unit tests, integration tests, fixture-based tests, and golden output tests — all of which should pass before a pull request is opened.

Everyone is welcome to contribute: open issues, propose pull requests, share ideas, or improve documentation. Participation is open to all, regardless of background or viewpoint.

This project follows the [FOSS Pluralism Manifesto](./FOSS_PLURALISM_MANIFESTO.md), which affirms respect for people, freedom to critique ideas, and space for diverse perspectives.

---

## Copyright and License

Copyright © 2026 Iwan van der Kleijn

Licensed under the [MIT License](https://choosealicense.com/licenses/mit/). See the [LICENSE file](./LICENSE) in the repository.
