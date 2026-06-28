use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result};
use clap::Args;
use rusqlite::{Connection, OpenFlags, OptionalExtension};
use serde::Serialize;
use serde_yaml::Value;

use crate::config::PatinaConfig;
use crate::discovery::{git, gitignore, scope};
use crate::output::{JsonEnvelope, WarningEntry, print_json};

#[derive(Debug, Args)]
pub struct StatusArgs {
    #[arg(long)]
    pub json: bool,
}

#[derive(Debug, Serialize)]
struct StatusData {
    git_worktree_detected: bool,
    uncommitted_knowledge_changes: bool,
    patina_ignored_by_git: bool,
    index_last_built: Option<String>,
    scope: Option<Value>,
}

pub fn run(args: StatusArgs, config: &PatinaConfig) -> Result<()> {
    let data = collect_status(config)?;
    let mut warnings = Vec::new();
    if !data.patina_ignored_by_git {
        warnings.push(WarningEntry::new(
            "patina_not_gitignored",
            ".patina/ should be listed in .gitignore",
        ));
    }
    if staged_patina_files()? {
        warnings.push(WarningEntry::new(
            "patina_staged",
            "files under .patina/ are staged for commit",
        ));
    }

    if args.json {
        print_json(&JsonEnvelope::success("status", data, warnings))?;
    } else {
        for warning in &warnings {
            println!("warning: {}", warning.message);
        }
        println!(
            "Git worktree detected: {}",
            yes_no(data.git_worktree_detected)
        );
        println!(
            "Uncommitted knowledge changes: {}",
            yes_no(data.uncommitted_knowledge_changes)
        );
        println!(
            ".patina/ ignored by Git: {}",
            yes_no(data.patina_ignored_by_git)
        );
        println!(
            "Index last built: {}",
            data.index_last_built.as_deref().unwrap_or("unknown")
        );
        if let Some(scope) = data.scope {
            println!("Scope metadata: {}", serde_json::to_string(&scope)?);
        }
    }
    Ok(())
}

fn collect_status(config: &PatinaConfig) -> Result<StatusData> {
    let worktree = git::detect()?;
    let root = worktree.root.clone().unwrap_or(std::env::current_dir()?);
    let knowledge_dir = config.knowledge_dir();
    let scope = scope::load(&knowledge_dir)?;

    Ok(StatusData {
        git_worktree_detected: worktree.inside,
        uncommitted_knowledge_changes: uncommitted_knowledge_changes(&knowledge_dir)?,
        patina_ignored_by_git: gitignore::has_entry(&root.join(".gitignore"), ".patina/")?,
        index_last_built: index_last_built(&root.join(".patina/index.sqlite"))?,
        scope: scope.metadata,
    })
}

fn uncommitted_knowledge_changes(knowledge_dir: &Path) -> Result<bool> {
    let output = Command::new("git")
        .args(["status", "--porcelain", "--"])
        .arg(knowledge_dir)
        .output();
    match output {
        Ok(output) if output.status.success() => Ok(!output.stdout.is_empty()),
        Ok(_) => Ok(false),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(error).context("failed to run git status"),
    }
}

fn staged_patina_files() -> Result<bool> {
    let output = Command::new("git")
        .args(["diff", "--cached", "--name-only", "--", ".patina"])
        .output();
    match output {
        Ok(output) if output.status.success() => Ok(!output.stdout.is_empty()),
        Ok(_) => Ok(false),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(error).context("failed to inspect staged .patina files"),
    }
}

fn index_last_built(path: &PathBuf) -> Result<Option<String>> {
    if !path.exists() {
        return Ok(None);
    }
    let conn = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY)
        .with_context(|| format!("failed to open {}", path.display()))?;
    conn.query_row(
        "SELECT value FROM meta WHERE key = 'updated_at'",
        [],
        |row| row.get(0),
    )
    .optional()
    .context("failed to read index updated_at")
}

fn yes_no(value: bool) -> &'static str {
    if value { "yes" } else { "no" }
}
