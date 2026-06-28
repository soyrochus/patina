use std::collections::HashSet;

const STOP_WORDS: &[&str] = &[
    "a", "an", "and", "are", "as", "at", "be", "but", "by", "can", "does", "for", "from", "how",
    "if", "in", "is", "it", "of", "on", "or", "should", "that", "the", "this", "to", "use", "what",
    "when", "where", "who", "why", "with",
];

const SHORT_TERM_ALLOW_LIST: &[&str] = &["ai", "cli", "db", "fts", "ui", "v0", "v1", "v2", "v3"];

#[derive(Debug, Clone)]
pub struct QueryPlan {
    raw: String,
    trimmed: String,
    terms: Vec<String>,
    empty: bool,
}

impl QueryPlan {
    pub fn new(raw: &str) -> Self {
        let trimmed = raw.trim().to_string();
        if trimmed.is_empty() {
            return Self {
                raw: raw.to_string(),
                trimmed,
                terms: Vec::new(),
                empty: true,
            };
        }

        let mut terms = normalize_terms(&trimmed);
        if terms.is_empty() {
            terms.push(trimmed.to_lowercase());
        }

        Self {
            raw: raw.to_string(),
            trimmed,
            terms,
            empty: false,
        }
    }

    pub fn raw(&self) -> &str {
        &self.raw
    }

    pub fn trimmed(&self) -> &str {
        &self.trimmed
    }

    pub fn terms(&self) -> &[String] {
        &self.terms
    }

    pub fn is_empty(&self) -> bool {
        self.empty
    }

    pub fn fts_all_expression(&self) -> String {
        self.fts_expression(" AND ")
    }

    pub fn fts_any_expression(&self) -> String {
        self.fts_expression(" OR ")
    }

    pub fn like_patterns(&self) -> Vec<String> {
        self.terms.iter().map(|term| format!("%{term}%")).collect()
    }

    pub fn matched_term_count(&self, text: &str) -> usize {
        let lower = text.to_lowercase();
        self.terms
            .iter()
            .filter(|term| lower.contains(term.as_str()))
            .count()
    }

    pub fn excerpt(&self, text: &str) -> String {
        excerpt_for_terms(text, &self.terms)
    }

    fn fts_expression(&self, separator: &str) -> String {
        self.terms
            .iter()
            .map(|term| quote_fts_term(term))
            .collect::<Vec<_>>()
            .join(separator)
    }
}

pub fn excerpt_for_terms(text: &str, terms: &[String]) -> String {
    let lower = text.to_lowercase();
    let start = terms
        .iter()
        .filter_map(|term| lower.find(term).map(|index| index.saturating_sub(80)))
        .min()
        .unwrap_or(0);
    text.chars().skip(start).take(240).collect()
}

fn normalize_terms(input: &str) -> Vec<String> {
    let separated = input
        .to_lowercase()
        .chars()
        .map(|character| {
            if character.is_alphanumeric() {
                character
            } else {
                ' '
            }
        })
        .collect::<String>();

    let stop_words = STOP_WORDS.iter().copied().collect::<HashSet<_>>();
    let short_allow_list = SHORT_TERM_ALLOW_LIST
        .iter()
        .copied()
        .collect::<HashSet<_>>();
    let mut seen = HashSet::new();
    let mut terms = Vec::new();

    for term in separated.split_ascii_whitespace() {
        if stop_words.contains(term) {
            continue;
        }
        if term.chars().count() < 3
            && !term.chars().any(|character| character.is_ascii_digit())
            && !short_allow_list.contains(term)
        {
            continue;
        }
        if seen.insert(term.to_string()) {
            terms.push(term.to_string());
        }
    }

    terms
}

fn quote_fts_term(term: &str) -> String {
    format!("\"{}\"", term.replace('"', "\"\""))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_natural_language_query() {
        let plan = QueryPlan::new("why should agents use Patina as durable context");

        assert_eq!(plan.terms(), &["agents", "patina", "durable", "context"]);
    }

    #[test]
    fn treats_punctuation_and_fts_syntax_as_separators() {
        let plan = QueryPlan::new("agents: durable OR context?");

        assert_eq!(plan.terms(), &["agents", "durable", "context"]);
    }

    #[test]
    fn removes_duplicates_and_filters_short_terms() {
        let plan = QueryPlan::new("AI ai x yz db v4 UX agents agents");

        assert_eq!(plan.terms(), &["ai", "db", "v4", "agents"]);
    }

    #[test]
    fn falls_back_to_trimmed_query_when_all_terms_are_removed() {
        let plan = QueryPlan::new("why and how");

        assert_eq!(plan.terms(), &["why and how"]);
        assert!(!plan.is_empty());
    }

    #[test]
    fn marks_blank_query_as_empty() {
        let plan = QueryPlan::new("   ");

        assert!(plan.is_empty());
        assert!(plan.terms().is_empty());
    }

    #[test]
    fn builds_quoted_fts_expressions() {
        let plan = QueryPlan::new("agents durable context");

        assert_eq!(
            plan.fts_all_expression(),
            "\"agents\" AND \"durable\" AND \"context\""
        );
        assert_eq!(
            plan.fts_any_expression(),
            "\"agents\" OR \"durable\" OR \"context\""
        );
    }

    #[test]
    fn builds_like_patterns_and_counts_matches() {
        let plan = QueryPlan::new("agents durable context");

        assert_eq!(
            plan.like_patterns(),
            vec!["%agents%", "%durable%", "%context%"]
        );
        assert_eq!(
            plan.matched_term_count("Agents cite durable project knowledge."),
            2
        );
    }
}
