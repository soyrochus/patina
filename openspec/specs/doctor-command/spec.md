# doctor-command Specification

## Purpose
TBD - created by archiving change patina-base-implementation. Update Purpose after archive.
## Requirements
### Requirement: Comprehensive environment check
`patina doctor` SHALL check the following items and report each as `ok`, `warning`, or `error`:

- Current directory is inside or near a Git worktree
- Knowledge directory exists
- `knowledge/README.md` exists
- `knowledge/AGENTS.md` exists
- `.patina/` exists or can be created
- `.patina/` is ignored by Git
- SQLite database exists (if previously indexed)
- SQLite integrity check passes (`PRAGMA integrity_check`)
- FTS5 is available in the bundled SQLite build
- Local index schema version is supported
- File permissions allow reads and local writes to `.patina/`
- Agent instruction files are present where expected
- `scope.yaml` is valid if present
- Large-file limits are configured

#### Scenario: Healthy environment
- **WHEN** `patina doctor` is run in a properly initialised repository with a valid index
- **THEN** all checks report `ok` and the exit code is 0

#### Scenario: Knowledge directory missing
- **WHEN** `patina doctor` is run and the knowledge directory does not exist
- **THEN** the check for the knowledge directory reports `error` and the exit code is non-zero

#### Scenario: .patina/ not Git-ignored
- **WHEN** `.patina/` is not listed in `.gitignore`
- **THEN** `patina doctor` reports a `warning` for that check

### Requirement: Read-only operation
`patina doctor` SHALL NOT modify any files unless a future `--fix` flag is explicitly implemented and requested. It is purely a diagnostic tool.

#### Scenario: Doctor does not create or modify files
- **WHEN** `patina doctor` is run on an uninitialised repository
- **THEN** no files are created, modified, or deleted; only diagnostic output is produced

### Requirement: Doctor JSON output
`patina doctor --json` SHALL return the standard JSON envelope with `data.checks` as an array. Each check SHALL have `name` (string), `status` (`ok`/`warning`/`error`), and `message` (string).

#### Scenario: Doctor JSON structure
- **WHEN** `patina doctor --json` is run
- **THEN** `data.checks` is an array; each element has `name`, `status`, and `message` fields

#### Scenario: Doctor with errors returns ok false
- **WHEN** any check has status `error`
- **THEN** the JSON envelope `ok` field is `false`

### Requirement: SQLite integrity check
If the SQLite database exists, `patina doctor` SHALL run `PRAGMA integrity_check` and report the result.

#### Scenario: Database passes integrity check
- **WHEN** the SQLite database is intact
- **THEN** the integrity check reports `ok`

#### Scenario: Database fails integrity check
- **WHEN** the SQLite database is corrupt
- **THEN** the integrity check reports `error` with a message suggesting `patina index --reset`

### Requirement: FTS5 availability check
`patina doctor` SHALL verify that the bundled SQLite supports FTS5 and report the result.

#### Scenario: FTS5 available
- **WHEN** FTS5 is available in the SQLite build
- **THEN** the FTS5 check reports `ok`

#### Scenario: FTS5 unavailable
- **WHEN** FTS5 is not available
- **THEN** the FTS5 check reports `warning` explaining that degraded search will be used

