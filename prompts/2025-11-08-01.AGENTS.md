# Build Patina: Rust egui Desktop Chat Client with MCP and Auth Modes

> Use this prompt verbatim to implement the application. You are an advanced code generation agent. Produce production-grade code, docs, and tests. Follow the constraints precisely. Ask no questions unless the requirement is genuinely ambiguous.

---

## Objective

Implement **Patina**, a native desktop chat client in **Rust** with **egui** UI that replicates a ChatGPT-like experience, supports **tool use via MCP**, and implements **two authentication modes** for MCP-connected backends:

1. **Server-managed OAuth** — The MCP server owns OAuth with third-party services (e.g., Atlassian Jira/Confluence). The desktop client authenticates to the MCP via an MCP client credential (e.g., static bearer/API key). First-time backend OAuth is a browser flow triggered by the MCP server; thereafter the server uses a stored refresh token. The client should reconnect later with no additional prompts.
2. **Client-managed OAuth with credential pass-through** — The desktop client runs a PKCE browser flow, stores tokens in the OS keychain, and injects access tokens to the MCP server if the MCP advertises credential pass-through for its backends.

The app must auto-connect to all configured MCP endpoints and maintain seamless re-use of auth state over days or weeks within token expiry rules.

---

## High-level capabilities

* Chat UI with streaming responses and tool-use blocks (ChatGPT-like UX).
* Pluggable LLM providers (OpenAI and Azure OpenAI through a unified driver).
* MCP client over **stdio** initially; extensible to **WebSocket**. JSON-RPC 2.0 request/response handling.
* Capability-driven auth handshake: detect per-MCP `auth_mode` and behave accordingly.
* Persistent sessions and settings. Clean separation of UI, state, providers, MCP, and credentials.

---

## Workspace architecture

Patina must be implemented as a **multi-crate Cargo workspace**. This allows modularity, reuse, and automated testing.

### Layout

```
patina/                     # Workspace root
├── Cargo.toml              # [workspace] declaration
├── app/                    # GUI binary crate (egui + orchestration)
│   ├── Cargo.toml
│   └── src/
│       ├── main.rs
│       └── app.rs
├── core/                   # Core library crate (state, llm, mcp, auth, store)
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs
│       ├── state.rs
│       ├── llm.rs
│       ├── mcp.rs
│       ├── auth.rs
│       ├── store.rs
│       └── telemetry.rs
├── tests/                  # Integration, unit, and e2e test crate
│   ├── Cargo.toml
│   └── src/
│       ├── unit/*
│       ├── integration/*
│       └── e2e/*
└── xtask/                  # Optional helper binary for automation
    ├── Cargo.toml
    └── src/main.rs
```

* **app/** – contains the egui front end, main event loop, and user interaction logic.
* **core/** – implements reusable logic for LLM calls, MCP communication, state management, auth flows, storage, etc.
* **tests/** – orchestrates integration and end-to-end tests (including mocks).
* **xtask/** – provides helper utilities (e.g., start mock servers, generate fixtures, CI smoke tests).

Each crate has its own `src/` directory, adhering to Cargo best practices.

---

## Configuration and environment variables

Patina supports both OpenAI and Azure OpenAI via a single driver. The configuration can come from a `.env` file, environment variables, or `settings.toml`.

### Example

```bash
LLM_PROVIDER="azure_openai"        # or: "openai"
# Azure OpenAI
AZURE_OPENAI_API_KEY=SECRET_KEY_REDACTED_FOR_SAFETY
AZURE_OPENAI_ENDPOINT=https://[your_azure_deployment].azure.com/
AZURE_OPENAI_API_VERSION=2024-12-01-preview
AZURE_OPENAI_DEPLOYMENT_NAME=gpt-4o

# OpenAI
OPENAI_API_KEY=SECRET_KEY_REDACTED_FOR_SAFETY
```

The driver auto-detects `LLM_PROVIDER` and adjusts headers and endpoints accordingly.

---

## Authentication requirements

*(Retains prior details for both server-managed and client-managed modes.)*

---

## Markdown rendering with syntax highlighting

* Use `egui_commonmark` for markdown parsing.
* Integrate `syntect` for fenced code block highlighting with theme cache.
* Use async background rendering for long messages.

---

## Chat history and persistence

* Store per-session transcript as `.jsonl` under the workspace data directory.
* Implement lazy loading, search, and export/import features.

---

## Testing strategy and automation

* Unit, integration, end-to-end, and UI snapshot tests live in `tests/`.
* Each core component (`auth`, `llm`, `mcp`, `state`) has independent unit tests.
* The `tests` crate launches a mock MCP server to validate OAuth flows, tool calls, reconnect, and persistence.
* A headless **smoke test** runs via `cargo run -p xtask -- smoke` that launches mocks, starts Patina in headless mode, performs scripted interactions, validates logs, and exits with code 0 on success.

### CI workflow

* GitHub Actions CI defined at workspace root:

  * `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`
  * `cargo test --workspace --all-features`
  * Run smoke test via `xtask` on Ubuntu, macOS, Windows.
  * Upload logs, UI snapshots, and golden transcripts.

---

## Acceptance criteria

*(As defined earlier, unchanged)*

---

## Stretch goals

*(As defined earlier, unchanged)*

---

## Implementation guidance

* Multi-crate modular design for maintainability and reuse.
* Each crate has its own `src/` root.
* Workspace root (`patina/`) holds `Cargo.toml` with `[workspace]` members.
* Core logic must compile independently for testing without GUI.
* `xtask` automates mock server management and CI checks.
* Add provider conformance tests for both OpenAI and Azure OpenAI streaming paths.
* Add UI snapshot tests to validate syntax highlighting and long-history virtualization.


## README

Create a README.md describing the full app according to standards common on Github. Description, installation, manual etc

