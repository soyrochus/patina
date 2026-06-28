use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::Args;
use serde::Serialize;
use std::fs;

use crate::config::PatinaConfig;
use crate::discovery::frontmatter;
use crate::output::{JsonEnvelope, print_json};
use crate::read::path as read_path;

#[derive(Debug, Args)]
pub struct ReadArgs {
    pub path: PathBuf,
    #[arg(long)]
    pub json: bool,
}

#[derive(Debug, Serialize)]
struct ReadData {
    content: String,
    front_matter: std::collections::BTreeMap<String, serde_yaml::Value>,
}

pub fn run(args: ReadArgs, config: &PatinaConfig) -> Result<()> {
    let resolved =
        read_path::resolve_read_path(&args.path, &config.knowledge_dir(), &config.security)?;
    let content = fs::read_to_string(&resolved)
        .with_context(|| format!("failed to read {}", resolved.display()))?;
    let parsed = frontmatter::parse_file(&resolved, &content)
        .map_err(|error| anyhow::anyhow!(error.message))?;

    if args.json {
        print_json(&JsonEnvelope::success(
            "read",
            ReadData {
                content,
                front_matter: parsed.front_matter,
            },
            Vec::new(),
        ))?;
    } else {
        print!("{content}");
    }
    Ok(())
}
