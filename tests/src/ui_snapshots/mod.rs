use egui::{self, RawInput};
use patina::ui::ThemeMode;
use patina::{render_ui, PatinaEguiApp, UiSettingsStore};
use patina_core::{llm::LlmDriver, project::ProjectHandle, state::AppState};
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use tempfile::TempDir;
use tokio::runtime::Runtime;

fn test_runtime() -> Runtime {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("runtime")
}

fn build_app(theme: ThemeMode) -> PatinaEguiApp {
    let runtime = Arc::new(test_runtime());
    let temp_dir = TempDir::new().expect("temp dir");
    let project = ProjectHandle::create(temp_dir.path(), "SnapshotProject").expect("project");
    let store = project.transcript_store();
    let driver = runtime.block_on(LlmDriver::fake());
    {
        let seeded_driver = driver.clone();
        let state = AppState::with_store(project.clone(), store, seeded_driver);
        runtime
            .block_on(state.send_user_message("Seed snapshot conversation"))
            .expect("seed message");
    }
    let mut settings = UiSettingsStore::temporary();
    settings.data_mut().theme_mode = theme;
    PatinaEguiApp::new(Some(project), driver, runtime, settings)
}

fn capture_snapshot(app: &mut PatinaEguiApp) -> String {
    let ctx = egui::Context::default();
    let output = ctx.run(RawInput::default(), |ctx| {
        render_ui(ctx, app);
    });
    summarize_output(&ctx, &output)
}

fn summarize_output(ctx: &egui::Context, output: &egui::FullOutput) -> String {
    let mut lines = Vec::new();
    lines.push(format!(
        "textures:set={} free={}",
        output.textures_delta.set.len(),
        output.textures_delta.free.len()
    ));
    let clipped = ctx.tessellate(output.shapes.clone(), 1.0);
    lines.push(format!("primitives={}", clipped.len()));
    for (idx, primitive) in clipped.iter().take(64).enumerate() {
        let clip = primitive.clip_rect;
        let clip_desc = format!(
            "[{:.1},{:.1},{:.1},{:.1}]",
            clip.min.x, clip.min.y, clip.max.x, clip.max.y
        );
        let summary = match &primitive.primitive {
            egui::epaint::Primitive::Mesh(mesh) => format!(
                "mesh:{}v {}i {}",
                mesh.vertices.len(),
                mesh.indices.len(),
                clip_desc
            ),
            egui::epaint::Primitive::Callback(_) => format!("callback {}", clip_desc),
        };
        lines.push(format!("{idx}:{summary}"));
    }
    lines.join("\n")
}

fn assert_snapshot(name: &str, actual: &str) {
    let path = snapshot_path(name);
    if let Ok(expected) = fs::read_to_string(&path) {
        assert_eq!(
            actual.trim_end(),
            expected.trim_end(),
            "snapshot {} drifted",
            name
        );
    } else {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("create snapshot dir");
        }
        fs::write(&path, actual).expect("write snapshot");
        panic!(
            "snapshot {} created at {}. Re-run tests.",
            name,
            path.display()
        );
    }
}

fn snapshot_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("ui_snapshots")
        .join(format!("{name}.snapshot"))
}

#[test]
fn renders_dark_ui_snapshot() {
    let mut app = build_app(ThemeMode::Dark);
    let snapshot = capture_snapshot(&mut app);
    assert_snapshot("dark", &snapshot);
}

#[test]
fn renders_light_ui_snapshot() {
    let mut app = build_app(ThemeMode::Light);
    let snapshot = capture_snapshot(&mut app);
    assert_snapshot("light", &snapshot);
}
