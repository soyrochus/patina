## ADDED Requirements

### Requirement: Install-skills command interface
`patina install-skills` SHALL accept `--for <target>` zero or more times, `--force`, and `--json`.

Valid `--for` values SHALL be `github-copilot`, `codex`, `claude-code`, and `all`. The value `all` SHALL be equivalent to selecting `github-copilot`, `codex`, and `claude-code`.

If `--for` is omitted, Patina SHALL write or update only `<knowledge_dir>/AGENTS.md` and SHALL NOT write host-specific skill directories.

#### Scenario: No targets writes shared instructions only
- **WHEN** `patina install-skills` is run without `--for`
- **THEN** `<knowledge_dir>/AGENTS.md` is created or updated
- **AND** no `.github/skills`, `.agents/skills`, or `.claude/skills` Patina skill files are written

#### Scenario: All target expands to every host
- **WHEN** `patina install-skills --for all` is run
- **THEN** GitHub Copilot, Codex, and Claude Code Patina skill files are written

#### Scenario: Repeated target flags are accepted
- **WHEN** `patina install-skills --for github-copilot --for codex` is run
- **THEN** GitHub Copilot and Codex Patina skill files are written

### Requirement: Shared AGENTS instructions
Every `patina install-skills` invocation SHALL ensure `<knowledge_dir>/AGENTS.md` exists. The path `<knowledge_dir>` SHALL resolve from Patina configuration and default to `knowledge` when no config overrides it.

The shared file SHALL contain a Patina-managed section delimited by:

```markdown
<!-- BEGIN PATINA AGENT INSTRUCTIONS -->
<!-- END PATINA AGENT INSTRUCTIONS -->
```

If the file does not exist, Patina SHALL create it. If it exists and contains the managed section, Patina SHALL replace only that section. If it exists without the managed section, Patina SHALL leave existing content intact and append the managed section when `--force` is provided.

The shared section SHALL describe Patina as a local-first Git-compatible Markdown knowledge tool and reference `patina query`, `patina read`, `patina lint`, `patina stale`, and `patina index`.

#### Scenario: Shared instructions use configured knowledge directory
- **WHEN** `patina install-skills` runs with a custom configured knowledge directory
- **THEN** the shared instructions are written under that configured directory

#### Scenario: Managed shared section is replaced
- **WHEN** `<knowledge_dir>/AGENTS.md` already contains the Patina-managed section and `patina install-skills --force` is run
- **THEN** only the Patina-managed section is replaced

### Requirement: Generated skill names and content
Patina SHALL install exactly two initial Agent Skills for each selected host target: `patina-query` and `patina-check`.

Each generated skill SHALL be a directory containing `SKILL.md`. Each `SKILL.md` SHALL include YAML front matter with at minimum `name` and `description`, SHALL include `license: MIT`, SHALL include compatibility text requiring the Patina CLI, and SHALL include the marker `<!-- PATINA GENERATED SKILL -->` near the top of the Markdown body.

Generated skill front matter SHALL NOT include `allowed-tools` by default.

The `patina-query` skill SHALL instruct agents to search with `patina query "<terms>" --json --limit 5`, read relevant pages with `patina read <path> --json`, answer from Git-tracked Markdown content, and avoid citing `.patina/` generated index files.

The `patina-check` skill SHALL instruct agents to run `patina lint --json`, run `patina stale --json`, report errors and warnings, and avoid unrelated knowledge edits while lint errors are present.

All generated skill files SHALL substitute `<knowledge_dir>` with the configured knowledge directory.

#### Scenario: Generated skills have required front matter and marker
- **WHEN** a host target is installed
- **THEN** each generated `SKILL.md` contains valid YAML front matter with `name` and `description`
- **AND** each generated `SKILL.md` contains `<!-- PATINA GENERATED SKILL -->`
- **AND** no generated `SKILL.md` contains `allowed-tools`

#### Scenario: Knowledge directory placeholder is substituted
- **WHEN** `patina install-skills` writes skill files
- **THEN** generated skill content references the actual configured knowledge directory path

### Requirement: GitHub Copilot skills target
When `github-copilot` is selected, Patina SHALL write:

```text
.github/skills/patina-query/SKILL.md
.github/skills/patina-check/SKILL.md
```

Patina SHALL NOT write `.github/prompts/patina-query.prompt.md` or `.github/prompts/patina-check.prompt.md` as part of the default implementation.

#### Scenario: GitHub Copilot target writes skills
- **WHEN** `patina install-skills --for github-copilot` is run
- **THEN** `.github/skills/patina-query/SKILL.md` and `.github/skills/patina-check/SKILL.md` exist
- **AND** `.github/prompts/patina-query.prompt.md` and `.github/prompts/patina-check.prompt.md` do not exist

### Requirement: Codex skills target
When `codex` is selected, Patina SHALL write:

```text
.agents/skills/patina-query/SKILL.md
.agents/skills/patina-check/SKILL.md
AGENTS.md
```

The root `AGENTS.md` content SHALL be supplementary passive context only. It SHALL contain a Patina-managed section delimited by:

```markdown
<!-- BEGIN PATINA CODEX CONTEXT -->
<!-- END PATINA CODEX CONTEXT -->
```

The Codex section SHALL point to `.agents/skills/`, name `patina-query` and `patina-check`, list core Patina commands, and reference `<knowledge_dir>/AGENTS.md`.

#### Scenario: Codex target writes skills and root context
- **WHEN** `patina install-skills --for codex` is run
- **THEN** `.agents/skills/patina-query/SKILL.md` and `.agents/skills/patina-check/SKILL.md` exist
- **AND** root `AGENTS.md` contains the Patina Codex context section

### Requirement: Claude Code skills target
When `claude-code` is selected, Patina SHALL write:

```text
.claude/skills/patina-query/SKILL.md
.claude/skills/patina-check/SKILL.md
```

The generated files SHALL be Agent Skill files and SHALL NOT assert a fixed slash-command syntax.

#### Scenario: Claude Code target writes skills
- **WHEN** `patina install-skills --for claude-code` is run
- **THEN** `.claude/skills/patina-query/SKILL.md` and `.claude/skills/patina-check/SKILL.md` exist

### Requirement: Skill file write policy
For each generated `SKILL.md`, Patina SHALL create the parent directory if missing.

If `SKILL.md` does not exist, Patina SHALL create it. If it exists and contains `<!-- PATINA GENERATED SKILL -->`, Patina SHALL replace the entire file. If it exists and does not contain that marker, Patina SHALL skip it by default, emit a warning with code `skill_file_exists_not_managed`, and overwrite it only when `--force` is provided.

#### Scenario: Managed skill is replaced
- **WHEN** an existing target `SKILL.md` contains `<!-- PATINA GENERATED SKILL -->` and `patina install-skills` is run
- **THEN** the file is replaced with the current generated content

#### Scenario: Non-managed skill is skipped
- **WHEN** an existing target `SKILL.md` does not contain `<!-- PATINA GENERATED SKILL -->` and `patina install-skills` is run without `--force`
- **THEN** the file is not overwritten
- **AND** a warning with code `skill_file_exists_not_managed` is emitted

#### Scenario: Force overwrites non-managed skill
- **WHEN** an existing target `SKILL.md` does not contain `<!-- PATINA GENERATED SKILL -->` and `patina install-skills --force` is run
- **THEN** the file is overwritten

### Requirement: Duplicate skill warning
Patina SHALL emit an informational warning when installing multiple host targets that may expose duplicate skill names to tools that scan several skill roots. The command SHALL still complete.

#### Scenario: Multiple host targets warn about duplicates
- **WHEN** `patina install-skills --for github-copilot --for codex` is run
- **THEN** the command succeeds
- **AND** a warning explains that multiple host installs may create duplicate visible Patina skill names

### Requirement: Install-skills JSON output
`patina install-skills --json` SHALL return the standard Patina JSON envelope. On success, `data.files_written` SHALL list files created or modified, `data.files_skipped` SHALL list files skipped by write policy, and `data.targets` SHALL list normalized selected targets.

Warnings SHALL use standard warning entries and include `path` when a specific file was skipped.

#### Scenario: JSON reports written files
- **WHEN** `patina install-skills --for github-copilot --json` succeeds
- **THEN** `ok` is `true`
- **AND** `command` is `"install-skills"`
- **AND** `data.files_written` includes `<knowledge_dir>/AGENTS.md`, `.github/skills/patina-query/SKILL.md`, and `.github/skills/patina-check/SKILL.md`
- **AND** `data.targets` includes `"github-copilot"`

#### Scenario: JSON reports skipped files
- **WHEN** `patina install-skills --for github-copilot --json` skips a non-managed existing skill file
- **THEN** `ok` is `true`
- **AND** `data.files_skipped` includes the skipped path
- **AND** `warnings` includes code `skill_file_exists_not_managed`
