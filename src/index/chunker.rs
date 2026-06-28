use serde::Serialize;

use crate::config::IndexConfig;
use crate::index::sha256_hex;

#[derive(Debug, Clone, Serialize)]
pub struct Chunk {
    pub ordinal: usize,
    pub heading_path: Vec<String>,
    pub text: String,
    pub token_estimate: usize,
    pub sha256: String,
}

struct Section {
    heading_path: Vec<String>,
    text: String,
}

pub fn chunk_markdown(markdown: &str, config: &IndexConfig) -> Vec<Chunk> {
    let sections = heading_sections(markdown);
    let mut chunks = Vec::new();

    for section in sections {
        for text in split_oversized_section(&section.text, config.chunk_size, config.chunk_overlap)
        {
            let token_estimate = estimate_tokens(&text);
            chunks.push(Chunk {
                ordinal: chunks.len(),
                heading_path: section.heading_path.clone(),
                sha256: sha256_hex(text.as_bytes()),
                text,
                token_estimate,
            });
        }
    }

    if chunks.is_empty() && !markdown.trim().is_empty() {
        let text = markdown.to_string();
        chunks.push(Chunk {
            ordinal: 0,
            heading_path: Vec::new(),
            token_estimate: estimate_tokens(&text),
            sha256: sha256_hex(text.as_bytes()),
            text,
        });
    }

    chunks
}

pub fn estimate_tokens(text: &str) -> usize {
    text.chars().count().div_ceil(4)
}

fn heading_sections(markdown: &str) -> Vec<Section> {
    let mut sections = Vec::new();
    let mut heading_stack: Vec<String> = Vec::new();
    let mut current_heading = Vec::new();
    let mut current_text = String::new();

    for line in markdown.lines() {
        if let Some((level, title)) = parse_atx_heading(line) {
            if !current_text.trim().is_empty() {
                sections.push(Section {
                    heading_path: current_heading.clone(),
                    text: current_text.trim().to_string(),
                });
                current_text.clear();
            }

            let level_index = level.saturating_sub(1);
            heading_stack.truncate(level_index);
            heading_stack.push(title);
            current_heading = heading_stack.clone();
        }

        current_text.push_str(line);
        current_text.push('\n');
    }

    if !current_text.trim().is_empty() {
        sections.push(Section {
            heading_path: current_heading,
            text: current_text.trim().to_string(),
        });
    }

    sections
}

fn parse_atx_heading(line: &str) -> Option<(usize, String)> {
    let trimmed = line.trim_start();
    let hashes = trimmed.chars().take_while(|ch| *ch == '#').count();
    if hashes == 0 || hashes > 6 {
        return None;
    }

    let remainder = trimmed.get(hashes..)?;
    if !remainder.starts_with(char::is_whitespace) {
        return None;
    }

    let title = remainder.trim().trim_end_matches('#').trim();
    if title.is_empty() {
        return None;
    }

    Some((hashes, title.to_string()))
}

fn split_oversized_section(text: &str, chunk_size: usize, chunk_overlap: usize) -> Vec<String> {
    if estimate_tokens(text) <= chunk_size {
        return vec![text.to_string()];
    }

    let max_chars = chunk_size.saturating_mul(4).max(1);
    let overlap_chars = chunk_overlap
        .saturating_mul(4)
        .min(max_chars.saturating_sub(1));
    let paragraphs: Vec<&str> = text.split("\n\n").collect();
    let mut chunks = Vec::new();
    let mut current = String::new();

    for paragraph in paragraphs {
        let candidate_len = current.len() + paragraph.len() + 2;
        if !current.is_empty() && candidate_len > max_chars {
            chunks.push(current.trim().to_string());
            let overlap = tail_chars(&current, overlap_chars);
            current = overlap;
            if !current.is_empty() {
                current.push_str("\n\n");
            }
        }

        current.push_str(paragraph);
        current.push_str("\n\n");
    }

    if !current.trim().is_empty() {
        chunks.push(current.trim().to_string());
    }

    chunks
}

fn tail_chars(text: &str, count: usize) -> String {
    let chars: Vec<char> = text.chars().collect();
    let start = chars.len().saturating_sub(count);
    chars[start..].iter().collect::<String>().trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config(chunk_size: usize, chunk_overlap: usize) -> IndexConfig {
        IndexConfig {
            chunk_size,
            chunk_overlap,
            chunk_strategy: "heading-aware".to_string(),
        }
    }

    #[test]
    fn chunks_single_heading() {
        let chunks = chunk_markdown("# A\n\nBody", &config(1200, 150));

        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].heading_path, vec!["A"]);
    }

    #[test]
    fn chunks_multiple_headings() {
        let chunks = chunk_markdown(
            "# H1\n\nIntro\n\n## H2a\n\nA\n\n## H2b\n\nB",
            &config(1200, 150),
        );

        assert_eq!(chunks.len(), 3);
        assert_eq!(chunks[0].heading_path, vec!["H1"]);
        assert_eq!(chunks[1].heading_path, vec!["H1", "H2a"]);
        assert_eq!(chunks[2].heading_path, vec!["H1", "H2b"]);
    }

    #[test]
    fn splits_oversized_sections_with_overlap() {
        let markdown = format!("# A\n\n{}\n\n{}", "a".repeat(80), "b".repeat(80));
        let chunks = chunk_markdown(&markdown, &config(10, 2));

        assert!(chunks.len() > 1);
        assert!(chunks.iter().all(|chunk| chunk.heading_path == vec!["A"]));
    }

    #[test]
    fn estimates_tokens_by_ceiling_char_count_over_four() {
        assert_eq!(estimate_tokens(&"x".repeat(400)), 100);
        assert_eq!(estimate_tokens("x"), 1);
    }
}
