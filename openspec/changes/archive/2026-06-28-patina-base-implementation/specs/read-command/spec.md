## ADDED Requirements

### Requirement: Path canonicalization and knowledge root boundary enforcement
`patina read <path>` SHALL canonicalize the requested path using the OS path resolution and verify it remains inside the configured knowledge root. Paths that resolve outside the root SHALL be rejected with an error.

#### Scenario: Valid path inside knowledge root
- **WHEN** `patina read knowledge/wiki/concepts/foo.md` is run and the file exists within the knowledge root
- **THEN** the file contents are printed to stdout

#### Scenario: Path traversal attempt
- **WHEN** `patina read knowledge/../../etc/passwd` is run
- **THEN** the command exits with an error `path traversal rejected: path resolves outside the knowledge root` and does not read the file

#### Scenario: Absolute path outside knowledge root
- **WHEN** `patina read /etc/hosts` is run
- **THEN** the command exits with a path boundary error

### Requirement: Symlink policy enforcement
`patina read` SHALL reject symlinks that resolve outside the knowledge root. Internal symlinks (resolving within the root) SHALL be accepted only if `security.allow_internal_symlinks = true`. External symlinks SHALL always be rejected.

#### Scenario: Symlink resolving outside root is rejected
- **WHEN** a file at `knowledge/link.md` is a symlink resolving to `/tmp/secret.txt`
- **THEN** `patina read knowledge/link.md` exits with a symlink rejection error

#### Scenario: Internal symlink with allow_internal_symlinks = true
- **WHEN** a symlink inside the knowledge root is resolved and `security.allow_internal_symlinks = true`
- **THEN** `patina read` follows the symlink and returns the file contents

#### Scenario: Internal symlink with allow_internal_symlinks = false (default)
- **WHEN** a symlink inside the knowledge root is encountered and `security.allow_internal_symlinks = false`
- **THEN** `patina read` rejects the symlink with a clear error

### Requirement: File not found error
If the requested file does not exist, `patina read` SHALL exit with a clear error.

#### Scenario: Non-existent file
- **WHEN** `patina read knowledge/wiki/does-not-exist.md` is run
- **THEN** the command exits with an error indicating the file was not found

### Requirement: Read JSON output
`patina read --json <path>` SHALL return the standard JSON envelope with the file content in `data.content` and front matter fields in `data.front_matter`.

#### Scenario: Read with --json
- **WHEN** `patina read --json knowledge/wiki/concepts/foo.md` succeeds
- **THEN** `ok` is `true`, `data.content` is the raw Markdown text, `data.front_matter` is the parsed front matter object
