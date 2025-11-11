# Prompt — Wire model list from `patina.yaml`, persist selection in `ui_settings.json`, remove env usage

## Objective

Refactor Patina so that:

1. **All environment variable usage is removed**, including `.env` loaders and any `std::env` reads.
2. **Available models** for the dropdown are loaded from **`patina.yaml`** at app startup and whenever the **Settings** menu changes relevant configuration.
3. **Current selections** (model and temperature) are read from and persisted to **`ui_settings.json`** — this file remains the visualization state.
4. On **Send**, use the selected model and temperature from `ui_settings.json`.
5. Show a **modal error** if validation fails.

Preserve crate boundaries: UI in `app/`, logic in `core/`. No UI types in `core`. Do not block the egui loop.

---

## Sources of truth and scope resolution

### A. `patina.yaml` — configuration provider of the available models

* Purpose: supplies the authoritative **`available_models`** list for the dropdown.
* Location scope:

  * **Project scope**: `<project_root>/.patina/patina.yaml` (if “use project settings” or equivalent toggle is active).
  * **User scope**: user config dir when project scope is not active.

    * Linux: `$XDG_CONFIG_HOME/patina` or `$HOME/.config/patina`
    * macOS: `~/Library/Application Support/Patina`
    * Windows: `%APPDATA%\Patina`
* Load timing:

  * At application startup.
  * Whenever the user changes the Settings option that affects scope or provider configuration (i.e., switching between user vs project, or editing model-related settings).

### B. `ui_settings.json` — visualization state

* Purpose: stores **current user selections** for:

  * `model` (string)
  * `temperature` (number)
* Location scope:

  * Same resolution rule as above: project-scoped file overrides when project scope is active; otherwise use user config dir.
* Persistence:

  * Any change in the dropdown selection or temperature slider is **immediately saved** back to `ui_settings.json`.

---

## Required changes

### 1) Purge environment usage

* Remove all `std::env::var*` reads and any `.env`/dotenv usage across the workspace; delete the dependencies from `Cargo.toml`.
* Replace any former env-backed config with reads from `patina.yaml` and `ui_settings.json` as specified.

### 2) Settings I/O services (UI boundary, non-blocking)

Create or extend small services in `app/` (e.g., `app/src/settings.rs` and `app/src/config.rs`):

```rust
// Scope selection
pub enum Scope {
    User,
    Project(std::path::PathBuf),
}

// Visualization state (current selections)
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct UiSettings {
    pub model: String,        // e.g., "gpt-4o"
    pub temperature: f32,     // honor existing range used in the app
    // ... existing fields (theme, sizes, etc.)
}

// Provider config read from patina.yaml
#[derive(Clone, Debug, serde::Deserialize)]
pub struct ProviderConfig {
    pub available_models: Vec<String>,
    // leave room for other non-visual config keys that patina.yaml may already contain
}

// Load/save — must not block egui thread
pub async fn load_ui_settings(scope: &Scope) -> anyhow::Result<UiSettings>;
pub async fn save_ui_settings(scope: &Scope, s: &UiSettings) -> anyhow::Result<()>;

pub async fn load_provider_config(scope: &Scope) -> anyhow::Result<ProviderConfig>;
```

Implementation notes:

* Use an async file API or offload blocking I/O with a background task.
* On missing files:

  * `ui_settings.json`: create with safe defaults and persist.
  * `patina.yaml`: if absent, treat `available_models` as empty and surface a non-fatal warning to the UI (see validation below).

### 3) UI behavior — dropdown and temperature

* **Dropdown items** come **only** from `ProviderConfig.available_models` loaded from `patina.yaml`.
* **Selected value** is `UiSettings.model`. On change:

  * Update `UiSettings.model`.
  * Immediately `save_ui_settings(scope, &settings)`.
* **Temperature slider** is bound to `UiSettings.temperature`. On change:

  * Immediately save to `ui_settings.json`.
* **Reload trigger**:

  * When the user changes the Settings option that affects scope or provider configuration, reload `ProviderConfig` from `patina.yaml` and refresh the dropdown. If the current `UiSettings.model` is not in the new list, keep it in state but mark invalid (see validation) until the user picks a valid entry.

### 4) Validation and error dialog

Before sending:

* `available_models` must be non-empty. If empty, show a modal:

  * “No models are configured. Edit Settings to add models in patina.yaml.”
* `UiSettings.model` must be non-empty and present in `available_models`. If not, show:

  * “Selected model is not available. Pick a model from the list in patina.yaml.”

No network call is attempted if validation fails.

### 5) Core invocation contract

Keep `core` UI-agnostic and accept parameters:

```rust
pub async fn send_completion(
    model: &str,
    temperature: f32,
    prompt: &str,
) -> Result<CompletionResponse, LlmError>;
```

* UI passes `UiSettings.model` and `UiSettings.temperature`.
* Core maps `model` to the correct underlying provider logic as currently designed.
* Temperature is forwarded to the provider payload.

### 6) Send button flow

1. Read `UiSettings` from memory (already loaded) and `ProviderConfig` (already loaded).
2. Validate against `available_models`.
3. If valid, dispatch an async task calling `core::send_completion(model, temperature, prompt)`.
4. Append user message immediately; append assistant or error on completion.

### 7) Settings menu → reload semantics

* Implement a small function to handle “settings changed” events:

  * Recompute `Scope` (user vs project).
  * Reload `ProviderConfig` from `patina.yaml` in that scope.
  * Keep `UiSettings` scoped identically and continue to persist changes to the correct `ui_settings.json`.
  * If the selected model is now invalid, surface the validation state in the UI and block send until corrected.

---

## Tests and acceptance

### Unit tests

* **Scope resolution**: user vs project for both files.
* **ProviderConfig parsing**: missing file → empty list; malformed → surfaced error with clear message.
* **UiSettings round-trip**: change model and temperature, save, reload, values persist.

### Integration tests (headless)

* With project scope active and `patina.yaml` containing `["gpt-4o","gpt-4.1-mini"]` and `ui_settings.json` having `model="gpt-4o", temperature=0.7`:

  * Ensure dropdown is populated; `send_completion` receives `"gpt-4o"` and `0.7`.
* After changing scope to user where `patina.yaml` has only `["gpt-4.1-mini"]`:

  * Dropdown updates; if `UiSettings.model="gpt-4o"` remains, validation blocks send with the correct error until the user selects a valid item.

### Manual checks

* Startup: dropdown reflects models from `patina.yaml`; selection and temperature reflect `ui_settings.json`.
* Changing model or temperature persists immediately and survives restart.
* Deleting `.env` or any env usage has no effect because none is used.
* Changing settings scope causes a reload from the correct `patina.yaml`.

---

## Guardrails

* Do not read or write provider visualization state in `patina.yaml`. It only provides the config list `available_models`.
* Do not store current selections anywhere except `ui_settings.json`.
* Do not block the egui render loop; perform I/O off the UI thread.
* Keep `core` free of egui, and surface user-friendly errors at the UI boundary.
* `cargo fmt --all` and `clippy -D warnings` must pass.
* Follow rules and guidelines relatyed with architecture as described in [architecture.md](docs/architecture.md)  and 
 [coding_guidelines.md](docs/coding_guidelines.md)

**Implement exactly the above. Preserve crate boundaries and immediate-mode patterns.**
