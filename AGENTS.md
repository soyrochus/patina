# Feature: agent-mode

## Role

**You are a senior implementation agent and expert in Rust**

## Objective

Extend **Patina** with a new **Agent Mode** while preserving the existing **Ask** mode.
The new Agent Mode introduces **code-execution with MCP (Model Context Protocol)** through a local **Orchestrator Runtime** and sandboxed **RhaiSandbox** engine.

**Important:**
Do **not** overwrite or modify the existing request–response (“Ask”) path.
Add a **mode selector** in the UI that allows switching between:

* **Ask** → current direct request/response behavior (default)
* **Agent Mode** → new orchestrated execution path described below

The UI must persist the user’s last selection (e.g., `"mode": "ask"` or `"mode": "agent"`) in `ui_settings.json`.

---

## Dual-Mode Behavior

| Mode              | Description                                                                                                                                                                               | Processing Path                                                                                 |
| ----------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------- |
| **Ask (default)** | Direct prompt–response mode. The message is sent straight to the configured LLM provider (e.g., OpenAI, Azure, local model).                                                              | `UI → LLM Provider → Response → UI`                                                             |
| **Agent Mode**    | The new orchestrated flow using code-execution with MCP. The prompt is sent to a local Orchestrator, which builds a Plan, executes code via the RhaiSandbox, and returns compact results. | `UI → Orchestrator Runtime → SandboxEngine (RhaiSandbox) → MCP tools → Orchestrator → LLM → UI` |

### UI Changes

* Add a **Mode Selector** next to the model dropdown:

  ```
  Mode: [ Ask ▼ | Agent Mode ▼ ]
  ```
* Default = **Ask**
* Persist the user’s choice in `ui_settings.json`:

  ```json
  "mode": "ask"
  ```

### Routing Logic

* If **Ask** → keep current flow unchanged.
* If **Agent Mode** → route prompt to the local **Orchestrator Runtime**, which manages planning, execution, and summarization.
* Both modes share the same chat interface and rendering system.

---

## Guardrails

Follow all guidelines defined in:

* [architecture.md](docs/architecture.md)
* [coding_guidelines.md](docs/coding_guidelines.md)

You may extend these when necessary to meet the new specifications, but **never overwrite or break existing functionality**.

---

## Operating Principles

* **Separation of concerns**

  * `UI ↔ Orchestrator ↔ SandboxEngine ↔ MCP tools`
* **Capability-based design** — only allowlisted tools per run.
* **Result compaction** — no large payloads in model context.
* **Determinism** — same plan + inputs + tool versions → same outputs.
* **Security by isolation** — Sandbox is a separate process under OS limits.
* **Observability** — unified tracing of time, cost, and token use.
* **Trait-based extensibility** — execution logic defined by `SandboxEngine`, with a single concrete implementation now: **RhaiSandbox**.
* **Dual-mode compatibility** — existing Ask mode remains untouched; Agent Mode runs independently through its own orchestrator pipeline.

---

## Deliverables

1. **UI update** — Add “Mode” selector; store choice in `ui_settings.json`.
2. **Orchestrator crate** — planning, graph execution, and budget enforcement.
3. **SandboxEngine trait** — abstraction for executing short code safely.
4. **RhaiSandbox** — concrete implementation using [Rhai](https://rhai.rs/).
5. **MCP client module** — schema discovery, caching, and on-demand tool loading.
6. **Contracts** — `Plan`, `ExecutionUnit`, `ResultEnvelope`, `ArtifactHandle`, `Error`.
7. **Capability manifest + policy layer.**
8. **Structured telemetry and logs.**
9. **Acceptance tests** verifying efficiency, isolation, and dual-mode stability.

---

## Core Components

### 1. Orchestrator

* **Planner:** builds a small **Plan** (DAG of `NodeSpecs`) from a goal.
* **Executor:** walks the DAG, dispatches steps to the SandboxEngine, enforces per-node budgets.
* **Reducer:** merges partial results and produces compact summaries.
* **PolicyGate:** validates allowed tools and write permissions.
* **Cache:** tool schemas, auth handshakes, repeated summaries.

### 2. SandboxEngine Trait

Abstract interface for code execution engines.

```text
trait SandboxEngine {
    fn execute(unit: ExecutionUnit) -> Result<ResultEnvelope, SandboxError>;
    fn health() -> SandboxHealth;
    fn capabilities() -> Vec<ToolName>;
}
```

### 3. RhaiSandbox (initial implementation)

* Executes **Rhai** scripts for step code.
* Registered host functions call only vetted MCP tool wrappers.
* Guards:

  * `max_operations`, `max_call_levels`, `max_string_len`, `max_array_size`.
  * Disable `eval`, `import`, `time`, and any I/O.
* Runs as a **separate process** with:

  * CPU/memory/file limits (`rlimit`, `seccomp` or OS equivalent).
  * Network disabled by default.
  * Watchdog timeout per execution.
* Returns a `ResultEnvelope`:

  ```json
  {
    "summary": "Fetched 45 rows; 12 updated",
    "artifacts": [{"type": "table", "handle": "artifact://..."}],
    "state_updates": {"changed": 12},
    "metrics": {"cpu_ms": 330, "mem_mb": 64}
  }
  ```

Future engines (e.g., **WASM** or **QuickJS**) will also implement this trait.

---

## Execution Contracts

**ExecutionUnit**

```json
{
  "engine": "rhai",
  "code": "/* short Rhai script */",
  "params": {"input_path": "..."},
  "allowed_tools": ["mcp://drive.read"],
  "budgets": {"cpu_ms": 1500, "mem_mb": 256}
}
```

**ResultEnvelope**

```json
{
  "summary": "Processed 12 items",
  "artifacts": [{"handle": "artifact://..."}],
  "state_updates": {"ok": true},
  "metrics": {"cpu_ms": 200}
}
```

**Error taxonomy**

```json
{"kind":"TOOL","code":"NOT_FOUND"}
{"kind":"CODE","code":"RUNTIME_ERROR"}
{"kind":"POLICY","code":"CAPABILITY_DENIED"}
{"kind":"BUDGET","code":"CPU_LIMIT"}
{"kind":"SANDBOX","code":"PROC_CRASH"}
```

---

## Policies & Capabilities

* **Manifest** defines allowed MCP tool URIs, rate limits, and read/write scope.
* **Secrets** live only in the sandbox (`secrets_ref`), never in prompts or logs.
* **Mutating operations** (e.g., write/update) require explicit approval nodes.

---

## Budgets & Limits

* **Per node:** CPU time, memory, max ops, output size, token cap.
* **Per run:** max nodes, total wall clock, max tool calls.
* **On breach:** terminate node → typed `BUDGET` error → Orchestrator may re-plan with reduced scope.

---

## Telemetry

* One **trace span** per node: metrics include tokens, time, mem, tool calls.
* **Run summary:** goal, plan hash, outcome, costs, policy decisions.
* **Sampling:** full traces on errors, sampled traces on success.
* **Redaction:** ensure no secrets or large data in summaries.

---

## Security & Isolation

* Sandbox runs as a separate process with:

  * no inbound network,
  * per-run rlimits,
  * OS-level isolation (namespace/AppContainer).
* Communication: JSON over stdio/domain socket.
* Watchdog kills worker on timeout or protocol violation.
* Static checks reject Rhai scripts exceeding code size or forbidden ops.

---

## Planner Behavior

* Convert user prompt → **Goal → Constraints → Plan**.
* Prefer local data transforms inside Sandbox over sending large payloads to LLM.
* Defer schema loading to when tools are actually used.
* Automatically insert **ApprovalNode** for any write operation.

---

## Executor Rules

* Execute nodes in DAG order.
* On error: classify, retry idempotent steps once, then re-plan or degrade gracefully.
* Maintain a shared `state` object; all inputs/outputs explicit.

---

## Caching

* Schema cache keyed by `server@version`.
* Deterministic result cache: same inputs → identical artifacts.
* Prompt cache to reduce repeated LLM work.

---

## Configuration

```
PATINA_AGENT_MODE=orchestrator
SANDBOX_ENGINE=rhai
SANDBOX_PATH=/usr/local/bin/patina-sandbox
MCP_REGISTRY=~/.patina/mcp/registry.json
CAPABILITIES=./.patina/capabilities.json
BUDGETS=./.patina/budgets.json
ARTIFACT_DIR=./.patina/artifacts
```

---

## Acceptance Criteria

1. **Ask mode unchanged:** existing request–response works identically.
2. **Agent Mode selectable:** second selector visible and persistent.
3. **Token use** reduced ≥ 85 % vs direct LLM-tool calls.
4. **Latency** improved ≥ 30 % for multi-tool flows.
5. **Security:** no secrets in logs/prompts; sandbox denies network.
6. **Reproducibility:** same plan + inputs → same results.
7. **Determinism tests:** repeated runs produce identical `summary` hashes.
8. **Budget enforcement:** infinite loop Rhai script halts with `CPU_LIMIT`.
9. **Policy enforcement:** Rhai scripts cannot access FS/env/time.
10. **Swap test:** replace RhaiSandbox with dummy `NullSandbox` without code changes.
11. **UX clarity:** users can switch Ask ↔ Agent Mode seamlessly; state saved in `ui_settings.json`.

---

## Work Plan

1. **UI update** — add Ask/Agent Mode selector; persist choice.
2. **Routing logic** — `Ask` = current direct call; `Agent` = new Orchestrator flow.
3. Scaffold Orchestrator and Sandbox crates.
4. Define JSON contracts and error taxonomy.
5. Implement `MCPClient` with progressive schema caching.
6. Implement `SandboxEngine` trait + RhaiSandbox subprocess + watchdog.
7. Integrate telemetry, budgets, and redaction.
8. Test dual-mode behavior, determinism, budgets, and isolation.
9. Demonstrate acceptance metrics; document results.

---

## Example Snippets (illustrative)

**Result summary example**

```text
"summary": "Fetched 50 records, filtered to 12 active, prepared artifact (artifact://...)."
```

**Capability manifest**

```json
{"allow":["mcp://drive.read","mcp://jira.update?fields=status"],"deny":["mcp://drive.delete"]}
```

**Error mapping**

```text
MCP 404 → {"kind":"TOOL","code":"NOT_FOUND"} → no retry → partial re-plan
```

---

✅ **Key principle:**

> **Ask** stays as-is. **Agent Mode** adds orchestration and sandboxing without interfering with current workflows.
