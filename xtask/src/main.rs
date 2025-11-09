use anyhow::Result;
use clap::{Parser, Subcommand};
use patina_core::project::ProjectHandle;
use patina_core::state::AppState;
use patina_core::{llm::LlmDriver, telemetry};
use std::sync::Arc;
use tempfile::TempDir;
use tokio::runtime::Runtime;
use tracing::info;
use tracing_subscriber::EnvFilter;

#[derive(Parser, Debug)]
#[command(name = "xtask", version, about = "Automation helpers for Patina")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Run a lightweight smoke test that exercises the Patina core logic.
    Smoke,
}

fn main() -> Result<()> {
    telemetry::init_tracing(EnvFilter::new("info"))?;
    let cli = Cli::parse();

    match cli.command {
        Commands::Smoke => smoke_test(),
    }
}

fn smoke_test() -> Result<()> {
    let runtime = Runtime::new()?;
    let temp_dir = TempDir::new()?;
    let project = ProjectHandle::create(temp_dir.path(), "SmokeProject")?;
    let store = project.transcript_store();
    let driver = runtime.block_on(LlmDriver::fake());
    let state = Arc::new(AppState::with_store(project, store, driver));

    runtime.block_on(state.send_user_message("ping from xtask"))?;
    if let Some(conversation) = state.active_conversation() {
        info!(
            "messages" = conversation.messages.len(),
            "smoke test conversation saved"
        );
    }

    Ok(())
}
