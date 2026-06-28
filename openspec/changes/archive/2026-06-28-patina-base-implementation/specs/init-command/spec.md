## ADDED Requirements

### Requirement: Scaffold knowledge directory structure
`patina init` SHALL create the knowledge directory structure at the path configured in `knowledge.dir` (default `knowledge/`). It SHALL create at minimum: `knowledge/README.md`, `knowledge/AGENTS.md`, `knowledge/wiki/`, `knowledge/sources/`, and `knowledge/schemas/`.

#### Scenario: Fresh init in empty directory
- **WHEN** `patina init` is run in a directory with no `knowledge/` folder
- **THEN** `knowledge/README.md`, `knowledge/AGENTS.md`, `knowledge/wiki/`, `knowledge/sources/`, and `knowledge/schemas/` are created

#### Scenario: Init in directory where knowledge/ already exists
- **WHEN** `patina init` is run and `knowledge/` already exists
- **THEN** Patina does not overwrite existing files, emits a warning that the directory exists, and exits without error

### Requirement: Git worktree detection
`patina init` SHALL detect whether the current directory is inside a Git worktree by checking for a `.git` directory or by invoking `git rev-parse --is-inside-work-tree`.

#### Scenario: Init inside a Git worktree
- **WHEN** `patina init` is run inside a Git repository
- **THEN** it proceeds without a Git warning

#### Scenario: Init outside a Git worktree without --no-git
- **WHEN** `patina init` is run in a directory with no Git repository
- **THEN** it emits a warning that no Git repository was detected and prompts the user to confirm or pass `--no-git`

#### Scenario: Init outside Git with --no-git flag
- **WHEN** `patina init --no-git` is run outside a Git repository
- **THEN** init proceeds without the Git warning

### Requirement: .gitignore entry for .patina/
`patina init` SHALL add `.patina/` to the repository `.gitignore` file if it is not already present. If no `.gitignore` exists, it SHALL create one.

#### Scenario: No .gitignore present
- **WHEN** `patina init` runs and no `.gitignore` file exists
- **THEN** a `.gitignore` file is created containing `.patina/`

#### Scenario: .gitignore exists without .patina/ entry
- **WHEN** `patina init` runs and `.gitignore` does not contain `.patina/`
- **THEN** `.patina/` is appended to the existing `.gitignore`

#### Scenario: .gitignore already contains .patina/
- **WHEN** `patina init` runs and `.gitignore` already contains `.patina/`
- **THEN** no duplicate entry is added

### Requirement: Idempotent initialization
Running `patina init` multiple times on the same repository SHALL be safe and SHALL NOT destroy or overwrite existing knowledge content.

#### Scenario: Second init run on initialized repo
- **WHEN** `patina init` is run on a repository that was previously initialized
- **THEN** existing files are preserved, no content is overwritten, exit code is 0
