use eframe::egui;
use patina_app::{PatinaEguiApp, UiSettingsStore};
use patina_core::telemetry;
use patina_core::{llm::LlmDriver, state::AppState, store::TranscriptStore};
use std::sync::Arc;
use tokio::runtime::Runtime;
use tracing_subscriber::EnvFilter;

fn main() -> anyhow::Result<()> {
    telemetry::init_tracing(EnvFilter::from_default_env())?;

    let runtime = Arc::new(Runtime::new()?);
    let store = TranscriptStore::default();
    let driver = runtime.block_on(LlmDriver::from_environment());

    let app_state = Arc::new(AppState::new(store, driver));
    let runtime_for_ui = runtime.clone();
    let mut settings = Some(UiSettingsStore::load());
    let initial_size = settings.as_ref().unwrap().data().window_size;
    let inner_size = egui::vec2(initial_size[0].max(1024.0), initial_size[1].max(720.0));
    let viewport = egui::ViewportBuilder::default()
        .with_inner_size(inner_size)
        .with_min_inner_size(egui::vec2(1024.0, 720.0));
    let native_options = eframe::NativeOptions {
        viewport,
        follow_system_theme: true,
        default_theme: settings
            .as_ref()
            .unwrap()
            .data()
            .theme_mode
            .fallback_theme(),
        ..Default::default()
    };

    eframe::run_native(
        "Patina",
        native_options,
        Box::new(move |_cc| {
            let settings_store = settings.take().expect("UI settings already consumed");
            Box::new(PatinaEguiApp::new(
                app_state.clone(),
                runtime_for_ui.clone(),
                settings_store,
            ))
        }),
    )
    .map_err(|err| anyhow::anyhow!(err.to_string()))?;

    Ok(())
}
