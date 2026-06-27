## ADDED Requirements

### Requirement: Git-aware Markdown file walking
Patina SHALL walk the knowledge directory using the `ignore` crate, which respects `.gitignore`, `.ignore`, and global Git exclusion patterns. Only files tracked or trackable by Git SHALL be considered for indexing and linting by default.

#### Scenario: File excluded by .gitignore is not indexed
- **WHEN** a Markdown file is listed in `.gitignore`
- **THEN** `patina index` does not include it in the index

#### Scenario: All Markdown files in knowledge/ are discovered
- **WHEN** `patina index` runs on a knowledge directory with nested subdirectories
- **THEN** all `.md` files not excluded by ignore rules are discovered recursively

### Requirement: YAML front matter extraction
Patina SHALL detect and extract YAML front matter delimited by `---` at the start of a Markdown file. The front matter SHALL be parsed into key-value pairs using a YAML parser.

#### Scenario: Valid front matter is extracted
- **WHEN** a Markdown file begins with `---\ntitle: Foo\ntype: concept\nstatus: active\n---`
- **THEN** `title`, `type`, and `status` are extracted and available for lint and index operations

#### Scenario: File with no front matter
- **WHEN** a Markdown file has no `---` delimiter at the start
- **THEN** front matter is treated as empty; lint reports a missing front matter error

#### Scenario: Invalid YAML in front matter
- **WHEN** a Markdown file has `---` delimiters but invalid YAML between them
- **THEN** a lint error is emitted with code `invalid_front_matter` and the file path

### Requirement: Large file skip with warning
If a Markdown file exceeds `limits.max_markdown_file_mb` (default 10 MB), Patina SHALL skip the file and emit a warning with the file path and size.

#### Scenario: Markdown file exceeds size limit
- **WHEN** a Markdown file is larger than `max_markdown_file_mb`
- **THEN** the file is skipped with a warning; no error; other files are processed

### Requirement: Total file count warning
If the number of Markdown files in the knowledge directory exceeds `limits.max_total_markdown_files` (default 50000), Patina SHALL emit a warning that performance may degrade. It SHALL NOT fail solely due to file count.

#### Scenario: File count exceeds warning threshold
- **WHEN** the knowledge directory contains more than `max_total_markdown_files` Markdown files
- **THEN** a warning is emitted and indexing continues

### Requirement: scope.yaml parsing
If `knowledge/scope.yaml` is present, Patina SHALL parse it and store the scope metadata. It SHALL warn if the YAML is malformed. Its presence is optional; absence is not an error.

#### Scenario: Valid scope.yaml is parsed
- **WHEN** `knowledge/scope.yaml` contains valid scope metadata
- **THEN** `patina doctor` and `patina status` display the scope and client fields

#### Scenario: Absent scope.yaml
- **WHEN** no `knowledge/scope.yaml` exists
- **THEN** no error or warning is emitted; scope is treated as undefined

#### Scenario: Malformed scope.yaml
- **WHEN** `knowledge/scope.yaml` contains invalid YAML
- **THEN** a warning is emitted with the file path; Patina continues without scope metadata

### Requirement: Scope-based provenance warnings
When a page references a source file whose path root has a different (or stricter) scope than the current knowledge root's scope, Patina SHALL emit a deterministic warning.

#### Scenario: Source reference outside knowledge root
- **WHEN** a page's `source_refs` field points to a path outside the knowledge root
- **THEN** a warning is emitted stating the reference is outside the current knowledge root

#### Scenario: Confidentiality classification change detected
- **WHEN** a page's scope classification has changed compared to the indexed version
- **THEN** a warning is emitted noting the classification change
