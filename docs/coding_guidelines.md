# Patina Coding & Architecture Guidelines

This document defines style, architecture, and generation rules for AI-assisted code creation.

---

## 1. General Rules

- Use **Rust 2021** edition with `clippy` warnings as errors.
- Keep the workspace clean: UI in `app/`, logic in `core/`, automation in `xtask/`.
- Never import `egui` or UI types inside `core`.
- One module should have a clear ownership boundary.

---

## 2. Async and Concurrency

- All I/O or network work must be `async` using **tokio**.
- The UI thread (`eframe` main loop) must never block.
- Communication between UI and background logic should use channels:
  ```rust
  enum Cmd {
      SendPrompt{ project: ProjectId, conv: ConvId, prompt: String },
      Cancel{ request_id: Uuid },
      InvokeTool{ tool: ToolRef, args: serde_json::Value },
  }
    ```

* Background task consumes `Cmd` and emits events back.

---

## 3. Error Handling

* Library errors → `thiserror::Error`
* Application boundary → `anyhow::Result` with `.context()`
* Never use `.unwrap()` or `.expect()` except in tests.

---

## 4. Configuration & Secrets

Use the following paths:

| OS      | Path                                                |
| ------- | --------------------------------------------------- |
| Linux   | `$XDG_CONFIG_HOME/patina` or `$HOME/.config/patina` |
| macOS   | `~/Library/Application Support/Patina`              |
| Windows | `%APPDATA%\Patina`                                  |

Secrets must **not** be logged. Integrate the OS keychain when available.

---

## 5. Providers & MCP

* Define new providers by implementing the `LlmProvider` trait.
* Network code and retries must stay inside the provider module.
* MCP clients must be sandboxed: timeouts, size limits, clear error mapping.
* No provider-specific types in `app`.

---

## 6. UI Layer Practices

* Keep `AppState` minimal; use a clear state machine:

  ```rust
  enum RunState { Idle, Requesting, Streaming, Failed(String) }
  ```
* Render from immutable data; issue commands for side-effects.
* Group UI components into functions returning `egui::Response`.
* Use `egui_commonmark` + `syntect` for markdown with syntax highlighting.
* Call `ctx.request_repaint()` only when needed.

---

## 7. Logging and Telemetry

* Use the `tracing` crate:

  ```rust
  span!(Level::INFO, "llm_call", provider, model, project_id, conv_id);
  ```
* Optional OpenTelemetry exporter for external collection.

---

## 8. File and Module Structure

* Keep files under 400 lines.
* Group related types in a single module.
* Use `mod.rs` only for re-exports and high-level orchestration.
* Add doc comments (`///`) with examples for all public items.

---

## 9. Testing

* **Core:** mock HTTP and persistence round-trip tests.
* **UI:** snapshot tests via `egui` harness.
* **Xtask:** end-to-end “smoke” tests (simulate chat session).

---

## 10. Code Style

* Run `cargo fmt --all` before committing.
* Use `clippy` and fix all warnings.
* Follow Rust naming conventions:

  * Modules: `snake_case`
  * Types / Traits: `CamelCase`
  * Constants: `UPPER_SNAKE_CASE`
* Prefer `&str` over `String` unless ownership is required.
* Use `Arc` only when necessary—prefer single ownership with borrowing.

---

## 11. Do’s and Don’ts

✅ Do

* Keep `core` free of UI dependencies.
* Handle errors gracefully and display friendly messages.
* Write small, testable functions.
* Use structured logging for all network calls.
* Version persisted schemas.

🚫 Don’t

* Block inside `egui` callbacks.
* Store UI types in persistence.
* Use provider SDKs directly in the UI.
* Rely on global mutable state.
* Mix MCP and provider logic in the same module.

---

## 12. Commit and CI Discipline

* One feature or bug per PR.
* Update `CHANGELOG.md` for every user-visible change.
* CI must pass on all three OS targets.
* Release tags `v*` trigger GitHub Actions build and packaging.

---

## 13. Example Project Skeleton

```text
patina/
 ├─ app/
 │   ├─ main.rs
 │   ├─ ui/
 │   └─ settings.rs
 ├─ core/
 │   ├─ lib.rs
 │   ├─ provider/
 │   ├─ mcp/
 │   └─ conversation/
 ├─ xtask/
 │   └─ main.rs
 └─ tests/
     └─ core_tests.rs
```

---

## 14. Summary

Follow these conventions to keep Patina consistent, maintainable, and easy to extend.
The goal is **deterministic co-generation**: any AI agent generating Rust code
should produce output aligned with this architecture and style.

