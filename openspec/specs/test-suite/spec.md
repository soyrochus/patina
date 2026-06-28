# test-suite Specification

## Purpose
TBD - created by archiving change patina-base-implementation. Update Purpose after archive.
## Requirements
### Requirement: Fixture knowledge repository
Patina SHALL include a small fixture knowledge repository under `tests/fixtures/` or `tests/testdata/`. The fixture SHALL cover the following scenarios: valid knowledge directory, missing front matter, broken internal link, missing source reference, stale source hash, duplicate alias, large file warning trigger, and a query returning expected ranked results.

#### Scenario: Fixture directory exists in source tree
- **WHEN** the repository is cloned
- **THEN** `tests/fixtures/` (or equivalent) exists and contains at least one valid Markdown page and one page with missing front matter

### Requirement: Unit tests
Patina SHALL include unit tests for all discrete logic components: front matter parser, heading-aware chunker, token estimator, scoring formula components, path canonicalization, and JSON envelope serialization.

#### Scenario: Chunker unit test
- **WHEN** the chunker is given a Markdown document with known heading structure
- **THEN** the returned chunks match the expected count, heading paths, and SHA-256 values

#### Scenario: Token estimator unit test
- **WHEN** the token estimator is given a string of exactly 400 characters
- **THEN** it returns 100

#### Scenario: Path canonicalization unit test
- **WHEN** path canonicalization is called with `knowledge/../../etc/passwd` and root `knowledge/`
- **THEN** it returns an error indicating path traversal

### Requirement: Integration tests
Patina SHALL include integration tests that execute CLI commands against the fixture knowledge repository and verify exit codes and output structure.

#### Scenario: patina lint on fixture with errors
- **WHEN** `patina lint --json` is run against the fixture directory containing a page with missing front matter
- **THEN** exit code is non-zero, `ok` is `false`, and `errors` contains a `missing_required_field` entry

#### Scenario: patina index on clean fixture
- **WHEN** `patina index --full --json` is run against the valid fixture directory
- **THEN** exit code is 0, `ok` is `true`, and `data` contains a non-zero file count

### Requirement: Golden output tests
Patina SHALL include golden tests that compare the text and JSON output of core commands against checked-in expected output files. Golden output files SHALL be stored in `tests/golden/`.

#### Scenario: patina init golden output
- **WHEN** `patina init` is run in a clean temp directory
- **THEN** the stdout output matches the golden file for `init`

#### Scenario: patina query golden JSON output
- **WHEN** `patina query "example term" --json` is run against the fixture index
- **THEN** the JSON structure (excluding dynamic timestamps) matches the golden file

### Requirement: JSON envelope tests
Every command that supports `--json` SHALL have a test verifying the envelope structure contains `version`, `command`, `ok`, `data`, `warnings`, and `errors`.

#### Scenario: JSON envelope structure test for lint
- **WHEN** `patina lint --json` is run
- **THEN** the output can be deserialized and all six top-level keys are present

### Requirement: Path safety tests
Patina SHALL include tests verifying that path traversal and symlink policy violations are rejected by `patina read`.

#### Scenario: Path traversal rejection test
- **WHEN** `patina read "knowledge/../../etc/passwd"` is called in a test
- **THEN** the command exits with a non-zero code and does not read the file

#### Scenario: External symlink rejection test
- **WHEN** a symlink resolving outside the knowledge root is passed to `patina read`
- **THEN** the command exits with a symlink rejection error

### Requirement: SQLite migration tests
Patina SHALL include tests verifying that an unsupported schema version causes a descriptive error and that a fresh index has the correct schema version.

#### Scenario: Unsupported schema version test
- **WHEN** the `meta` table contains an unrecognised `schema_version`
- **THEN** any Patina command that reads the index exits with an error suggesting `patina index --reset`

#### Scenario: Fresh index has correct schema version
- **WHEN** `patina index --reset` creates a new database
- **THEN** `SELECT value FROM meta WHERE key = 'schema_version'` returns `"1"`

### Requirement: Query ranking tests
Patina SHALL include tests verifying that pages with title matches rank above pages with only body matches for the same query term.

#### Scenario: Title match outranks body match
- **WHEN** `patina query "controlled autonomy"` is run against an index containing one page titled "Controlled Autonomy" and another that mentions the term only in body text
- **THEN** the title-matching page appears first in the results

### Requirement: Cross-platform path tests
Patina SHALL include tests for path handling that verify correct behaviour on Windows-style paths (backslash separators) and Unix paths.

#### Scenario: Windows-style path handling
- **WHEN** a path using backslash separators is passed to path canonicalization on Windows
- **THEN** it is correctly resolved and boundary-checked

