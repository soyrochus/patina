# Patina

Patina is a local-first Markdown knowledge tool for making AI-assisted synthesis durable. It keeps project knowledge in Git, builds a disposable local SQLite index, and exposes deterministic CLI commands for humans and agents.

The project is inspired by Andrej Karpathy's LLM Wiki pattern: keep useful context in plain files, make retrieval cheap, and let agents read from the same durable knowledge base people maintain.

## Quick Start

```bash
cargo install --path .
patina init
patina index --reset
patina query "controlled autonomy"
```

Core commands:

```bash
patina lint --json
patina read knowledge/wiki/example.md --json
patina stale --json
patina doctor --json
patina install-agent --agent claude-code
```

## Install

From this repository:

```bash
cargo install --path .
```

From a Git checkout:

```bash
cargo install --git https://github.com/soyrochus/patina.git
```

Release builds are produced for Linux x64, macOS x64, macOS arm64, and Windows x64 when a version tag is pushed.

## Data Model

Patina stores source knowledge in `knowledge/` as Markdown with YAML front matter. The local index lives in `.patina/` and is intentionally disposable. Deleting `.patina/` and running `patina index --full` reconstructs the index from committed knowledge files.

## Independence

Patina is not tied to Obsidian, Claude, MCP, ChromaDB, Python, Node.js, or a hosted service. The CLI is the stable contract; agent-specific files are thin instruction wrappers around `patina query`, `patina read`, `patina lint`, and `patina stale`.


## Contributing & Principles of Participation

Pull requests are welcome. For major changes, please open an issue first
to discuss what you would like to change.

Please make sure to update tests as appropriate.

Everyone is invited and welcome to contribute: open issues, propose pull requests, share ideas, or help improve documentation.  
Participation is open to all, regardless of background or viewpoint.  

This project follows the [FOSS Pluralism Manifesto](./FOSS_PLURALISM_MANIFESTO.md),  
which affirms respect for people, freedom to critique ideas, and space for diverse perspectives.  

## Copyright and license

Copyright © 2026 Iwan van der Kleijn

Licensed under the MIT License 
[MIT](https://choosealicense.com/licenses/mit/)