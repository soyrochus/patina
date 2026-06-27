## ADDED Requirements

### Requirement: Binary entry point and subcommand dispatch
The `patina` binary SHALL use `clap` (derive API) for argument parsing with a top-level subcommand dispatch to `init`, `status`, `lint`, `index`, `query`, `read`, `stale`, `doctor`, and `install-agent`.

#### Scenario: Invoke with no arguments
- **WHEN** the user runs `patina` with no arguments
- **THEN** the binary prints help text listing all subcommands and exits with code 0

#### Scenario: Invoke unknown subcommand
- **WHEN** the user runs `patina unknown-cmd`
- **THEN** the binary prints an error and exits with a non-zero code

### Requirement: TOML configuration loading
Patina SHALL load configuration from a `patina.toml` file in the repository root (or knowledge root). If no config file exists, Patina SHALL apply compiled-in defaults without error.

#### Scenario: Config file absent
- **WHEN** no `patina.toml` is found
- **THEN** Patina runs with default values for all config keys and does not emit a warning

#### Scenario: Config file present with partial keys
- **WHEN** `patina.toml` contains only some config keys
- **THEN** Patina merges the file values with defaults; missing keys use their defaults

#### Scenario: Config file is invalid TOML
- **WHEN** `patina.toml` contains malformed TOML
- **THEN** Patina exits with a clear error identifying the parse problem and the file path

### Requirement: Default configuration values
Patina SHALL ship with the following compiled-in defaults:

```toml
[knowledge]
dir = "knowledge"

[index]
chunk_size = 1200
chunk_overlap = 150
chunk_strategy = "heading-aware"

[limits]
max_markdown_file_mb = 10
max_source_file_mb = 50
max_total_markdown_files = 50000
max_chunk_token_estimate = 1200

[security]
allow_internal_symlinks = false
allow_external_symlinks = false

[workspace]
enabled = false
roots = []
```

#### Scenario: Default chunk size applied
- **WHEN** no `[index]` section is present in config
- **THEN** `patina index` uses `chunk_size = 1200` and `chunk_strategy = "heading-aware"`

### Requirement: Multi-root workspace guard
If `workspace.enabled = true` is set in config, Patina SHALL exit with an error stating multi-root workspaces are not supported in this version.

#### Scenario: workspace.enabled is true
- **WHEN** `patina.toml` contains `[workspace]` with `enabled = true`
- **THEN** any Patina command exits with `error: multi-root workspaces are not supported in this version`

### Requirement: Stable JSON output envelope
Every command that accepts `--json` SHALL produce a JSON object with the following top-level keys: `version` (string), `command` (string), `ok` (boolean), `data` (object or null), `warnings` (array), `errors` (array).

#### Scenario: Successful command with --json
- **WHEN** a command completes successfully with `--json`
- **THEN** output is a JSON object where `ok` is `true`, `data` contains command-specific payload, `warnings` and `errors` are arrays (possibly empty)

#### Scenario: Failed command with --json
- **WHEN** a command fails with `--json`
- **THEN** output is a JSON object where `ok` is `false`, `data` is `null`, and `errors` contains at least one entry with `code`, `message`, `severity`, and optionally `path`

#### Scenario: JSON envelope version field
- **WHEN** any command runs with `--json`
- **THEN** the `version` field is `"0.1"` and `command` matches the subcommand name

### Requirement: Cross-platform binary targets
Patina SHALL be buildable and functional on macOS arm64, macOS x64, Linux x64, and Windows x64. No runtime dependency on Python, Node.js, ChromaDB, or a network connection SHALL be required.

#### Scenario: Build on Linux x64
- **WHEN** `cargo build --release` is run on Linux x64
- **THEN** the resulting binary runs `patina doctor` without error and exits 0

#### Scenario: Build on macOS arm64
- **WHEN** `cargo build --release` is run on macOS arm64
- **THEN** the resulting binary runs `patina doctor` without error and exits 0

#### Scenario: Build on Windows x64
- **WHEN** `cargo build --release` is run on Windows x64
- **THEN** the resulting binary runs `patina doctor` without error and exits 0
