## ADDED Requirements

### Requirement: review_after date expiry detection
`patina stale` SHALL compare the current date against the `review_after` front matter field for each indexed document. If the date has passed, the page SHALL be reported as stale with reason code `review_after_passed`.

#### Scenario: review_after date has passed
- **WHEN** a page has `review_after: 2024-01-01` and today is after that date
- **THEN** `patina stale` reports the page as stale with `reason.code = "review_after_passed"`

#### Scenario: review_after date has not passed
- **WHEN** a page has `review_after` set to a future date
- **THEN** the page is not reported as stale for this reason

#### Scenario: No review_after field
- **WHEN** a page has no `review_after` field
- **THEN** no review_after staleness is reported for that page

### Requirement: Source hash drift detection
`patina stale` SHALL compare the current SHA-256 of each source file referenced by a page against the hash stored in `source_refs.source_hash_at_index`. If the hash has changed, the page SHALL be reported as stale with severity `warning` and reason code `source_hash_changed`.

#### Scenario: Source file has changed since indexing
- **WHEN** a referenced source file has been modified after the page was last indexed
- **THEN** `patina stale` reports the page with `reason.code = "source_hash_changed"` and includes the source path

#### Scenario: Source file unchanged
- **WHEN** a referenced source file has not changed since indexing
- **THEN** no source_hash_changed staleness is reported

### Requirement: Missing source reference detection
If a page declares a `source_refs` entry but the referenced file does not exist, `patina stale` SHALL report it as an error (not just a warning) with code `missing_source_ref`.

#### Scenario: Source ref file deleted after indexing
- **WHEN** a source file was present at index time but has since been deleted
- **THEN** `patina stale` reports the page with `reason.code = "missing_source_ref"` and severity `error`

### Requirement: Deprecated page still linked from active pages
`patina stale` SHALL detect when a page with `status: deprecated` is still referenced by internal links from pages with `status: active`. Such pages SHALL be reported as stale with reason code `deprecated_but_linked`.

#### Scenario: Deprecated page linked from active page
- **WHEN** an active page links to a deprecated page
- **THEN** `patina stale` reports the deprecated page with `reason.code = "deprecated_but_linked"`

### Requirement: Draft age threshold
`patina stale` SHALL detect pages with `status: draft` whose `modified_at` or `indexed_at` is older than a configurable threshold (default: 90 days). Such pages SHALL be reported as stale with reason code `draft_too_old`.

#### Scenario: Draft page older than threshold
- **WHEN** a page has `status: draft` and was last modified more than 90 days ago
- **THEN** `patina stale` reports it with `reason.code = "draft_too_old"`

### Requirement: Stale JSON output with reasons
`patina stale --json` SHALL return the standard JSON envelope with `data.stale_pages` as an array. Each entry SHALL include the page path and a `reasons` array where each reason has `code`, `severity`, and optionally `source`.

#### Scenario: Stale results JSON structure
- **WHEN** `patina stale --json` finds stale pages
- **THEN** `data.stale_pages` is an array; each element has `path` and `reasons`; each reason has `code` and `severity`

#### Scenario: No stale pages
- **WHEN** `patina stale --json` finds no stale pages
- **THEN** `ok` is `true` and `data.stale_pages` is an empty array

### Requirement: Stale is warning-only for source hash drift
Source hash drift SHALL produce severity `warning`, not `error`. Missing source references SHALL produce severity `error`.

#### Scenario: Source hash changed produces warning
- **WHEN** a source file has changed since indexing
- **THEN** the stale reason has `severity = "warning"`

#### Scenario: Missing source ref produces error
- **WHEN** a source file referenced by a page does not exist
- **THEN** the stale reason has `severity = "error"`
