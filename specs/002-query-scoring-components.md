# Patina — Query Scoring Components: alias, tag, and freshness

## 1. Status

Pending implementation. The weighted scoring formula was fully specified in
`specs/001-base-implementation.md` §9. The infrastructure (weights, envelope,
`--explain` flag) is in place. Three of the seven components are currently
returning placeholder values instead of computed scores.

## 2. Why

The scoring formula in `src/query/scorer.rs` defines seven weighted components:

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

`alias`, `tag`, and `freshness` are the three components returning stub values:

| Component | Weight | Current value | Correct behaviour |
|---|---|---|---|
| `alias_match_bonus` | 0.07 | `0.0` always | 1.0 if any page alias contains the query |
| `tag_match_bonus` | 0.05 | `0.0` always | 1.0 if any page tag contains the query |
| `freshness_bonus` | 0.03 | `0.5` always | linear decay from 1.0 to 0.0 over 365 days |

These three components together carry 15 % of the total score weight. Because
`alias` and `tag` are permanently zero, pages that are the canonical target of
a query (their alias *is* the search term) rank identically to pages that only
happen to mention the term in body text. Because `freshness` is a fixed 0.5,
all pages are treated as equally stale, defeating the purpose of the signal.

The root cause is that `aliases` and `tags` are never persisted to the SQLite
index — they are used only at lint time — and `modified_at` is stored in the
`documents` table but is not fetched by the query path.

## 3. Constraints

- The fix must remain deterministic: the same index state and query must always
  produce the same scores.
- `freshness_bonus` must use a fixed "now" value passed into the scorer
  function, not `Utc::now()` called inside the scorer. This keeps unit tests
  reproducible.
- Aliases and tags must be stored as JSON-encoded arrays in TEXT columns.
  SQLite has no native array type; JSON strings are the established pattern
  already used elsewhere in the Patina ecosystem.
- Adding `aliases` and `tags` columns to `documents` is a schema change.
  `SCHEMA_VERSION` must be incremented from `"1"` to `"2"`. The index is
  explicitly disposable; users must run `patina index --reset` after upgrading.
- The `serde_json` crate is already in `Cargo.toml`; no new dependency is
  needed.
- `chrono` is already in `Cargo.toml`; date arithmetic for freshness requires
  no new dependency.

## 4. Front matter field conventions

The YAML front matter fields that drive these components are:

```yaml
aliases:
  - controlled autonomy
  - agent autonomy
tags:
  - agents
  - architecture
```

Both are YAML sequences. They may be absent; absence is treated as an empty
list, not an error. At index time, each sequence is serialized to a JSON array
string for storage (e.g. `["controlled autonomy","agent autonomy"]`). At query
time the JSON string is parsed back to a `Vec<String>` in the scorer.

The `modified_at` field is already populated at index time from the file system
modification timestamp (`fs::metadata(path)?.modified()?`) and stored as an
RFC 3339 string. No change to the indexing of `modified_at` is required.

## 5. Schema change

**File: `src/db/schema.rs`**

Add two columns to `CREATE_DOCUMENTS_TABLE`:

```sql
aliases TEXT,   -- JSON array, e.g. '["alias one","alias two"]'
tags    TEXT,   -- JSON array, e.g. '["tag-a","tag-b"]'
```

Full updated DDL:

```sql
CREATE TABLE IF NOT EXISTS documents (
    id                  INTEGER PRIMARY KEY,
    path                TEXT NOT NULL UNIQUE,
    title               TEXT,
    type                TEXT,
    status              TEXT,
    sha256              TEXT NOT NULL,
    modified_at         TEXT,
    indexed_at          TEXT,
    front_matter_updated TEXT,
    review_after        TEXT,
    scope_classification TEXT,
    aliases             TEXT,
    tags                TEXT
);
```

Bump the schema version constant:

```rust
pub const SCHEMA_VERSION: &str = "2";
```

No migration path is needed. The index is disposable. Any command that opens a
v1 database will hit the schema version check in `src/db/init.rs` and fail
with:

```
error: index schema version "1" is not supported by this version of Patina
Suggested action: run patina index --reset
```

## 6. Index-time changes

### 6.1 `src/db/documents.rs`

Add two fields to `DocumentRecord`:

```rust
pub aliases: Option<String>,  // JSON-serialized Vec<String>
pub tags:    Option<String>,  // JSON-serialized Vec<String>
```

Update the `upsert` INSERT and ON CONFLICT SET clause to include `aliases` and
`tags` in exactly the same pattern as the existing fields. The column order in
the INSERT must match the VALUES positional parameters — append `aliases` and
`tags` at the end of both the column list and the params list.

### 6.2 `src/cli/index.rs`

The `build_full_index` function constructs a `DocumentRecord` from each parsed
file. Add two helpers alongside the existing `string_field` function:

```rust
fn yaml_sequence_as_json(
    front_matter: &BTreeMap<String, Value>,
    field: &str,
) -> Option<String> {
    front_matter
        .get(field)
        .and_then(Value::as_sequence)
        .map(|seq| {
            let strings: Vec<&str> = seq
                .iter()
                .filter_map(Value::as_str)
                .collect();
            serde_json::to_string(&strings).unwrap_or_else(|_| "[]".to_string())
        })
}
```

Use this helper when building `DocumentRecord`:

```rust
let record = documents::DocumentRecord {
    // ... existing fields unchanged ...
    aliases: yaml_sequence_as_json(&parsed.front_matter, "aliases"),
    tags:    yaml_sequence_as_json(&parsed.front_matter, "tags"),
};
```

If the field is absent or contains no string values, `yaml_sequence_as_json`
returns `None`. Storing `None` rather than `Some("[]")` is intentional: `None`
maps to SQL NULL, which is cheaper to store and easy to distinguish from an
explicit empty list.

## 7. Query-time changes

### 7.1 `src/query/fts.rs`

`RawResult` must carry the three new fields needed by the scorer:

```rust
pub struct RawResult {
    pub path:                String,
    pub title:               Option<String>,
    pub page_type:           Option<String>,
    pub scope_classification: Option<String>,
    pub aliases:             Option<String>,   // NEW
    pub tags:                Option<String>,   // NEW
    pub modified_at:         Option<String>,   // NEW
    pub excerpt:             String,
    pub raw_score:           f64,
}
```

Update the SELECT in `search()` to fetch these columns from the `documents`
join. Append `d.aliases`, `d.tags`, `d.modified_at` after `d.scope_classification`:

```sql
SELECT
    d.path,
    d.title,
    d.type,
    d.scope_classification,
    d.aliases,
    d.tags,
    d.modified_at,
    c.text,
    bm25(chunks_fts) AS score
FROM chunks_fts
JOIN chunks c    ON c.id = chunks_fts.rowid
JOIN documents d ON d.id = c.document_id
WHERE chunks_fts MATCH ?1
ORDER BY score
LIMIT ?2
```

Update the `query_map` closure to read the new columns by positional index
(4 = aliases, 5 = tags, 6 = modified_at, 7 = text, 8 = score).

### 7.2 `src/query/fallback.rs`

Apply the identical `RawResult` field additions and SELECT column additions as
in `fts.rs`. Because `fallback.rs` reuses `RawResult` from `fts.rs` via
`use crate::query::fts::RawResult`, only the SELECT and `query_map` closure
need updating — the struct change in `fts.rs` is automatically picked up.

```sql
SELECT
    d.path,
    d.title,
    d.type,
    d.scope_classification,
    d.aliases,
    d.tags,
    d.modified_at,
    c.text
FROM chunks c
JOIN documents d ON d.id = c.document_id
WHERE lower(c.text) LIKE lower(?1)
LIMIT ?2
```

Positional indices: 0=path, 1=title, 2=type, 3=scope_classification,
4=aliases, 5=tags, 6=modified_at, 7=text. `raw_score` stays hardcoded `1.0`
for fallback results.

### 7.3 `src/cli/query.rs`

Update the call to `scorer::score_components()` to pass the three new fields
from the raw result and a `now` timestamp:

```rust
use chrono::Utc;

// computed once before the map, not inside it
let now = Utc::now();

let mut results = raw_results
    .into_iter()
    .zip(normalized)
    .map(|(raw, normalized_fts)| {
        let components = scorer::score_components(
            &args.terms,
            normalized_fts,
            raw.title.as_deref(),
            raw.page_type.as_deref(),
            raw.scope_classification.as_deref(),
            raw.aliases.as_deref(),       // NEW
            raw.tags.as_deref(),          // NEW
            raw.modified_at.as_deref(),   // NEW
            now,                          // NEW
        );
        // ... rest unchanged
    })
    .collect::<Vec<_>>();
```

### 7.4 `src/query/scorer.rs`

Update the `score_components` signature and implement the three bonuses.

**New signature:**

```rust
use chrono::{DateTime, Utc};

pub fn score_components(
    query: &str,
    normalized_fts: f64,
    title: Option<&str>,
    page_type: Option<&str>,
    scope_classification: Option<&str>,
    aliases: Option<&str>,       // JSON array string or None
    tags: Option<&str>,          // JSON array string or None
    modified_at: Option<&str>,   // RFC 3339 string or None
    now: DateTime<Utc>,
) -> ScoreComponents {
```

**`alias_match_bonus` — term-in-alias containment:**

```rust
let alias_bonus = aliases
    .and_then(|json| serde_json::from_str::<Vec<String>>(json).ok())
    .map(|list| {
        let q = query.to_lowercase();
        list.iter().any(|a| a.to_lowercase().contains(&q)) as u8 as f64
    })
    .unwrap_or(0.0);
```

Returns 1.0 if any alias contains the query string (case-insensitive
substring), 0.0 otherwise. A substring match is used rather than exact match
so that multi-word queries partially match compound aliases.

**`tag_match_bonus` — term-in-tag containment:**

```rust
let tag_bonus = tags
    .and_then(|json| serde_json::from_str::<Vec<String>>(json).ok())
    .map(|list| {
        let q = query.to_lowercase();
        list.iter().any(|t| t.to_lowercase().contains(&q)) as u8 as f64
    })
    .unwrap_or(0.0);
```

Identical pattern to alias. Returns 1.0 if any tag contains the query string,
0.0 otherwise.

**`freshness_bonus` — linear decay over 365 days:**

```rust
let freshness_bonus = modified_at
    .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
    .map(|dt| {
        let age_days = (now - dt.with_timezone(&Utc)).num_days().max(0);
        let decay_window = 365_i64;
        if age_days >= decay_window {
            0.0
        } else {
            (decay_window - age_days) as f64 / decay_window as f64
        }
    })
    .unwrap_or(0.0);
```

- A document modified today (`age_days = 0`) gets `freshness_bonus = 1.0`.
- A document modified 182 days ago gets approximately `0.50`.
- A document modified 365 or more days ago gets `0.0`.
- A document with no `modified_at` (NULL) gets `0.0` (conservative fallback).

`now` is passed in from the call site rather than computed inside the scorer so
that all results in a single query run share the same reference point and unit
tests can control time without mocking.

**Updated return value:**

```rust
ScoreComponents {
    fts:        normalized_fts.clamp(0.0, 1.0),
    title:      title_bonus,
    alias:      alias_bonus,
    tag:        tag_bonus,
    page_type:  page_type_bonus,
    freshness:  freshness_bonus,
    provenance: provenance_bonus,
}
```

## 8. Unit tests to add or update

All tests live in `src/query/scorer.rs` under `#[cfg(test)]`.

**Update the weights-sum-to-one test** — it already passes; no change needed
since the weights in `combined()` are not changing.

**Add: alias bonus fires on substring match**

```rust
#[test]
fn alias_bonus_matches_substring() {
    let now = Utc::now();
    let c = score_components(
        "autonomy",
        1.0,
        None,
        None,
        None,
        Some(r#"["controlled autonomy","agent loop"]"#),
        None,
        None,
        now,
    );
    assert_eq!(c.alias, 1.0);
}
```

**Add: alias bonus is zero when no alias matches**

```rust
#[test]
fn alias_bonus_no_match() {
    let now = Utc::now();
    let c = score_components("routing", 1.0, None, None, None,
        Some(r#"["controlled autonomy"]"#), None, None, now);
    assert_eq!(c.alias, 0.0);
}
```

**Add: tag bonus fires when tag contains query**

```rust
#[test]
fn tag_bonus_matches() {
    let now = Utc::now();
    let c = score_components("agents", 1.0, None, None, None,
        None, Some(r#"["agents","architecture"]"#), None, now);
    assert_eq!(c.tag, 1.0);
}
```

**Add: freshness is 1.0 for a document modified today**

```rust
#[test]
fn freshness_full_for_today() {
    let now = Utc::now();
    let modified = now.to_rfc3339();
    let c = score_components("x", 1.0, None, None, None, None, None,
        Some(&modified), now);
    assert!((c.freshness - 1.0).abs() < 0.01);
}
```

**Add: freshness is 0.0 for a document modified 400 days ago**

```rust
#[test]
fn freshness_zero_for_old_document() {
    use chrono::Duration;
    let now = Utc::now();
    let modified = (now - Duration::days(400)).to_rfc3339();
    let c = score_components("x", 1.0, None, None, None, None, None,
        Some(&modified), now);
    assert_eq!(c.freshness, 0.0);
}
```

**Add: freshness is 0.0 when modified_at is None**

```rust
#[test]
fn freshness_zero_when_absent() {
    let now = Utc::now();
    let c = score_components("x", 1.0, None, None, None, None, None,
        None, now);
    assert_eq!(c.freshness, 0.0);
}
```

## 9. Files changed — summary

| File | Change |
|---|---|
| `src/db/schema.rs` | Add `aliases TEXT`, `tags TEXT` columns; bump `SCHEMA_VERSION` to `"2"` |
| `src/db/documents.rs` | Add `aliases` and `tags` fields to `DocumentRecord`; update `upsert()` SQL |
| `src/cli/index.rs` | Add `yaml_sequence_as_json()` helper; populate `aliases` and `tags` in `DocumentRecord` |
| `src/query/fts.rs` | Add `aliases`, `tags`, `modified_at` to `RawResult`; extend SELECT |
| `src/query/fallback.rs` | Extend SELECT and `query_map` closure to match `fts.rs` |
| `src/cli/query.rs` | Compute `now`; pass `aliases`, `tags`, `modified_at`, `now` to `score_components()` |
| `src/query/scorer.rs` | Update `score_components()` signature; implement all three bonuses; add six unit tests |

No new crates. No changes to CLI flags, JSON envelope shape, or any command
other than `query`.

## 10. Acceptance criteria

- `patina query "autonomy" --json --explain` returns `alias: 1.0` for a page
  whose `aliases` list contains `"controlled autonomy"`.
- `patina query "agents" --json --explain` returns `tag: 1.0` for a page whose
  `tags` list contains `"agents"`.
- `patina query "any term" --json --explain` returns `freshness: 1.0` for a
  page modified today and `freshness: 0.0` for a page modified more than 365
  days ago.
- All seven scoring unit tests pass (`cargo test -p patina scorer`).
- `cargo check` and `cargo test` pass with `SCHEMA_VERSION = "2"`.
- A database built with schema version 1 causes Patina to exit with the schema
  version error, not a silent wrong-score result.
