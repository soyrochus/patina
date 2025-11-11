
# Patina Architecture Overview

Patina is a native desktop chat client written in Rust using `egui`/`eframe`.  
It provides a modular architecture for connecting to different AI model providers
(OpenAI, Azure OpenAI, and future local models) and integrating Model Context Protocol (MCP)
services in a fully local environment.

---

## 1. Workspace Layout

| Crate / Folder | Purpose |
|----------------|----------|
| **app/** | The main egui desktop application. Manages UI, input, menus, and projects. Renders chat history and model responses. |
| **core/** | Core business logic: conversation model, LLM provider abstraction, MCP client, and persistence layer. No UI code. |
| **xtask/** | Build, smoke test, and packaging utilities (`cargo run -p xtask -- smoke`). |
| **tests/** | Integration and end-to-end tests for `core`. |
| **.github/workflows/** | Release and CI pipeline for Linux, macOS, and Windows. |
| **images/**, **design/** | UI and logo assets. |
| **prompts/** | Prompt definitions for internal co-generation and documentation. |

---

## 2. Runtime Architecture

### UI Layer (`app/`)
- Built with **egui/eframe**, immediate-mode GUI.  
- Owns a single `AppState` struct holding all transient data.  
- Uses a **state machine** (`Idle`, `Requesting`, `Streaming`, `Canceled`, `Failed`) for chat lifecycle.  
- UI sends commands to a background worker instead of directly calling network APIs.

### Core Layer (`core/`)
- Defines the following subsystems:
  - **Conversation model** — role, content, timestamp, metadata (tokens, provider, tool calls).  
  - **Persistence** — append-only JSONL per project:  
    ```
    <project>/.patina/conversations/YYYY/<session>.jsonl
    ```
  - **Providers** — each implements:
    ```rust
    #[async_trait]
    pub trait LlmProvider {
        async fn complete(&self, req: Request) -> Result<Response, LlmError>;
        fn name(&self) -> &'static str;
        fn supports_streaming(&self) -> bool;
    }
    ```
  - **MCP Client** — manages discovery, authentication, and invocation of tools via JSON-RPC.

### Project Structure
Each project is a folder containing:

```text
ProjectName/
├─ ProjectName.pat        # Manifest (TOML)
└─ .patina/
└─ conversations/
└─ 2025/
└─ session_001.jsonl

```

### Configuration
Global configuration and secrets are stored under OS-specific paths:
- **Linux:** `$XDG_CONFIG_HOME/patina` or `$HOME/.config/patina`
- **macOS:** `~/Library/Application Support/Patina`
- **Windows:** `%APPDATA%\Patina`

---

## 3. Build and Release

The GitHub Actions workflow `Release (Linux + macOS arm64 + Windows)`:

- Builds static binaries named `patina` / `patina.exe`
- Embeds assets (logo, icons, metadata)
- Uploads to Releases on tag push (`v*`)
- Runs smoke tests via `xtask`

---

## 4. Current Weaknesses / Improvement Areas

| Area | Issue | Suggested Fix |
|-------|--------|---------------|
| **Streaming** | Partial streaming not yet implemented | Add async stream support and incremental UI updates |
| **Secrets** | Only environment variables supported | Integrate system keychain and redact logs |
| **Config paths** | Not unified | Centralize platform-aware config resolution |
| **MCP** | Only skeleton present | Define full lifecycle: discovery → auth → invocation |
| **Persistence** | Manual JSONL | Add schema versioning and migrations |
| **Error handling** | Mixed styles | Use `thiserror` for core, `anyhow` at app boundary |
| **Telemetry** | Minimal | Add `tracing` + optional OpenTelemetry exporter |
| **Tests** | Focused on core | Add golden-file tests for UI and provider mocks |
| **Release** | Functional but unsigned | Add signing, SBOM, and notarization |
| **Internationalization** | English only | Plan `fluent` or similar early if needed |

---

## 5. Summary

Patina’s design cleanly separates **presentation**, **logic**, and **providers**.
It is well-suited for iterative extension toward:
- Multiple provider backends (OpenAI, Azure, local)
- Local MCP toolchains
- Future distributed or multi-agent features

The core risk today is **coherence drift** as features grow.
The `core` crate must remain the single source of truth for logic,
and the `app` crate must stay a pure client consuming it.
