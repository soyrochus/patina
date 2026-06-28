use anyhow::{Result, bail};
use clap::Args;

use crate::cli::skills::{InstallRequest, run_request};
use crate::config::PatinaConfig;

#[derive(Debug, Args)]
pub struct InstallAgentArgs {
    #[arg(long)]
    pub json: bool,
    #[arg(long)]
    pub force: bool,
    #[arg(long)]
    pub agent: Option<String>,
}

pub fn run(args: InstallAgentArgs, config: &PatinaConfig) -> Result<()> {
    let targets = match args.agent {
        Some(agent) if agent == "claude-code" => vec!["claude-code".to_string()],
        Some(agent) => bail!(
            "unrecognised agent `{agent}`; supported install-skills targets: github-copilot, codex, claude-code, all"
        ),
        None => Vec::new(),
    };

    run_request(
        InstallRequest {
            targets,
            json: args.json,
            force: args.force,
            command_name: "install-agent",
            deprecated_install_agent: true,
        },
        config,
    )
}
