use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde_yaml::Value;

use crate::output::ErrorEntry;

#[derive(Debug, Clone)]
pub struct ParsedMarkdown {
    pub front_matter: BTreeMap<String, Value>,
    pub body: String,
}

#[derive(Debug, Clone)]
pub struct FrontMatterError {
    pub path: PathBuf,
    pub message: String,
}

impl FrontMatterError {
    pub fn to_error_entry(&self) -> ErrorEntry {
        ErrorEntry::new("invalid_front_matter", &self.message).with_path(&self.path)
    }
}

pub fn parse_file(path: &Path, contents: &str) -> Result<ParsedMarkdown, FrontMatterError> {
    let Some(after_opening_fence) = opening_fence_len(contents) else {
        return Ok(ParsedMarkdown {
            front_matter: BTreeMap::new(),
            body: contents.to_string(),
        });
    };

    let Some((yaml, body)) = split_front_matter(contents, after_opening_fence) else {
        return Err(FrontMatterError {
            path: path.to_path_buf(),
            message: format!(
                "invalid YAML front matter in {}: missing closing --- fence",
                path.display()
            ),
        });
    };

    let front_matter = parse_yaml_mapping(path, yaml)?;
    Ok(ParsedMarkdown {
        front_matter,
        body: body.to_string(),
    })
}

fn opening_fence_len(contents: &str) -> Option<usize> {
    if contents == "---" {
        return Some(3);
    }
    if contents.starts_with("---\n") {
        return Some(4);
    }
    if contents.starts_with("---\r\n") {
        return Some(5);
    }
    None
}

fn split_front_matter(contents: &str, start: usize) -> Option<(&str, &str)> {
    let mut offset = start;
    for line in contents[start..].split_inclusive('\n') {
        let trimmed = line.trim_end_matches(['\r', '\n']);
        if trimmed == "---" {
            let yaml = &contents[start..offset];
            let body = &contents[offset + line.len()..];
            return Some((yaml, body));
        }
        offset += line.len();
    }

    let tail = &contents[start..];
    if tail.trim_end_matches('\r') == "---" {
        return Some((&contents[start..start], ""));
    }

    None
}

fn parse_yaml_mapping(
    path: &Path,
    yaml: &str,
) -> Result<BTreeMap<String, Value>, FrontMatterError> {
    if yaml.trim().is_empty() {
        return Ok(BTreeMap::new());
    }

    let value = serde_yaml::from_str::<Value>(yaml).map_err(|error| FrontMatterError {
        path: path.to_path_buf(),
        message: format!("invalid YAML front matter in {}: {error}", path.display()),
    })?;

    let Value::Mapping(mapping) = value else {
        return Err(FrontMatterError {
            path: path.to_path_buf(),
            message: format!(
                "invalid YAML front matter in {}: expected a mapping",
                path.display()
            ),
        });
    };

    let mut result = BTreeMap::new();
    for (key, value) in mapping {
        let Some(key) = key.as_str() else {
            return Err(FrontMatterError {
                path: path.to_path_buf(),
                message: format!(
                    "invalid YAML front matter in {}: keys must be strings",
                    path.display()
                ),
            });
        };
        result.insert(key.to_string(), value);
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_valid_front_matter() {
        let parsed = parse_file(
            Path::new("page.md"),
            "---\ntitle: Foo\ntype: concept\nstatus: active\n---\n# Body\n",
        )
        .expect("front matter should parse");

        assert_eq!(parsed.front_matter["title"].as_str(), Some("Foo"));
        assert_eq!(parsed.body, "# Body\n");
    }

    #[test]
    fn no_front_matter_returns_empty_map() {
        let parsed = parse_file(Path::new("page.md"), "# Body\n").expect("page should parse");

        assert!(parsed.front_matter.is_empty());
        assert_eq!(parsed.body, "# Body\n");
    }

    #[test]
    fn invalid_yaml_reports_path() {
        let error = parse_file(Path::new("page.md"), "---\ntitle: [\n---\n")
            .expect_err("invalid YAML should fail");

        assert!(error.message.contains("page.md"));
    }
}
