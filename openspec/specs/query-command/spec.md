# query-command Specification

## Purpose

TBD - created by archiving change patina-base-implementation. Update Purpose after archive.

## Requirements

### Requirement: Natural-language query normalization

`patina query` SHALL normalize user query text into meaningful lexical terms before retrieval. Normalization SHALL:

- lowercase the input using Unicode-aware lowercasing where available in the standard library;
- treat punctuation and FTS5 syntax characters as separators;
- split on ASCII whitespace after separator replacement;
- trim empty terms;
- remove stop words;
- remove duplicate terms while preserving first-seen order;
- keep terms shorter than three characters only when they contain a digit or appear in the explicit short-term allow-list.

The initial stop-word list SHALL include at least: `a`, `an`, `and`, `are`, `as`, `at`, `be`, `but`, `by`, `can`, `does`, `for`, `from`, `how`, `if`, `in`, `is`, `it`, `of`, `on`, `or`, `should`, `that`, `the`, `this`, `to`, `use`, `what`, `when`, `where`, `who`, `why`, and `with`.

The initial short-term allow-list SHALL include at least: `ai`, `cli`, `db`, `fts`, `ui`, `v0`, `v1`, `v2`, and `v3`.

If normalization produces no terms, Patina SHALL fall back to the trimmed raw query as a single escaped term. If the trimmed raw query is empty, Patina SHALL return an empty result list with `ok = true`.

#### Scenario: Natural-language query produces lexical terms

- **WHEN** `patina query "why should agents use Patina as durable context" --json --explain` is run
- **THEN** retrieval uses the normalized terms `agents`, `patina`, `durable`, and `context`

#### Scenario: Punctuation is not executable FTS5 syntax

- **WHEN** `patina query "agents: durable OR context?" --json` is run
- **THEN** punctuation and FTS5 syntax characters are treated as separators instead of user-authored FTS5 syntax

#### Scenario: Empty query returns no results successfully

- **WHEN** `patina query "   " --json` is run
- **THEN** `ok` is `true` and `data.results` is an empty array

### Requirement: FTS5 BM25 full-text search

`patina query <terms>` SHALL search the FTS5 index over chunk text and return ranked results. BM25 scores SHALL be normalised to `[0.0, 1.0]` (higher = better) before combining with other scoring components.

FTS5 search SHALL build a safe FTS5 expression from normalized terms, not from the raw user string. Each normalized term SHALL be escaped or quoted so that user input cannot be interpreted as FTS5 operators, column filters, phrase syntax, or prefix syntax.

The default FTS5 expression SHALL require all normalized terms. If that strict all-term search returns no results, Patina SHALL retry within the same query invocation using a relaxed any-term expression. The retry SHALL NOT change the reported mode from `fts5`. Results from strict all-term search SHALL be preferred over results found only by relaxed any-term search; returning strict results without running relaxed search is acceptable.

#### Scenario: Query returns ranked results

- **WHEN** `patina query "controlled autonomy"` is run
- **THEN** results are returned in descending score order with file paths and excerpt text

#### Scenario: No results for query

- **WHEN** the normalized query terms match no chunks in the index
- **THEN** an empty result list is returned and `ok` is `true`

#### Scenario: Natural-language query matches meaningful terms

- **WHEN** `patina query "why should agents use Patina as durable context" --json` is run against an FTS5 index containing a chunk with `agents`, `Patina`, `durable`, and `context`
- **THEN** the chunk is eligible to be returned even though the raw sentence does not appear verbatim

#### Scenario: FTS5 retry keeps mode

- **WHEN** strict all-term FTS5 search returns no rows and relaxed any-term FTS5 search returns rows
- **THEN** `data.mode` remains `"fts5"`

### Requirement: LIKE-based fallback search

If FTS5 is unavailable, `patina query` SHALL fall back to `LIKE`-based search over normalized terms. The JSON output SHALL include `mode = "lexical-fallback"` and a warning SHALL be emitted.

Fallback search SHALL NOT require the full raw user sentence to appear as a single substring. The fallback path SHALL first search for chunks containing all normalized terms using `LIKE` patterns. If the all-term fallback returns no results, Patina SHALL retry with an any-term fallback using the same patterns.

Fallback results SHALL receive a deterministic raw lexical score based on the number of normalized terms matched in the chunk. A chunk matching more normalized terms MUST NOT score lower than a chunk matching fewer normalized terms solely because of fallback scoring.

#### Scenario: Fallback mode is indicated in JSON

- **WHEN** FTS5 is unavailable and `patina query --json` runs
- **THEN** `data.mode` is `"lexical-fallback"` and `warnings` contains the FTS5 unavailability message

#### Scenario: Fallback matches natural-language terms

- **WHEN** FTS5 is unavailable and `patina query "why should agents use Patina as durable context" --json` runs against chunks containing the normalized terms
- **THEN** fallback search does not require the raw sentence to appear as one substring

#### Scenario: Fallback retries with any term

- **WHEN** all-term fallback search returns no rows and any-term fallback search returns rows
- **THEN** `data.mode` is `"lexical-fallback"` and the matching rows are returned

#### Scenario: Fallback term count affects score

- **WHEN** fallback search returns one chunk matching three normalized terms and another chunk matching one normalized term
- **THEN** the three-term chunk does not receive a lower raw fallback score solely because of fallback scoring

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

`title_match_bonus` SHALL be `1.0` when any normalized query term appears in the page title using case-insensitive substring matching, and `0.0` otherwise.

`alias_match_bonus` SHALL be `1.0` when any normalized query term appears in any indexed alias for the page using case-insensitive substring matching, and `0.0` otherwise.

`tag_match_bonus` SHALL be `1.0` when any normalized query term appears in any indexed tag for the page using case-insensitive substring matching, and `0.0` otherwise.

`freshness_bonus` SHALL be computed from the indexed document `modified_at` timestamp using a linear decay from `1.0` to `0.0` over 365 days. Documents modified at the query reference time SHALL receive `1.0`; documents 365 or more days old SHALL receive `0.0`; documents with missing or unparsable `modified_at` values SHALL receive `0.0`.

All results from a single query run SHALL use the same query reference timestamp for freshness scoring.

#### Scenario: Score is between 0 and 1

- **WHEN** any result is returned from `patina query`
- **THEN** its score is a float in the range `[0.0, 1.0]`

#### Scenario: Title match increases score

- **WHEN** `patina query "why is controlled autonomy important" --json --explain` returns a page whose title includes `"Controlled Autonomy"`
- **THEN** that page receives a non-zero `title_match_bonus`

#### Scenario: Alias match increases score

- **WHEN** `patina query "why is controlled autonomy important" --json --explain` returns a page whose indexed aliases include `"controlled autonomy"`
- **THEN** that result's `score_components.alias` is `1.0`

#### Scenario: Alias without match has no bonus

- **WHEN** `patina query "routing" --json --explain` returns a page whose indexed aliases do not contain `"routing"`
- **THEN** that result's `score_components.alias` is `0.0`

#### Scenario: Tag match increases score

- **WHEN** `patina query "why do agents need durable context" --json --explain` returns a page whose indexed tags include `"agents"`
- **THEN** that result's `score_components.tag` is `1.0`

#### Scenario: Fresh document receives full freshness bonus

- **WHEN** `patina query "term" --json --explain` returns a page whose indexed `modified_at` equals the query reference timestamp
- **THEN** that result's `score_components.freshness` is `1.0`

#### Scenario: Old document receives no freshness bonus

- **WHEN** `patina query "term" --json --explain` returns a page whose indexed `modified_at` is 365 or more days before the query reference timestamp
- **THEN** that result's `score_components.freshness` is `0.0`

#### Scenario: Missing modified_at receives no freshness bonus

- **WHEN** `patina query "term" --json --explain` returns a page with no indexed `modified_at`
- **THEN** that result's `score_components.freshness` is `0.0`

### Requirement: --explain flag for score breakdown

`patina query --explain` SHALL include score component details in the JSON output for each result.

When `--explain` is provided, each result's `matches` array SHALL contain the normalized terms used for retrieval, not the unprocessed raw query string.

#### Scenario: --explain returns score components

- **WHEN** `patina query "example" --json --explain` is run
- **THEN** each result in `data.results` includes a `score_components` object with keys `fts`, `title`, `alias`, `tag`, `page_type`, `freshness`, `provenance`, and a `matches` array

#### Scenario: --explain returns normalized matches

- **WHEN** `patina query "why should agents use Patina as durable context" --json --explain` returns results
- **THEN** each result's `matches` array contains `agents`, `patina`, `durable`, and `context`

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
