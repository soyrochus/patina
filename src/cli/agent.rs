use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use clap::Args;
use serde::Serialize;

use crate::config::PatinaConfig;
use crate::output::{JsonEnvelope, print_json};

#[derive(Debug, Args)]
pub struct InstallAgentArgs {
    #[arg(long)]
    pub json: bool,
    #[arg(long)]
    pub force: bool,
    #[arg(long)]
    pub agent: Option<String>,
}

#[derive(Debug, Serialize)]
struct InstallAgentData {
    files_written: Vec<String>,
}

pub fn run(args: InstallAgentArgs, config: &PatinaConfig) -> Result<()> {
    let mut files_written = Vec::new();
    write_generic_agents(config, args.force, &mut files_written)?;

    if let Some(agent) = &args.agent {
        match agent.as_str() {
            "claude-code" => write_claude_code(args.force, &mut files_written)?,
            _ => bail!("unrecognised agent `{agent}`; supported agent types: claude-code"),
        }
    }

    if args.json {
        print_json(&JsonEnvelope::success(
            "install-agent",
            InstallAgentData {
                files_written: files_written.clone(),
            },
            Vec::new(),
        ))?;
    } else {
        for path in files_written {
            println!("wrote {path}");
        }
    }
    Ok(())
}

fn write_generic_agents(
    config: &PatinaConfig,
    force: bool,
    files_written: &mut Vec<String>,
) -> Result<()> {
    let path = config.knowledge_dir().join("AGENTS.md");
    write_if_needed(&path, GENERIC_AGENTS, force, files_written)
}

fn write_claude_code(force: bool, files_written: &mut Vec<String>) -> Result<()> {
    let path = PathBuf::from(".claude/CLAUDE.md");
    if force || !path.exists() {
        write_if_needed(&path, CLAUDE_CODE_SNIPPET, true, files_written)?;
    } else {
        let existing = fs::read_to_string(&path)
            .with_context(|| format!("failed to read {}", path.display()))?;
        if !existing.contains("patina query") {
            let mut updated = existing;
            updated.push_str("\n\n");
            updated.push_str(CLAUDE_CODE_SNIPPET);
            fs::write(&path, updated)
                .with_context(|| format!("failed to write {}", path.display()))?;
            files_written.push(path.display().to_string());
        }
    }
    Ok(())
}

fn write_if_needed(
    path: &Path,
    contents: &str,
    force: bool,
    files_written: &mut Vec<String>,
) -> Result<()> {
    if path.exists() && !force {
        return Ok(());
    }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }
    fs::write(path, contents).with_context(|| format!("failed to write {}", path.display()))?;
    files_written.push(path.display().to_string());
    Ok(())
}

const GENERIC_AGENTS: &str = r#"# Agent Instructions

Use the Patina CLI as the contract for this knowledge base.

- Search with `patina query "terms" --json`.
- Read exact files with `patina read path/to/page.md --json`.
- Validate before relying on content with `patina lint --json`.
- Check stale pages and source drift with `patina stale --json`.
"#;

const CLAUDE_CODE_SNIPPET: &str = r#"# Patina Knowledge

Before answering project-context questions, use `patina query`, then `patina read` for the cited pages. Run `patina lint` before editing knowledge files and `patina stale` when source freshness matters.
"#;
