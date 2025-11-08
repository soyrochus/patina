**Prompt for coding agent: Patina AI connectivity refactor**

Refactor the running Patina desktop app to add AI connectivity using a **single OpenAI-compatible driver** that supports both **OpenAI** and **Azure OpenAI**. Keep code multi-platform (Windows, macOS, Linux).

### What to build

* One “OpenAI driver” that switches behavior based on configuration:

  * `LLM_PROVIDER = "openai"` → OpenAI Chat Completions.
  * `LLM_PROVIDER = "azure_openai"` → Azure OpenAI Chat Completions using deployment, endpoint, and api-version.
* Read parameter names exactly as in the README:

  * `OPENAI_API_KEY`
  * `AZURE_OPENAI_API_KEY`, `AZURE_OPENAI_ENDPOINT`, `AZURE_OPENAI_API_VERSION`, `AZURE_OPENAI_DEPLOYMENT_NAME`
  * `LLM_PROVIDER` (selects backend)

### Configuration sources and precedence

Use this strict priority (higher wins, first found stops):

1. **Process environment variables**
2. **`.env` file in the current working directory** (load if present)
3. **`patina.yaml`** in the user’s home-config path
4. Otherwise: **no connection**; surface a clear UI/message: “AI not configured—create `patina.yaml` or set env vars.”

### `patina.yaml` schema (concise)

```yaml
ai:
  provider: openai | azure_openai
  openai:
    api_key: "…"              # if provider=openai
  azure_openai:
    api_key: "…"              # if provider=azure_openai
    endpoint: "https://<your>.azure.com/"
    api_version: "2024-12-01-preview"
    deployment_name: "gpt-4o"
```

### File locations (multi-platform)

Search for `patina.yaml` in this order (use the first that exists):

* **Linux:** `~/.config/patina/patina.yaml`, then `~/.patina/patina.yaml`
* **macOS:** `~/Library/Application Support/Patina/patina.yaml`, then `~/.patina/patina.yaml`
* **Windows:** `%APPDATA%\Patina\patina.yaml`, then `%USERPROFILE%\.patina\patina.yaml`

### Behavior

* Allow a default in `patina.yaml` (e.g., `provider: openai`), but let env vars override (e.g., switch to Azure by setting `LLM_PROVIDER=azure_openai` and Azure vars).
* Never log secrets. If missing/invalid config, do not crash; show a non-blocking status and a short “How to configure” hint.
* Keep the rest of the app unchanged; only wire the driver so a future call can stream chat responses.

**Deliverables:** config loader with precedence, OpenAI driver with dual backends, platform path resolution, `.env` support, and user-facing “not configured” status.
