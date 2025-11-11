use clap::{Parser, Subcommand};
use eframe::egui;
use patina::{
    config::{load_provider_config, load_ui_settings, Scope, UiSettings},
    logo_png_bytes, PatinaEguiApp,
};
use patina_core::llm::LlmDriver;
use patina_core::project::ProjectHandle;
use patina_core::telemetry;
use std::fs::File;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::runtime::Runtime;
use tracing_subscriber::EnvFilter;

#[derive(Parser, Debug)]
#[command(name = "Patina", version)]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
    #[arg(long)]
    project: Option<PathBuf>,
    #[arg(long)]
    new: Option<PathBuf>,
    #[arg(long)]
    name: Option<String>,
}

#[derive(Subcommand, Debug)]
enum Command {
    Export { project: PathBuf, out: PathBuf },
    Import { zip: PathBuf, into: PathBuf },
}

fn load_application_icon() -> Option<egui::IconData> {
    let image = image::load_from_memory(logo_png_bytes()).ok()?.to_rgba8();
    let (width, height) = (image.width(), image.height());
    Some(egui::IconData {
        rgba: image.into_raw(),
        width,
        height,
    })
}

fn main() -> anyhow::Result<()> {
    telemetry::init_tracing(EnvFilter::from_default_env())?;

    let cli = Cli::parse();

    match &cli.command {
        Some(Command::Export { project, out }) => {
            let handle = ProjectHandle::open(project)?;
            let file = File::create(out)?;
            handle.export_zip(file)?;
            return Ok(());
        }
        Some(Command::Import { zip, into }) => {
            let file = File::open(zip)?;
            let imported = ProjectHandle::import_zip(file, into)?;
            println!(
                "Imported project {} at {}",
                imported.name(),
                imported.paths().root.display()
            );
            return Ok(());
        }
        None => {}
    }

    let runtime = Arc::new(Runtime::new()?);
    let driver = runtime.block_on(LlmDriver::from_environment());

    let scope = Scope::User;
    let mut ui_settings = runtime.block_on(load_ui_settings(&scope))?;
    let provider_config = runtime.block_on(load_provider_config(&scope))?;
    let project = resolve_project(&cli, &mut ui_settings)?;
    let runtime_for_ui = runtime.clone();
    let mut settings = Some(ui_settings);
    let mut provider = Some(provider_config);
    let scope_for_ui = scope.clone();
    let initial_size = settings.as_ref().unwrap().window_size;
    let inner_size = egui::vec2(initial_size[0].max(1024.0), initial_size[1].max(720.0));
    let mut viewport = egui::ViewportBuilder::default()
        .with_inner_size(inner_size)
        .with_min_inner_size(egui::vec2(1024.0, 720.0));
    if let Some(icon) = load_application_icon() {
        viewport = viewport.with_icon(icon);
    }
    let native_options = eframe::NativeOptions {
        viewport,
        follow_system_theme: true,
        default_theme: settings.as_ref().unwrap().theme_mode.fallback_theme(),
        ..Default::default()
    };

    let window_title = project
        .as_ref()
        .map(|handle| format!("Patina — {}", handle.name()))
        .unwrap_or_else(|| "Patina".to_string());

    eframe::run_native(
        &window_title,
        native_options,
        Box::new(move |_cc| {
            let ui_settings = settings.take().expect("UI settings already consumed");
            let provider_config = provider.take().expect("provider config already consumed");
            Box::new(PatinaEguiApp::new(
                project.clone(),
                driver.clone(),
                runtime_for_ui.clone(),
                scope_for_ui.clone(),
                ui_settings,
                provider_config,
            ))
        }),
    )
    .map_err(|err| anyhow::anyhow!(err.to_string()))?;

    Ok(())
}

fn resolve_project(cli: &Cli, settings: &mut UiSettings) -> anyhow::Result<Option<ProjectHandle>> {
    if let Some(new_path) = &cli.new {
        let name = cli
            .name
            .clone()
            .or_else(|| infer_name(new_path))
            .ok_or_else(|| anyhow::anyhow!("--name is required when creating a project"))?;
        return ProjectHandle::create(new_path, &name).map(Some);
    }

    if let Some(path) = &cli.project {
        return ProjectHandle::open(path).map(Some);
    }

    if let Some(stored) = settings.current_project.clone() {
        match ProjectHandle::open(Path::new(&stored)) {
            Ok(handle) => return Ok(Some(handle)),
            Err(_) => {
                settings.current_project = None;
                settings.recent_projects.retain(|entry| entry != &stored);
            }
        }
    }

    Ok(None)
}

fn infer_name(path: &Path) -> Option<String> {
    path.file_name().and_then(|os| {
        let name = os.to_str()?;
        if let Some(stripped) = name.strip_suffix(".pat") {
            Some(stripped.to_string())
        } else {
            Some(name.to_string())
        }
    })
}
