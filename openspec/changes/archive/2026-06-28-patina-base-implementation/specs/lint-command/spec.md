## ADDED Requirements

### Requirement: Required front matter key validation
`patina lint` SHALL verify that every Markdown page in the knowledge directory contains the default required front matter keys: `title`, `type`, and `status`. Missing keys SHALL be reported as errors.

#### Scenario: Page missing title
- **WHEN** a page's front matter lacks a `title` key
- **THEN** lint reports an error with code `missing_required_field`, the field name, and the file path

#### Scenario: Page has all required fields
- **WHEN** a page's front matter contains `title`, `type`, and `status`
- **THEN** no error is reported for required fields

### Requirement: Allowed status and type value validation
Patina SHALL validate that `status` is one of `draft`, `active`, `deprecated`, `archived` and that `type` is one of the allowed page types (`concept`, `system`, `project`, `decision`, `person`, `process`, `glossary`, `source`, `index`, `open-question`). Unrecognised values SHALL be errors.

#### Scenario: Unknown status value
- **WHEN** a page has `status: unknown-value`
- **THEN** lint reports an error with code `invalid_field_value` and the offending value

#### Scenario: Valid status value
- **WHEN** a page has `status: active`
- **THEN** no status-related error is reported

#### Scenario: Unknown type value
- **WHEN** a page has `type: made-up-type`
- **THEN** lint reports an error with code `invalid_field_value` for the `type` field

### Requirement: Page-type-specific required field rules
If `[lint.page_types.<type>]` is configured with a `required` list, Patina SHALL enforce those additional required fields for pages of that type.

#### Scenario: decision page missing decided_on
- **WHEN** `[lint.page_types.decision]` requires `["title","type","status","decided_on"]` and a decision page lacks `decided_on`
- **THEN** lint reports a missing required field error for `decided_on`

#### Scenario: No page-type rule configured — default fields apply
- **WHEN** a page type has no specific rule configured
- **THEN** only the default required fields (`title`, `type`, `status`) are enforced

### Requirement: Internal link validation
`patina lint` SHALL check that all internal Markdown links (`[[wikilinks]]` or `[text](relative/path.md)`) resolve to existing files within the knowledge root.

#### Scenario: Link to existing file
- **WHEN** a page contains a link to a file that exists in the knowledge directory
- **THEN** no link error is reported

#### Scenario: Link to missing file
- **WHEN** a page contains a link to a file that does not exist
- **THEN** lint reports an error with code `broken_link` and the target path

### Requirement: Alias uniqueness check
`patina lint` SHALL detect duplicate `aliases` values across all pages. Duplicate aliases SHALL be reported as errors with both conflicting file paths.

#### Scenario: Two pages share an alias
- **WHEN** two different pages both declare the same value in their `aliases` front matter field
- **THEN** lint reports an error with code `duplicate_alias` listing both file paths

#### Scenario: Unique aliases across all pages
- **WHEN** all alias values are unique across the knowledge directory
- **THEN** no alias-related error is reported

### Requirement: Source reference existence check
`patina lint` SHALL check that all paths listed in a page's `source_refs` front matter field exist on the filesystem. Missing source references SHALL be errors.

#### Scenario: Source reference to existing file
- **WHEN** a page declares a source reference that exists
- **THEN** no source reference error is reported

#### Scenario: Source reference to missing file
- **WHEN** a page declares a source reference to a file that does not exist
- **THEN** lint reports an error with code `missing_source_ref` and the missing path

### Requirement: Symlink warning during lint
`patina lint` SHALL emit a warning when it encounters a symlink within the knowledge directory during file walking.

#### Scenario: Symlink encountered during lint
- **WHEN** a symlink exists in the knowledge directory
- **THEN** lint emits a warning with the symlink path and continues

### Requirement: Lint JSON output
`patina lint --json` SHALL return the standard JSON envelope. Errors and warnings SHALL be in the `errors` and `warnings` arrays. The `ok` field SHALL be `false` if any errors were found.

#### Scenario: Lint finds errors with --json
- **WHEN** `patina lint --json` is run and errors are found
- **THEN** `ok` is `false` and `errors` contains entries with `code`, `message`, `path`, and `severity`

#### Scenario: Clean lint with --json
- **WHEN** `patina lint --json` finds no errors or warnings
- **THEN** `ok` is `true`, `errors` is `[]`, `warnings` is `[]`
