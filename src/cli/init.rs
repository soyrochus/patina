use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::Path;

use anyhow::{Context, Result, bail};
use clap::Args;
use serde::Serialize;

use crate::config::PatinaConfig;
use crate::discovery::git;
use crate::discovery::gitignore;
use crate::output::{JsonEnvelope, WarningEntry, print_json};

#[derive(Debug, Args)]
pub struct InitArgs {
    #[arg(long)]
    pub json: bool,
    #[arg(long)]
    pub no_git: bool,
}

#[derive(Debug, Serialize)]
struct InitData {
    files_written: Vec<String>,
    files_skipped: Vec<String>,
}

pub fn run(args: InitArgs, config: &PatinaConfig) -> Result<()> {
    let worktree = git::detect()?;
    if !worktree.inside && !args.no_git {
        if !args.json {
            eprintln!("warning: no Git repository detected");
        }
        bail!("no Git repository detected; rerun with --no-git to initialize anyway");
    }

    let base = worktree
        .root
        .clone()
        .unwrap_or(std::env::current_dir().context("failed to read current directory")?);
    let knowledge_dir = base.join(config.knowledge_dir());

    let mut files_written = Vec::new();
    let mut files_skipped = Vec::new();
    let mut warnings = Vec::new();

    create_dir(&knowledge_dir)?;
    create_dir(&knowledge_dir.join("wiki"))?;
    create_dir(&knowledge_dir.join("sources"))?;
    create_dir(&knowledge_dir.join("schemas"))?;

    write_stub(
        &knowledge_dir.join("README.md"),
        "# Knowledge\n\nThis directory contains Patina-managed Markdown knowledge.\n",
        &mut files_written,
        &mut files_skipped,
    )?;
    write_stub(
        &knowledge_dir.join("AGENTS.md"),
        "# Agent Instructions\n\nUse `patina query`, `patina read`, `patina lint`, and `patina stale` to work with this knowledge base.\n",
        &mut files_written,
        &mut files_skipped,
    )?;

    if !worktree.inside {
        warnings.push(WarningEntry::new(
            "git_worktree_missing",
            "no Git repository detected; initialized because --no-git was supplied",
        ));
    }

    ensure_gitignore_entry(&base, &mut files_written, &mut files_skipped)?;

    let data = InitData {
        files_written,
        files_skipped,
    };

    if args.json {
        print_json(&JsonEnvelope::success("init", data, warnings))?;
    } else {
        for warning in warnings {
            println!("warning: {}", warning.message);
        }
        println!(
            "initialized Patina knowledge directory at {}",
            knowledge_dir.display()
        );
    }

    Ok(())
}

fn create_dir(path: &Path) -> Result<()> {
    fs::create_dir_all(path)
        .with_context(|| format!("failed to create directory {}", path.display()))
}

fn write_stub(
    path: &Path,
    contents: &str,
    files_written: &mut Vec<String>,
    files_skipped: &mut Vec<String>,
) -> Result<()> {
    if path.exists() {
        files_skipped.push(path.display().to_string());
        return Ok(());
    }

    fs::write(path, contents).with_context(|| format!("failed to write {}", path.display()))?;
    files_written.push(path.display().to_string());
    Ok(())
}

fn ensure_gitignore_entry(
    base: &Path,
    files_written: &mut Vec<String>,
    files_skipped: &mut Vec<String>,
) -> Result<()> {
    let gitignore = base.join(".gitignore");
    if gitignore::has_entry(&gitignore, ".patina/")? {
        files_skipped.push(gitignore.display().to_string());
        return Ok(());
    }

    append_gitignore_entry(&gitignore, ".patina/")?;
    files_written.push(gitignore.display().to_string());
    Ok(())
}

fn append_gitignore_entry(path: &Path, entry: &str) -> Result<()> {
    let needs_leading_newline = path
        .metadata()
        .map(|metadata| metadata.len() > 0)
        .unwrap_or(false);
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .with_context(|| format!("failed to open {}", path.display()))?;
    if needs_leading_newline {
        writeln!(file)?;
    }
    writeln!(file, "{entry}")?;
    Ok(())
}
