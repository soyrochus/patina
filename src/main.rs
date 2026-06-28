use std::process::ExitCode;

use anyhow::Result;
use clap::{CommandFactory, Parser};
use patina::cli::{Cli, Commands};
use patina::config::PatinaConfig;
use patina::output::{ErrorEntry, JsonEnvelope};

fn main() -> ExitCode {
    if std::env::args_os().len() == 1 {
        let mut command = Cli::command();
        if let Err(error) = command.print_help() {
            eprintln!("error: {error}");
            return ExitCode::from(1);
        }
        println!();
        return ExitCode::SUCCESS;
    }

    let cli = Cli::parse();
    let json = cli.json();
    let command = cli.command_name().to_string();

    match run(cli) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            if json {
                let envelope: JsonEnvelope<serde_json::Value> =
                    JsonEnvelope::<serde_json::Value>::failure(
                        &command,
                        ErrorEntry::from_anyhow(&error),
                    );
                match serde_json::to_string_pretty(&envelope) {
                    Ok(serialized) => eprintln!("{serialized}"),
                    Err(serialization_error) => {
                        eprintln!("error: {error}");
                        eprintln!("error: failed to serialize JSON error: {serialization_error}");
                    }
                }
            } else {
                eprintln!("error: {error}");
            }
            ExitCode::from(1)
        }
    }
}

fn run(cli: Cli) -> Result<()> {
    let config = PatinaConfig::load_for_current_dir()?;
    config.ensure_supported()?;

    match cli.command {
        Commands::Init(args) => patina::cli::init::run(args, &config),
        Commands::Status(args) => patina::cli::status::run(args, &config),
        Commands::Lint(args) => patina::cli::lint::run(args, &config),
        Commands::Index(args) => patina::cli::index::run(args, &config),
        Commands::Query(args) => patina::cli::query::run(args),
        Commands::Read(args) => patina::cli::read::run(args, &config),
        Commands::Stale(args) => patina::cli::stale::run(args, &config),
        Commands::Doctor(args) => patina::cli::doctor::run(args, &config),
        Commands::InstallAgent(args) => patina::cli::agent::run(args, &config),
    }
}
