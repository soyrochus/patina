## MODIFIED Requirements

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

`alias_match_bonus` SHALL be `1.0` when any indexed alias for the page contains the query string using case-insensitive substring matching, and `0.0` otherwise.

`tag_match_bonus` SHALL be `1.0` when any indexed tag for the page contains the query string using case-insensitive substring matching, and `0.0` otherwise.

`freshness_bonus` SHALL be computed from the indexed document `modified_at` timestamp using a linear decay from `1.0` to `0.0` over 365 days. Documents modified at the query reference time SHALL receive `1.0`; documents 365 or more days old SHALL receive `0.0`; documents with missing or unparsable `modified_at` values SHALL receive `0.0`.

All results from a single query run SHALL use the same query reference timestamp for freshness scoring.

#### Scenario: Score is between 0 and 1
- **WHEN** any result is returned from `patina query`
- **THEN** its score is a float in the range `[0.0, 1.0]`

#### Scenario: Title match increases score
- **WHEN** a query term exactly matches a page's title
- **THEN** that page receives a non-zero `title_match_bonus`

#### Scenario: Alias match increases score
- **WHEN** `patina query "autonomy" --json --explain` returns a page whose indexed aliases include `"controlled autonomy"`
- **THEN** that result's `score_components.alias` is `1.0`

#### Scenario: Alias without match has no bonus
- **WHEN** `patina query "routing" --json --explain` returns a page whose indexed aliases do not contain `"routing"`
- **THEN** that result's `score_components.alias` is `0.0`

#### Scenario: Tag match increases score
- **WHEN** `patina query "agents" --json --explain` returns a page whose indexed tags include `"agents"`
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
