pub mod agent;
pub mod doctor;
pub mod index;
pub mod init;
pub mod lint;
pub mod query;
pub mod read;
pub mod skills;
pub mod stale;
pub mod status;

use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(name = "patina")]
#[command(version, about)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

impl Cli {
    pub fn json(&self) -> bool {
        match &self.command {
            Commands::Init(args) => args.json,
            Commands::Status(args) => args.json,
            Commands::Lint(args) => args.json,
            Commands::Index(args) => args.json,
            Commands::Query(args) => args.json,
            Commands::Read(args) => args.json,
            Commands::Stale(args) => args.json,
            Commands::Doctor(args) => args.json,
            Commands::InstallSkills(args) => args.json,
            Commands::InstallAgent(args) => args.json,
        }
    }

    pub fn command_name(&self) -> &'static str {
        match &self.command {
            Commands::Init(_) => "init",
            Commands::Status(_) => "status",
            Commands::Lint(_) => "lint",
            Commands::Index(_) => "index",
            Commands::Query(_) => "query",
            Commands::Read(_) => "read",
            Commands::Stale(_) => "stale",
            Commands::Doctor(_) => "doctor",
            Commands::InstallSkills(_) => "install-skills",
            Commands::InstallAgent(_) => "install-agent",
        }
    }
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    Init(init::InitArgs),
    Status(status::StatusArgs),
    Lint(lint::LintArgs),
    Index(index::IndexArgs),
    Query(query::QueryArgs),
    Read(read::ReadArgs),
    Stale(stale::StaleArgs),
    Doctor(doctor::DoctorArgs),
    #[command(name = "install-skills")]
    InstallSkills(skills::InstallSkillsArgs),
    #[command(name = "install-agent")]
    InstallAgent(agent::InstallAgentArgs),
}
