# agent.md — Patina Projects (minimal refactor)

## Role

You are a senior Rust engineer. Implement a **minimal refactor** so Patina can work with **multiple projects** on disk. Do **not** add new subsystems (no SQLite, no secrets store, no blobs/cache managers). Keep the current conversation storage format and logic—just relocate it into each project’s directory and add import/export.

## Goals

* Patina operates on a **Project** selected by the user.
* A Project is a **directory** anywhere on disk, with:

  * A **root index file** named exactly like the directory, with `.pat` extension.
  * A hidden **`.patina/` directory** containing the **conversations** subfolder that holds the existing JSONL history (moved from the old home/Library location).
* Patina can **Create**, **Open**, **Export** (zip), and **Import** (unzip) projects.
* Patina remains **decoupled from any IDE workspace** (e.g., VS Code).

## Non-Goals (for this refactor)

* No SQLite or new indices.
* No secrets management, blobs, or cache subsystems.
* No new message formats—reuse existing JSON/JSONL as-is.

## On-disk layout (minimal)

For a project named `Atlas`:

```
Atlas/
├─ Atlas.pat                 # root index file, same basename as directory
├─ (optional user files and folders)
└─ .patina/
   └─ conversations/         # existing conversation history moved here (JSON/JSONL)
      ├─ 2025/
      │  └─ <uuid>.jsonl
      └─ (whatever the current implementation already writes)
```

### `Atlas.pat` (TOML)

Small, human-readable. Only what we need now.

```toml
version = 1
name = "Atlas"
created_utc = "2025-11-09T00:00:00Z"

[paths]
internal = ".patina"
conversations = ".patina/conversations"
```

## Behavioral requirements

### Create Project

* User chooses a new directory or name; app scaffolds:

  * `<Name>/`
  * `<Name>/<Name>.pat` (populate with minimal TOML above)
  * `<Name>/.patina/conversations/` (empty)
* Set this project as current.

### Open Project

* Accept either the project directory or the `.pat` file path.
* Validate the presence of `<Dir>/<DirName>.pat` and `.patina/conversations/`.
* Load the current conversation list from `.patina/conversations/` using the **existing** loaders.

### Refactor storage location

* **All writes/reads of conversation history must move from the old app-data/home/Library path to `project/.patina/conversations/`.**
* The conversation line format, file naming, and rotation stay exactly as today.

### One-time migration (optional UX)

* When opening a project for the first time on this machine, **offer** to import the legacy global history:

  * Copy existing conversation files from the old location into `project/.patina/conversations/` preserving structure and timestamps.
  * Do not delete the legacy copy automatically; offer a “Delete legacy store” button after successful copy.

### Export (zip)

* `Export Project` produces a `.zip` of the entire project directory (including `.patina/conversations/` and the `.pat` file).
* No special redactions in this minimal refactor—just a faithful zip of the tree.

### Import (zip)

* `Import Project` lets the user pick a zip and a destination directory. Unzip into the chosen folder.
* Validate after unzip and then open the imported project.

### Decoupling from IDEs

* A Patina Project is **not tied** to VS Code or any source workspace. Opening a Patina Project must not read or write to `.vscode/` or any repo metadata.

## UI requirements (light)

* Welcome screen: **New Project**, **Open Project**, **Open Recent**.
* Window title: `Patina — <ProjectName>`.
* Side panel shows conversations from the current project (exactly as today, just reading the new path).
* Export/Import actions in a Project menu.

## CLI (optional but simple)

* `patina --project <path>` where `<path>` is a directory or `.pat` file.
* `patina --new <dir> --name <Name>` creates a project skeleton.
* `patina export --project <dir> --out <zip>`
* `patina import --zip <zip> --into <dir>`

## Implementation tasks

1. **Paths and model**

   * Introduce `ProjectPaths { root, pat_file, internal, conversations }`.
   * Resolve paths from the `.pat` file’s location.

2. **Create/Open**

   * `ProjectHandle::create(at: &Path, name: &str) -> Result<ProjectHandle>`.
   * `ProjectHandle::open(from: &Path) -> Result<ProjectHandle>` supports either dir or `.pat` file.

3. **Conversation IO refactor**

   * Replace all references to the legacy home/Library storage with `ProjectPaths.conversations`.
   * Keep serializers, file rotation, and JSONL format unchanged.

4. **Import/Export**

   * `ProjectHandle::export_zip<W: Write>(&self, to: W) -> Result<()>` — stream zip of the project directory.
   * `ProjectHandle::import_zip<R: Read>(zip: R, into_dir: &Path) -> Result<ProjectHandle>` — unzip → validate → open.

5. **Legacy migration (optional UX)**

   * Add a dialog on first open: “Import previous Patina history into this project?” If yes, copy files as-is into `.patina/conversations/`.

6. **Recent projects**

   * Maintain a user-level recent list of project paths (does not affect the project structure).

## Acceptance criteria

* Creating a project generates `<Name>/<Name>.pat` and `.patina/conversations/`.
* Starting a new conversation writes only under `.patina/conversations/`.
* Opening an existing project shows the same conversation list and content as before the refactor (after optional migration).
* Export produces a single zip that, when imported elsewhere, opens and works with all conversations intact.
* No reads/writes occur in the legacy home/Library store once a project is open.
* Patina functions normally when VS Code is closed or open on an unrelated repo; no `.vscode/` or repo writes.

## Notes for the codegen agent

* Use existing conversation IO code paths; only change the **root** directory they point to.
* Keep all formats stable; do not add DBs, caches, or new files.
* Be careful with path handling across platforms (Windows/macOS/Linux).
* Ensure zip operations stream to avoid large memory spikes.
