# Patina

[![License: MIT](https://img.shields.io/badge/License-MIT-green.svg)](LICENSE)
![Rust 1.90+](https://img.shields.io/badge/Rust-1.90%2B-orange.svg)
[![Build: Cargo](https://img.shields.io/badge/Build-Cargo-8A2BE2.svg)](https://doc.rust-lang.org/cargo/)
![OS: macOS | Linux | Windows](https://img.shields.io/badge/OS-macOS%20%7C%20Linux%20%7C%20Windows-lightgrey.svg)
[![FOSS Pluralism Manifesto](https://img.shields.io/badge/Manifesto-FOSS%20Pluralism-8A2BE2.svg)](FOSS_PLURALISM_MANIFESTO.md)
[![Contributions welcome](https://img.shields.io/badge/Contributions-welcome-brightgreen.svg)](https://github.com/soyrochus/patina/issues)


![Patina logo](images/patina-logo-min-transparent.png)

Patina is a native desktop chat client built in Rust with an egui interface, designed to connect seamlessly with large language models (LLMs) through both cloud and local providers. The current implementation enables direct interaction with OpenAI models, providing a clean, responsive, and fully functional chat experience. While many advanced capabilities — such as local LLM integration and extended provider support — are still in active development, the application is evolving quickly toward a full-featured, independent alternative to proprietary AI clients.

Beyond serving as a chat interface, Patina is also conceived as an experimental platform for rapid AI integration and prototyping. Its modular design and support for the Model Context Protocol (MCP) allow developers to attach new agent models or services without altering the core code. This makes it ideal for fast iteration and experimentation, whether testing local LLMs, exploring new AI workflows, or building decoupled agent systems. As the project expands, Patina aims to remain both a practical everyday tool and a flexible testbed for AI-driven desktop innovation.

**Patina Desktop in Light Mode**
![Patina in Light mode](images/patina-min-light.png)

**Patina Desktop in Dark Mode**
![Patina in Dark mode](images/patina-min-dark.png)

## Workspace layout

```
patina/
├── app/          # Graphical user interface built with eframe/egui
├── core/         # Shared business logic, state, LLM providers, auth, and MCP
├── tests/        # Unit, integration, and end-to-end style tests
└── xtask/        # Automation helpers (smoke tests, fixtures, CI hooks)
```

Each crate has its own `Cargo.toml` and uses workspace dependencies declared at the root.

## Features

- **Chat experience:** Markdown-rendered conversations with syntax highlighting for code blocks via `egui_commonmark` and `syntect`.
- **LLM provider abstraction:** Unified driver for OpenAI, Azure OpenAI, and a mock provider used by tests. Streaming responses are planned but not yet implemented.
- **Authentication orchestration:** Handles server- and client-managed OAuth modes with persisted secrets ready for reuse.
- **MCP integration scaffolding:** JSON-RPC ready client registry capable of simulating tool invocations and auth handshakes.
- **Persistent history:** Conversations are stored as JSON Lines files and reloaded on startup.
- **Automation:** An `xtask smoke` command exercises the core logic without launching the UI.

## Getting started

### Prerequisites

- Rust 1.76 or newer with `cargo`
- A recent graphics driver capable of running `egui`/`eframe`

Optional environment variables configure LLM providers:

```
LLM_PROVIDER=openai              # or azure_openai, mock
OPENAI_API_KEY=...               # required for OpenAI provider
OPENAI_MODEL=gpt-4o-mini
AZURE_OPENAI_ENDPOINT=https://example.openai.azure.com/
AZURE_OPENAI_API_KEY=...
AZURE_OPENAI_DEPLOYMENT_NAME=gpt-4o
```

### Building and running the desktop app

```
cargo run -p patina
```

The first launch creates a data directory under your platform’s application data folder (for example, `~/Library/Application Support/Patina` on macOS). Conversations persist between sessions.

### Running automated tests

```
cargo test --workspace
```

To execute the smoke test provided by the automation crate:

```bash
cargo run -p xtask -- smoke
```

### Release binaries

Tagging a commit with `v*` (for example `git tag v0.3.0 && git push --tags`) triggers
`.github/workflows/release.yml`. The workflow builds single-file binaries named `patina`
(`patina.exe` on Windows) with embedded assets for Linux, macOS, and Windows, strips the symbols,
and uploads the artifacts as workflow outputs.

## Project Management

Patina organizes your conversations into **projects** — self-contained directories that store all chat history and settings. Each project is independent and portable, making it easy to organize different workstreams, share conversations, or back up your data.

### Project Structure

Each Patina project follows a simple directory layout:

```text
MyProject/
├── MyProject.pat              # Project manifest (TOML format)
├── (your files and folders)   # Optional user content
└── .patina/                   # Hidden project data
    └── conversations/          # Chat history (JSONL format)
        └── 2025/
            ├── conv1.jsonl
            └── conv2.jsonl
```

- **`ProjectName.pat`**: A TOML manifest file containing project metadata (name, creation date, internal paths)
- **`.patina/conversations/`**: Contains all conversation history in JSONL format, organized by year
- The project directory can contain any additional files or folders you need

### Creating a New Project

#### Creating via GUI

1. Launch Patina
2. Choose **File → New Project** from the menu
3. Select a location and enter a project name
4. Patina creates the project directory and opens it automatically

#### Creating via Command Line

```bash
# Create a new project directory
patina --new /path/to/MyProject --name "MyProject"

# Or specify just the .pat file location
patina --new /path/to/MyProject.pat --name "MyProject"
```

### Opening Projects

#### Opening via GUI

1. Choose **File → Open Project** from the menu
2. Navigate to either:
   - The project directory (e.g., `MyProject/`)
   - The `.pat` manifest file (e.g., `MyProject.pat`)
3. Patina loads the project and displays all conversations

#### Opening via Command Line

```bash
# Open by project directory
patina --project /path/to/MyProject/

# Or open by .pat file
patina --project /path/to/MyProject/MyProject.pat
```

### Importing and Exporting Projects

#### Export a Project

Export creates a ZIP archive containing the entire project directory:

```bash
# Command line export
patina export --project /path/to/MyProject --out /path/to/backup.zip
```

The exported ZIP contains:

- The project manifest (`.pat` file)
- All conversation history
- Any additional files in the project directory

#### Import a Project

Import extracts a project ZIP archive to a new location:

```bash
# Command line import
patina import --zip /path/to/backup.zip --into /path/to/destination/
```

After import, you can open the project normally. The imported project retains all conversations and settings.

### Recent Projects

Patina remembers recently opened projects for quick access. Recent projects appear in:

- The welcome screen when no project is open
- **File → Open Recent** menu (if implemented in UI)

### Project Independence

Each Patina project is completely self-contained:

- **No global settings**: Each project stores its own conversation history and preferences
- **IDE independent**: Projects don't interfere with VS Code workspaces or other development tools
- **Portable**: Copy or move project directories freely between machines
- **Isolated**: Different projects can use different LLM providers or settings

### Best Practices

- **Organize by purpose**: Create separate projects for different work areas (e.g., "WebDev", "Research", "Personal")
- **Regular exports**: Export important projects periodically for backup
- **Meaningful names**: Use descriptive project names that reflect their purpose
- **Keep it simple**: The project directory can contain additional files, but avoid complex nested structures

### Troubleshooting

- **"Project directory is not empty"**: When creating a project, ensure the target directory doesn't exist or is completely empty
- **"Project manifest not found"**: Verify the `.pat` file exists and matches the directory name exactly
- **Import fails**: Ensure the destination directory is empty or doesn't exist yet

## Project structure in detail

### app crate

Implements the `eframe` application. It renders the conversation list, message view, and composer. Background tasks spawn on a dedicated Tokio runtime and synchronize with the UI using unbounded channels. Streaming UI updates are planned, but the current client displays each response once it has fully completed.

### core crate

Holds the domain logic:

- `state.rs` – application state machine, conversation management, persistence hooks.
- `llm.rs` – provider abstractions for OpenAI, Azure OpenAI, and a mock driver used by tests.
- `mcp.rs` – lightweight MCP client and registry with auth-aware handshake scaffolding.
- `auth.rs` – server/client OAuth coordination that persists refreshed tokens alongside transcripts.
- `store.rs` – JSONL transcript storage and secret persistence.
- `telemetry.rs` – idempotent tracing initialization for binaries and tools.

### tests crate

Hosts unit, integration, and end-to-end tests. The initial suite validates conversation persistence and response generation using the mock LLM driver. Additional tests can be added under `unit/`, `integration/`, and `e2e/`.

### xtask crate

Provides automation entry points. `cargo run -p xtask -- smoke` spins up the core logic with the mock LLM driver and logs the resulting conversation metadata, suitable for CI smoke checks.

## Contributing

1. Fork and clone the repository.
2. Run `cargo fmt` before committing.
3. Add tests for any new functionality in the appropriate crate.
4. Use `cargo run -p xtask -- smoke` to validate end-to-end behavior.


## Principles of Participation

Everyone is invited and welcome to contribute: open issues, propose pull requests, share ideas, or help improve documentation. Participation is open to all, regardless of background or viewpoint.

This project follows the [FOSS Pluralism Manifesto](./FOSS_PLURALISM_MANIFESTO.md), which affirms respect for people, freedom to critique ideas, and space for diverse perspectives.


## License and Copyright

Copyright (c) 2025, Iwan van der Kleijn

This project is licensed under the MIT License. See the [LICENSE](LICENSE) file for details.
