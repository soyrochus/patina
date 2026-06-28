use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Component, Path, PathBuf};

use anyhow::{Context, Result};
use pulldown_cmark::{Event, Parser, Tag};
use rusqlite::{Connection, OpenFlags};
use serde::Serialize;
use serde_yaml::Value;

use crate::config::PatinaConfig;
use crate::discovery::frontmatter::{ParsedMarkdown, parse_file};
use crate::discovery::scope;
use crate::discovery::walker;
use crate::output::{ErrorEntry, WarningEntry};

const VALID_STATUSES: &[&str] = &["draft", "active", "deprecated", "archived"];
const VALID_TYPES: &[&str] = &[
    "concept",
    "system",
    "project",
    "decision",
    "person",
    "process",
    "glossary",
    "source",
    "index",
    "open-question",
];

#[derive(Debug, Clone)]
pub struct LintReport {
    pub files_checked: usize,
    pub errors: Vec<ErrorEntry>,
    pub warnings: Vec<WarningEntry>,
}

impl LintReport {
    pub fn data(&self) -> LintData {
        LintData {
            files_checked: self.files_checked,
            error_count: self.errors.len(),
            warning_count: self.warnings.len(),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct LintData {
    pub files_checked: usize,
    pub error_count: usize,
    pub warning_count: usize,
}

#[derive(Debug)]
struct Page {
    path: PathBuf,
    parsed: ParsedMarkdown,
}

pub fn run_lint(config: &PatinaConfig) -> Result<LintReport> {
    let knowledge_root = config.knowledge_dir();
    let scope = scope::load(&knowledge_root)?;
    let scope_classification = scope.classification();
    let walk = walker::walk_markdown(&knowledge_root, config)?;
    let mut warnings = scope.warnings;
    warnings.extend(walk.warnings);
    let mut errors = Vec::new();
    let mut pages = Vec::new();

    for file in walk.files {
        if file.is_symlink {
            warnings.push(
                WarningEntry::new(
                    "symlink_encountered",
                    format!("symlink encountered during lint: {}", file.path.display()),
                )
                .with_path(&file.path),
            );
        }

        let contents = fs::read_to_string(&file.path)
            .with_context(|| format!("failed to read {}", file.path.display()))?;
        match parse_file(&file.path, &contents) {
            Ok(parsed) => {
                validate_front_matter(config, &file.path, &parsed.front_matter, &mut errors);
                validate_links(&knowledge_root, &file.path, &parsed.body, &mut errors);
                validate_source_refs(
                    &knowledge_root,
                    &file.path,
                    &parsed.front_matter,
                    &mut errors,
                    &mut warnings,
                );
                pages.push(Page {
                    path: file.path,
                    parsed,
                });
            }
            Err(error) => errors.push(error.to_error_entry()),
        }
    }

    validate_alias_uniqueness(&pages, &mut errors);
    validate_scope_classification_drift(scope_classification, &mut warnings)?;

    Ok(LintReport {
        files_checked: pages.len(),
        errors,
        warnings,
    })
}

fn validate_front_matter(
    config: &PatinaConfig,
    path: &Path,
    front_matter: &BTreeMap<String, Value>,
    errors: &mut Vec<ErrorEntry>,
) {
    for field in ["title", "type", "status"] {
        if !front_matter.contains_key(field) {
            errors.push(missing_required_field(path, field));
        }
    }

    if let Some(status) = string_field(front_matter, "status") {
        if !VALID_STATUSES.contains(&status) {
            errors.push(invalid_field_value(path, "status", status));
        }
    }

    if let Some(page_type) = string_field(front_matter, "type") {
        if !VALID_TYPES.contains(&page_type) {
            errors.push(invalid_field_value(path, "type", page_type));
        }

        if let Some(rule) = config.lint.page_types.get(page_type) {
            for field in &rule.required {
                if !front_matter.contains_key(field) {
                    errors.push(missing_required_field(path, field));
                }
            }
        }
    }
}

fn validate_scope_classification_drift(
    current_scope: Option<String>,
    warnings: &mut Vec<WarningEntry>,
) -> Result<()> {
    let db_path = PathBuf::from(".patina/index.sqlite");
    if !db_path.exists() {
        return Ok(());
    }

    let conn = Connection::open_with_flags(db_path, OpenFlags::SQLITE_OPEN_READ_ONLY)
        .context("failed to open Patina index for classification drift check")?;
    let mut statement = match conn.prepare("SELECT path, scope_classification FROM documents") {
        Ok(statement) => statement,
        Err(_) => return Ok(()),
    };

    let rows = statement
        .query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, Option<String>>(1)?))
        })
        .context("failed to read indexed scope classifications")?;

    for row in rows {
        let (path, indexed_scope) = row.context("failed to read indexed document scope")?;
        if indexed_scope != current_scope {
            warnings.push(
                WarningEntry::new(
                    "scope_classification_changed",
                    format!(
                        "scope classification for {path} changed from `{}` to `{}` since indexing",
                        indexed_scope.as_deref().unwrap_or("none"),
                        current_scope.as_deref().unwrap_or("none")
                    ),
                )
                .with_path(Path::new(&path)),
            );
        }
    }

    Ok(())
}

fn validate_links(knowledge_root: &Path, path: &Path, body: &str, errors: &mut Vec<ErrorEntry>) {
    for target in wikilink_targets(body) {
        if !resolve_wikilink(knowledge_root, &target).exists() {
            errors.push(
                ErrorEntry::new(
                    "broken_link",
                    format!("broken wikilink target `{target}` in {}", path.display()),
                )
                .with_path(path),
            );
        }
    }

    for event in Parser::new(body) {
        if let Event::Start(Tag::Link { dest_url, .. }) = event {
            let target = dest_url.as_ref();
            if should_validate_markdown_link(target) {
                let resolved = resolve_markdown_link(knowledge_root, path, target);
                if !resolved.exists() {
                    errors.push(
                        ErrorEntry::new(
                            "broken_link",
                            format!("broken Markdown link `{target}` in {}", path.display()),
                        )
                        .with_path(path),
                    );
                }
            }
        }
    }
}

fn validate_alias_uniqueness(pages: &[Page], errors: &mut Vec<ErrorEntry>) {
    let mut aliases: BTreeMap<String, PathBuf> = BTreeMap::new();
    let mut reported = BTreeSet::new();

    for page in pages {
        for alias in aliases_from(&page.parsed.front_matter) {
            if let Some(existing) = aliases.get(&alias) {
                if reported.insert(alias.clone()) {
                    errors.push(
                        ErrorEntry::new(
                            "duplicate_alias",
                            format!(
                                "duplicate alias `{alias}` declared in {} and {}",
                                existing.display(),
                                page.path.display()
                            ),
                        )
                        .with_path(&page.path),
                    );
                }
            } else {
                aliases.insert(alias, page.path.clone());
            }
        }
    }
}

fn validate_source_refs(
    knowledge_root: &Path,
    path: &Path,
    front_matter: &BTreeMap<String, Value>,
    errors: &mut Vec<ErrorEntry>,
    warnings: &mut Vec<WarningEntry>,
) {
    for source_ref in string_list_field(front_matter, "source_refs") {
        let resolved = resolve_source_ref(knowledge_root, &source_ref);
        let resolved_for_boundary = absolute_normalized(&resolved);
        let knowledge_root_for_boundary = absolute_normalized(knowledge_root);
        if !resolved.exists() {
            errors.push(
                ErrorEntry::new(
                    "missing_source_ref",
                    format!(
                        "missing source reference `{source_ref}` in {}",
                        path.display()
                    ),
                )
                .with_path(path),
            );
        }

        if !resolved_for_boundary.starts_with(&knowledge_root_for_boundary) {
            warnings.push(
                WarningEntry::new(
                    "source_ref_outside_knowledge_root",
                    format!(
                        "source reference `{source_ref}` in {} points outside the knowledge root",
                        path.display()
                    ),
                )
                .with_path(path),
            );
        }
    }
}

fn missing_required_field(path: &Path, field: &str) -> ErrorEntry {
    ErrorEntry::new(
        "missing_required_field",
        format!("missing required field `{field}` in {}", path.display()),
    )
    .with_path(path)
}

fn invalid_field_value(path: &Path, field: &str, value: &str) -> ErrorEntry {
    ErrorEntry::new(
        "invalid_field_value",
        format!(
            "invalid value `{value}` for field `{field}` in {}",
            path.display()
        ),
    )
    .with_path(path)
}

fn string_field<'a>(front_matter: &'a BTreeMap<String, Value>, field: &str) -> Option<&'a str> {
    front_matter.get(field).and_then(Value::as_str)
}

fn aliases_from(front_matter: &BTreeMap<String, Value>) -> Vec<String> {
    string_list_field(front_matter, "aliases")
}

fn string_list_field(front_matter: &BTreeMap<String, Value>, field: &str) -> Vec<String> {
    match front_matter.get(field) {
        Some(Value::String(value)) => vec![value.clone()],
        Some(Value::Sequence(values)) => values
            .iter()
            .filter_map(Value::as_str)
            .map(ToOwned::to_owned)
            .collect(),
        _ => Vec::new(),
    }
}

fn wikilink_targets(body: &str) -> Vec<String> {
    let mut targets = Vec::new();
    let mut rest = body;

    while let Some(start) = rest.find("[[") {
        rest = &rest[start + 2..];
        let Some(end) = rest.find("]]") else {
            break;
        };
        let raw = &rest[..end];
        let target = raw
            .split('|')
            .next()
            .unwrap_or(raw)
            .split('#')
            .next()
            .unwrap_or(raw)
            .trim();
        if !target.is_empty() {
            targets.push(target.to_string());
        }
        rest = &rest[end + 2..];
    }

    targets
}

fn resolve_wikilink(knowledge_root: &Path, target: &str) -> PathBuf {
    let path = Path::new(target);
    let candidate = if path.extension().is_some() {
        knowledge_root.join(path)
    } else {
        knowledge_root.join(format!("{target}.md"))
    };

    if candidate.exists() || target.contains('/') || target.contains('\\') {
        return candidate;
    }

    find_wikilink_by_file_name(knowledge_root, &format!("{target}.md")).unwrap_or(candidate)
}

fn find_wikilink_by_file_name(knowledge_root: &Path, file_name: &str) -> Option<PathBuf> {
    walkdir::WalkDir::new(knowledge_root)
        .into_iter()
        .filter_map(|entry| entry.ok())
        .find_map(|entry| {
            let path = entry.path();
            path.is_file()
                .then_some(path)
                .filter(|path| path.file_name().and_then(|name| name.to_str()) == Some(file_name))
                .map(Path::to_path_buf)
        })
}

fn should_validate_markdown_link(target: &str) -> bool {
    if target.starts_with("http://")
        || target.starts_with("https://")
        || target.starts_with('#')
        || target.starts_with("mailto:")
    {
        return false;
    }

    target
        .split('#')
        .next()
        .and_then(|path| Path::new(path).extension())
        .and_then(|extension| extension.to_str())
        .map(|extension| extension.eq_ignore_ascii_case("md"))
        .unwrap_or(false)
}

fn resolve_markdown_link(knowledge_root: &Path, source: &Path, target: &str) -> PathBuf {
    let target = target.split('#').next().unwrap_or(target);
    let path = Path::new(target);
    if path.is_absolute() {
        path.to_path_buf()
    } else if path.starts_with(knowledge_root) {
        path.to_path_buf()
    } else {
        source.parent().unwrap_or(knowledge_root).join(path)
    }
}

fn resolve_source_ref(knowledge_root: &Path, source_ref: &str) -> PathBuf {
    let path = Path::new(source_ref);
    if path.is_absolute() {
        return path.to_path_buf();
    }

    if let Some(root_name) = knowledge_root.file_name() {
        if path.starts_with(root_name) {
            if let Some(parent) = knowledge_root.parent() {
                return parent.join(path);
            }
        }
    }

    knowledge_root.join(path)
}

fn absolute_normalized(path: &Path) -> PathBuf {
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join(path)
    };
    normalize_components(&absolute)
}

fn normalize_components(path: &Path) -> PathBuf {
    let mut normalized = PathBuf::new();

    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                normalized.pop();
            }
            Component::Normal(part) => normalized.push(part),
            Component::RootDir | Component::Prefix(_) => normalized.push(component.as_os_str()),
        }
    }

    normalized
}
