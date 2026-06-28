use std::path::{Path, PathBuf};

use anyhow::Result;
use ignore::WalkBuilder;

use crate::config::PatinaConfig;
use crate::output::WarningEntry;

#[derive(Debug, Clone)]
pub struct MarkdownFile {
    pub path: PathBuf,
    pub size_bytes: u64,
    pub is_symlink: bool,
}

#[derive(Debug, Clone)]
pub struct WalkResult {
    pub files: Vec<MarkdownFile>,
    pub warnings: Vec<WarningEntry>,
}

pub fn walk_markdown(knowledge_root: &Path, config: &PatinaConfig) -> Result<WalkResult> {
    let mut files = Vec::new();
    let mut warnings = Vec::new();
    let max_file_size = config.limits.max_markdown_file_mb * 1024 * 1024;

    for entry in WalkBuilder::new(knowledge_root)
        .git_ignore(true)
        .git_global(true)
        .ignore(true)
        .hidden(false)
        .build()
    {
        let entry = match entry {
            Ok(entry) => entry,
            Err(error) => {
                warnings.push(WarningEntry::new(
                    "walk_error",
                    format!("failed to walk knowledge directory: {error}"),
                ));
                continue;
            }
        };

        let file_type = entry.file_type();
        if !file_type
            .map(|file_type| file_type.is_file() || file_type.is_symlink())
            .unwrap_or(false)
        {
            continue;
        }

        let path = entry.path();
        if !is_markdown(path) {
            continue;
        }

        let metadata = match path.symlink_metadata() {
            Ok(metadata) => metadata,
            Err(error) => {
                warnings.push(
                    WarningEntry::new(
                        "metadata_unavailable",
                        format!("failed to read metadata for {}: {error}", path.display()),
                    )
                    .with_path(path),
                );
                continue;
            }
        };

        let size_bytes = metadata.len();
        if size_bytes > max_file_size {
            warnings.push(
                WarningEntry::new(
                    "markdown_file_too_large",
                    format!(
                        "skipping {} because it is {} bytes, above the configured {} MB limit",
                        path.display(),
                        size_bytes,
                        config.limits.max_markdown_file_mb
                    ),
                )
                .with_path(path),
            );
            continue;
        }

        files.push(MarkdownFile {
            path: path.to_path_buf(),
            size_bytes,
            is_symlink: metadata.file_type().is_symlink(),
        });
    }

    if files.len() as u64 > config.limits.max_total_markdown_files {
        warnings.push(WarningEntry::new(
            "markdown_file_count_exceeded",
            format!(
                "knowledge directory contains {} Markdown files, above the configured {} file warning threshold",
                files.len(),
                config.limits.max_total_markdown_files
            ),
        ));
    }

    Ok(WalkResult { files, warnings })
}

fn is_markdown(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .map(|extension| extension.eq_ignore_ascii_case("md"))
        .unwrap_or(false)
}
