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
    query: &str,
    normalized_fts: f64,
    title: Option<&str>,
    page_type: Option<&str>,
    scope_classification: Option<&str>,
) -> ScoreComponents {
    let query_lower = query.to_lowercase();
    let title_bonus = title
        .map(|title| title.to_lowercase().contains(&query_lower) as u8 as f64)
        .unwrap_or(0.0);
    let page_type_bonus = page_type
        .map(|page_type| (page_type == "concept" || page_type == "decision") as u8 as f64)
        .unwrap_or(0.0);
    let provenance_bonus = scope_classification.is_some() as u8 as f64;

    ScoreComponents {
        fts: normalized_fts.clamp(0.0, 1.0),
        title: title_bonus,
        alias: 0.0,
        tag: 0.0,
        page_type: page_type_bonus,
        freshness: 0.5,
        provenance: provenance_bonus,
    }
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

#[cfg(test)]
mod tests {
    use super::*;

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
}
