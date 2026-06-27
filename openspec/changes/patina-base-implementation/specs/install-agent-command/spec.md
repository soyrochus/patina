## ADDED Requirements

### Requirement: Generic agent instruction file generation
`patina install-agent` SHALL write a generic agent instruction file to `knowledge/AGENTS.md` if it does not exist. If it already exists, Patina SHALL not overwrite it unless `--force` is passed.

#### Scenario: AGENTS.md does not exist
- **WHEN** `patina install-agent` is run and `knowledge/AGENTS.md` does not exist
- **THEN** `knowledge/AGENTS.md` is created with generic agent instructions for using Patina commands

#### Scenario: AGENTS.md already exists
- **WHEN** `patina install-agent` is run and `knowledge/AGENTS.md` already exists
- **THEN** the existing file is not overwritten; a message is displayed indicating the file was skipped

#### Scenario: AGENTS.md already exists with --force
- **WHEN** `patina install-agent --force` is run
- **THEN** `knowledge/AGENTS.md` is overwritten with fresh generic instructions

### Requirement: Agent instructions are thin CLI wrappers
The generated agent instruction content SHALL describe how to use `patina` CLI commands (`patina query`, `patina read`, `patina lint`, `patina stale`) rather than defining agent-specific protocols. The CLI contract comes before any MCP adapter.

#### Scenario: Generated AGENTS.md references CLI commands
- **WHEN** `patina install-agent` creates `knowledge/AGENTS.md`
- **THEN** the file contents reference `patina query`, `patina read`, and at least one other CLI command

### Requirement: Tool-specific instruction target
`patina install-agent --agent <name>` SHALL write instruction files to the appropriate location for the named agent tool (e.g., a Claude Code-specific file at `.claude/CLAUDE.md`). If the agent type is not recognised, Patina SHALL emit an error.

#### Scenario: Known agent target
- **WHEN** `patina install-agent --agent claude-code` is run
- **THEN** a Claude Code-specific instruction snippet is written to `.claude/CLAUDE.md` or appended if the file exists

#### Scenario: Unknown agent type
- **WHEN** `patina install-agent --agent unknown-tool` is run
- **THEN** an error is emitted listing supported agent types

### Requirement: Install-agent JSON output
`patina install-agent --json` SHALL return the standard JSON envelope with `data.files_written` listing the paths of any files created or modified.

#### Scenario: Install-agent JSON reports written files
- **WHEN** `patina install-agent --json` succeeds
- **THEN** `ok` is `true` and `data.files_written` is an array of the paths created
