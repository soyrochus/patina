use chrono::{DateTime, Utc};
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct ScoreComponents {
    pub fts: f64,
    pub title: f64,
    pub alias: f64,
    pub tag: f64,
    pub page_type: f64,
    pub freshness: f64,
    pub provenance: f64,
}

impl ScoreComponents {
    pub fn combined(&self) -> f64 {
        self.fts * 0.70
            + self.title * 0.10
            + self.alias * 0.07
            + self.tag * 0.05
            + self.page_type * 0.03
            + self.freshness * 0.03
            + self.provenance * 0.02
    }
}

pub fn score_components(
    terms: &[String],
    normalized_fts: f64,
    title: Option<&str>,
    page_type: Option<&str>,
    scope_classification: Option<&str>,
    aliases: Option<&str>,
    tags: Option<&str>,
    modified_at: Option<&str>,
    now: DateTime<Utc>,
) -> ScoreComponents {
    let title_bonus = field_contains_any_term(title, terms);
    let alias_bonus = json_list_contains_any_term(aliases, terms);
    let tag_bonus = json_list_contains_any_term(tags, terms);
    let page_type_bonus = page_type
        .map(|page_type| (page_type == "concept" || page_type == "decision") as u8 as f64)
        .unwrap_or(0.0);
    let freshness_bonus = freshness_bonus(modified_at, now);
    let provenance_bonus = scope_classification.is_some() as u8 as f64;

    ScoreComponents {
        fts: normalized_fts.clamp(0.0, 1.0),
        title: title_bonus,
        alias: alias_bonus,
        tag: tag_bonus,
        page_type: page_type_bonus,
        freshness: freshness_bonus,
        provenance: provenance_bonus,
    }
}

fn field_contains_any_term(value: Option<&str>, terms: &[String]) -> f64 {
    value
        .map(|value| {
            let lower = value.to_lowercase();
            terms.iter().any(|term| lower.contains(term.as_str())) as u8 as f64
        })
        .unwrap_or(0.0)
}

fn json_list_contains_any_term(json: Option<&str>, terms: &[String]) -> f64 {
    json.and_then(|value| serde_json::from_str::<Vec<String>>(value).ok())
        .map(|values| {
            values.iter().any(|value| {
                let lower = value.to_lowercase();
                terms.iter().any(|term| lower.contains(term.as_str()))
            }) as u8 as f64
        })
        .unwrap_or(0.0)
}

fn freshness_bonus(modified_at: Option<&str>, now: DateTime<Utc>) -> f64 {
    const DECAY_WINDOW_DAYS: i64 = 365;

    modified_at
        .and_then(|value| DateTime::parse_from_rfc3339(value).ok())
        .map(|modified| {
            let age_days = (now - modified.with_timezone(&Utc)).num_days().max(0);
            if age_days >= DECAY_WINDOW_DAYS {
                0.0
            } else {
                (DECAY_WINDOW_DAYS - age_days) as f64 / DECAY_WINDOW_DAYS as f64
            }
        })
        .unwrap_or(0.0)
}

pub fn normalize_bm25(raw_scores: &[f64]) -> Vec<f64> {
    if raw_scores.is_empty() {
        return Vec::new();
    }

    if raw_scores.len() == 1 {
        return vec![1.0];
    }

    let min = raw_scores.iter().copied().fold(f64::INFINITY, f64::min);
    let max = raw_scores.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    if (max - min).abs() < f64::EPSILON {
        return vec![1.0; raw_scores.len()];
    }

    raw_scores
        .iter()
        .map(|score| (max - score) / (max - min))
        .collect()
}

pub fn normalize_higher_is_better(raw_scores: &[f64]) -> Vec<f64> {
    if raw_scores.is_empty() {
        return Vec::new();
    }

    let max = raw_scores.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    if max <= 0.0 {
        return vec![0.0; raw_scores.len()];
    }

    raw_scores
        .iter()
        .map(|score| (score / max).clamp(0.0, 1.0))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    fn fixed_now() -> DateTime<Utc> {
        DateTime::parse_from_rfc3339("2026-01-01T00:00:00Z")
            .expect("fixed timestamp should parse")
            .with_timezone(&Utc)
    }

    fn terms(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| value.to_string()).collect()
    }

    #[test]
    fn weights_sum_to_one() {
        let components = ScoreComponents {
            fts: 1.0,
            title: 1.0,
            alias: 1.0,
            tag: 1.0,
            page_type: 1.0,
            freshness: 1.0,
            provenance: 1.0,
        };

        assert!((components.combined() - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn normalizes_single_result_to_one() {
        assert_eq!(normalize_bm25(&[-3.0]), vec![1.0]);
    }

    #[test]
    fn normalizes_higher_scores_as_better() {
        assert_eq!(normalize_higher_is_better(&[1.0, 2.0]), vec![0.5, 1.0]);
    }

    #[test]
    fn alias_bonus_matches_substring() {
        let components = score_components(
            &terms(&["autonomy"]),
            1.0,
            None,
            None,
            None,
            Some(r#"["controlled autonomy","agent loop"]"#),
            None,
            None,
            fixed_now(),
        );

        assert_eq!(components.alias, 1.0);
    }

    #[test]
    fn alias_bonus_no_match() {
        let components = score_components(
            &terms(&["routing"]),
            1.0,
            None,
            None,
            None,
            Some(r#"["controlled autonomy"]"#),
            None,
            None,
            fixed_now(),
        );

        assert_eq!(components.alias, 0.0);
    }

    #[test]
    fn tag_bonus_matches() {
        let components = score_components(
            &terms(&["agents"]),
            1.0,
            None,
            None,
            None,
            None,
            Some(r#"["agents","architecture"]"#),
            None,
            fixed_now(),
        );

        assert_eq!(components.tag, 1.0);
    }

    #[test]
    fn freshness_full_for_today() {
        let now = fixed_now();
        let modified = now.to_rfc3339();
        let components = score_components(
            &terms(&["x"]),
            1.0,
            None,
            None,
            None,
            None,
            None,
            Some(&modified),
            now,
        );

        assert_eq!(components.freshness, 1.0);
    }

    #[test]
    fn freshness_zero_for_old_document() {
        let now = fixed_now();
        let modified = (now - Duration::days(400)).to_rfc3339();
        let components = score_components(
            &terms(&["x"]),
            1.0,
            None,
            None,
            None,
            None,
            None,
            Some(&modified),
            now,
        );

        assert_eq!(components.freshness, 0.0);
    }

    #[test]
    fn freshness_zero_when_absent() {
        let components = score_components(
            &terms(&["x"]),
            1.0,
            None,
            None,
            None,
            None,
            None,
            None,
            fixed_now(),
        );

        assert_eq!(components.freshness, 0.0);
    }

    #[test]
    fn title_bonus_matches_any_term() {
        let components = score_components(
            &terms(&["controlled", "autonomy", "important"]),
            1.0,
            Some("Controlled Autonomy"),
            None,
            None,
            None,
            None,
            None,
            fixed_now(),
        );

        assert_eq!(components.title, 1.0);
    }

    #[test]
    fn alias_bonus_matches_any_normalized_term() {
        let components = score_components(
            &terms(&["controlled", "autonomy", "important"]),
            1.0,
            None,
            None,
            None,
            Some(r#"["controlled autonomy"]"#),
            None,
            None,
            fixed_now(),
        );

        assert_eq!(components.alias, 1.0);
    }
}
