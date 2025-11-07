use patina_core::telemetry;
use patina_core::{llm::LlmDriver, state::AppState, store::TranscriptStore};
use std::sync::Arc;
use tokio::runtime::Runtime;
use tracing_subscriber::EnvFilter;

mod app;

use app::PatinaEguiApp;

fn main() -> anyhow::Result<()> {
    telemetry::init_tracing(EnvFilter::from_default_env())?;

    let runtime = Arc::new(Runtime::new()?);
    let store = TranscriptStore::default();
    let driver = runtime.block_on(LlmDriver::from_environment());

    let app_state = Arc::new(AppState::new(store, driver));
    let runtime_for_ui = runtime.clone();

    eframe::run_native(
        "Patina",
        eframe::NativeOptions::default(),
        Box::new(move |_cc| {
            Box::new(PatinaEguiApp::new(
                app_state.clone(),
                runtime_for_ui.clone(),
            ))
        }),
    )
    .map_err(|err| anyhow::anyhow!(err.to_string()))?;

    Ok(())
}
