## ADDED Requirements

### Requirement: FTS5 BM25 full-text search
`patina query <terms>` SHALL search the FTS5 index over chunk text and return ranked results. BM25 scores SHALL be normalised to `[0.0, 1.0]` (higher = better) before combining with other scoring components.

#### Scenario: Query returns ranked results
- **WHEN** `patina query "controlled autonomy"` is run
- **THEN** results are returned in descending score order with file paths and excerpt text

#### Scenario: No results for query
- **WHEN** the query terms match no chunks in the index
- **THEN** an empty result list is returned and `ok` is `true`

### Requirement: LIKE-based fallback search
If FTS5 is unavailable, `patina query` SHALL fall back to `LIKE`-based search with `WHERE lower(text) LIKE lower('%term%')`. The JSON output SHALL include `mode = "lexical-fallback"` and a warning SHALL be emitted.

#### Scenario: Fallback mode is indicated in JSON
- **WHEN** FTS5 is unavailable and `patina query --json` runs
- **THEN** `data.mode` is `"lexical-fallback"` and `warnings` contains the FTS5 unavailability message

### Requirement: Transparent weighted scoring
`patina query` SHALL compute a score for each result using the following weighted formula:

```
score =
  normalized_fts_score * 0.70
+ title_match_bonus    * 0.10
+ alias_match_bonus    * 0.07
+ tag_match_bonus      * 0.05
+ page_type_bonus      * 0.03
+ freshness_bonus      * 0.03
+ provenance_bonus     * 0.02
```

All components SHALL be normalised to `[0.0, 1.0]`.

#### Scenario: Score is between 0 and 1
- **WHEN** any result is returned from `patina query`
- **THEN** its score is a float in the range `[0.0, 1.0]`

#### Scenario: Title match increases score
- **WHEN** a query term exactly matches a page's title
- **THEN** that page receives a non-zero `title_match_bonus`

### Requirement: --explain flag for score breakdown
`patina query --explain` SHALL include score component details in the JSON output for each result.

#### Scenario: --explain returns score components
- **WHEN** `patina query "example" --json --explain` is run
- **THEN** each result in `data.results` includes a `score_components` object with keys `fts`, `title`, `alias`, `tag`, `page_type`, `freshness`, `provenance`, and a `matches` array

### Requirement: Query JSON output envelope
`patina query --json` SHALL return the standard JSON envelope. Results SHALL be in `data.results` as an array of objects with at minimum `path`, `score`, and `excerpt` fields.

#### Scenario: Query JSON structure
- **WHEN** `patina query "term" --json` returns results
- **THEN** `data.results` is an array; each element has `path` (string), `score` (float), `excerpt` (string)

### Requirement: Result limit
`patina query` SHALL accept a `--limit N` flag (default 10) that caps the number of results returned.

#### Scenario: Default limit of 10 results
- **WHEN** `patina query "term"` matches more than 10 chunks
- **THEN** at most 10 results are returned

#### Scenario: --limit flag overrides default
- **WHEN** `patina query "term" --limit 25` is run
- **THEN** at most 25 results are returned

### Requirement: patina status command
`patina status` SHALL report the current repository state including: Git worktree detected (yes/no), uncommitted knowledge changes (yes/no), `.patina/` ignored by Git (yes/no), index last built timestamp, and scope metadata if `scope.yaml` is present.

#### Scenario: Status in a clean Git repo
- **WHEN** `patina status` is run in a Git repository with no uncommitted knowledge changes
- **THEN** output includes `Git worktree detected: yes` and `Uncommitted knowledge changes: no`

#### Scenario: Status warns if .patina/ is not gitignored
- **WHEN** `.patina/` is not listed in `.gitignore`
- **THEN** `patina status` warns that `.patina/` should be Git-ignored

#### Scenario: Status warns if .patina/ is staged
- **WHEN** any file under `.patina/` is staged for commit
- **THEN** `patina status` emits a warning that local index files should not be committed
