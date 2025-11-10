const ABOUT_WINDOW_WIDTH: f32 = 640.0;
const ABOUT_WINDOW_HEIGHT: f32 = 360.0;
const ABOUT_LOGO_MAX_WIDTH: f32 = 240.0;

use crate::{
    assets,
    settings::SettingsPanel,
    ui::{
        ChatPanel, ChatPanelState, InputBar, InputBarOutput, InputBarState, McpSidebarEntry,
        McpStatus, MenuBar, MenuBarOutput, MenuBarState, Sidebar, SidebarOutput, SidebarState,
        ThemeMode, ThemePalette,
    },
};
use anyhow::{anyhow, Result};
use directories::ProjectDirs;
use egui::{self, Margin, RichText, Stroke, TextureOptions};
use egui_commonmark::CommonMarkCache;
use patina_core::project::ProjectHandle;
use patina_core::state::AppState;
use patina_core::{llm::LlmDriver, LlmStatus};
use rfd::FileDialog;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::runtime::Runtime;
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};
use tracing::{error, warn};
use uuid::Uuid;

const SETTINGS_FLUSH_INTERVAL: Duration = Duration::from_secs(2);
const SPLASH_DURATION: Duration = Duration::from_secs(1);
const MANUAL_DISMISS_DELAY: Duration = Duration::from_millis(150);

#[derive(Clone, Copy)]
enum AboutMode {
    Splash { opened: Instant },
    Manual { opened: Instant },
}

pub struct PatinaEguiApp {
    state: Option<Arc<AppState>>,
    driver: LlmDriver,
    runtime: Arc<Runtime>,
    tx: UnboundedSender<Result<()>>,
    rx: UnboundedReceiver<Result<()>>,
    menu_state: MenuBarState,
    sidebar_state: SidebarState,
    input_state: InputBarState,
    chat_panel_state: ChatPanelState,
    markdown_cache: CommonMarkCache,
    settings: UiSettingsStore,
    settings_panel: SettingsPanel,
    palette: ThemePalette,
    system_theme: Option<eframe::Theme>,
    error: Option<String>,
    mcp_entries: Vec<McpSidebarEntry>,
    pinned_lookup: HashSet<Uuid>,
    last_settings_flush: Instant,
    logo_texture: Option<egui::TextureHandle>,
    about_mode: Option<AboutMode>,
    pending_exit: bool,
    pending_title: Option<String>,
    current_workspace: Option<String>,
}

impl PatinaEguiApp {
    pub fn new(
        project: Option<ProjectHandle>,
        driver: LlmDriver,
        runtime: Arc<Runtime>,
        mut settings: UiSettingsStore,
    ) -> Self {
        let settings_panel = SettingsPanel::new();
        let global_theme = settings_panel.app_settings().theme;
        if settings.data().theme_mode != global_theme {
            settings.data_mut().theme_mode = global_theme;
        }
        let (tx, rx) = unbounded_channel();
        let mut app = Self {
            state: None,
            driver,
            runtime,
            tx,
            rx,
            menu_state: MenuBarState {
                theme_mode: global_theme,
            },
            sidebar_state: {
                let mut sidebar = SidebarState::new();
                sidebar.collapsed = !settings.data().sidebar_visible;
                sidebar
            },
            input_state: InputBarState::new(
                settings.data().model.clone(),
                settings.data().temperature,
                settings.data().retain_input,
            ),
            chat_panel_state: ChatPanelState::default(),
            markdown_cache: CommonMarkCache::default(),
            settings,
            settings_panel,
            palette: match global_theme {
                ThemeMode::Light => ThemePalette::for_light(),
                _ => ThemePalette::for_dark(),
            },
            system_theme: None,
            error: None,
            mcp_entries: default_mcp_entries(),
            pinned_lookup: HashSet::new(),
            last_settings_flush: Instant::now(),
            logo_texture: None,
            about_mode: Some(AboutMode::Splash {
                opened: Instant::now(),
            }),
            pending_exit: false,
            pending_title: None,
            current_workspace: None,
        };
        app.refresh_pinned_cache();
        if let Some(project) = project {
            app.activate_project(project);
        } else {
            {
                let data = app.settings.data_mut();
                data.current_project = None;
                data.last_conversation = None;
            }
            app.settings_panel.set_project(None);
            app.pending_title = Some("Patina".to_string());
            app.current_workspace = None;
        }
        app
    }

    fn process_background_results(&mut self) {
        while let Ok(result) = self.rx.try_recv() {
            if let Err(err) = result {
                error!(error = ?err, "Failed to send message");
                self.error = Some(err.to_string());
            } else {
                self.error = None;
            }
        }
    }

    fn handle_shortcuts(&mut self, ctx: &egui::Context) {
        let shortcuts = ctx.input(|input| {
            let command_only =
                input.modifiers.command && !input.modifiers.shift && !input.modifiers.alt;
            let new_chat = command_only && input.key_pressed(egui::Key::N);
            let toggle_sidebar = command_only && input.key_pressed(egui::Key::M);
            let focus_search = command_only && input.key_pressed(egui::Key::K);
            (new_chat, toggle_sidebar, focus_search)
        });
        if shortcuts.0 {
            self.create_new_chat();
        }
        if shortcuts.1 {
            self.toggle_sidebar();
        }
        if shortcuts.2 {
            self.sidebar_state.request_search_focus();
        }
    }

    fn ensure_logo_texture(&mut self, ctx: &egui::Context) {
        if self.logo_texture.is_some() {
            return;
        }
        let image = assets::logo_color_image().clone();
        let texture = ctx.load_texture("patina_logo", image, TextureOptions::LINEAR);
        self.logo_texture = Some(texture);
    }

    fn apply_theme(&mut self, ctx: &egui::Context) {
        let resolved_mode = match self.menu_state.theme_mode {
            ThemeMode::System => match self.system_theme.unwrap_or(eframe::Theme::Dark) {
                eframe::Theme::Light => ThemeMode::Light,
                eframe::Theme::Dark => ThemeMode::Dark,
            },
            mode => mode,
        };
        self.palette = match resolved_mode {
            ThemeMode::Light => ThemePalette::for_light(),
            _ => ThemePalette::for_dark(),
        };
        ctx.set_visuals(
            self.palette
                .visuals(matches!(resolved_mode, ThemeMode::Dark)),
        );
    }

    fn layout(&mut self, ctx: &egui::Context) {
        let project_loaded = self.state.is_some();
        let llm_status = self.driver.status();
        egui::TopBottomPanel::top("menu_bar")
            .frame(
                egui::Frame::none()
                    .fill(self.palette.surface)
                    .inner_margin(Margin::symmetric(12.0, 8.0)),
            )
            .show(ctx, |ui| {
                let output = MenuBar::show(
                    ui,
                    &mut self.menu_state,
                    self.logo_texture.as_ref(),
                    project_loaded,
                    self.current_workspace.as_deref(),
                );
                self.handle_menu_output(output);
                if let Some(err) = &self.error {
                    ui.colored_label(self.palette.warning, err);
                }
                if let LlmStatus::Unconfigured(message) = &llm_status {
                    ui.add_space(4.0);
                    ui.colored_label(self.palette.warning, message);
                    ui.label(
                        RichText::new(
                            "Set OPENAI_/AZURE_ env vars or create patina.yaml to enable AI.",
                        )
                        .color(self.palette.text_secondary)
                        .small(),
                    );
                }
            });

        if let Some(state) = self.state.as_ref() {
            let active_conversation = state.active_conversation();

            if self.sidebar_state.collapsed {
                egui::SidePanel::left("sidebar_collapsed")
                    .resizable(false)
                    .exact_width(36.0)
                    .frame(
                        egui::Frame::none()
                            .fill(self.palette.sidebar_background)
                            .inner_margin(Margin::same(6.0)),
                    )
                    .show(ctx, |ui| {
                        ui.vertical_centered(|ui| {
                            if ui.button("⟩").clicked() {
                                self.sidebar_state.collapsed = false;
                                self.set_sidebar_visibility(true);
                            }
                        });
                    });
            } else {
                let summaries = state.conversation_summaries();
                let pinned_order = self.settings.data().pinned_chats.clone();

                let response = egui::SidePanel::left("sidebar")
                    .resizable(true)
                    .min_width(220.0)
                    .default_width(self.settings.data().sidebar_width)
                    .frame(
                        egui::Frame::none()
                            .fill(self.palette.sidebar_background)
                            .inner_margin(Margin::same(12.0)),
                    )
                    .show(ctx, |ui| {
                        ui.horizontal(|ui| {
                            if ui.button("⟨").clicked() {
                                self.sidebar_state.collapsed = true;
                                self.set_sidebar_visibility(false);
                            }
                            ui.label(RichText::new("Workspace").strong());
                        });
                        ui.add_space(8.0);
                        let active_id = active_conversation
                            .as_ref()
                            .map(|conversation| conversation.id);
                        let sidebar_output = Sidebar::show(
                            ui,
                            &mut self.sidebar_state,
                            &self.palette,
                            &summaries,
                            &self.pinned_lookup,
                            &pinned_order,
                            &mut self.mcp_entries,
                            active_id,
                        );
                        self.handle_sidebar_output(sidebar_output);
                    });

                let width = response.response.rect.width();
                if (self.settings.data().sidebar_width - width).abs() > 1.0 {
                    self.settings.data_mut().sidebar_width = width;
                }
            }

            egui::TopBottomPanel::bottom("chat_input")
                .frame(
                    egui::Frame::none()
                        .fill(self.palette.surface)
                        .inner_margin(Margin::same(12.0)),
                )
                .show(ctx, |ui| {
                    let input_output = InputBar::show(ui, &mut self.input_state, &self.palette);
                    self.handle_input_output(input_output);
                });

            egui::CentralPanel::default()
                .frame(
                    egui::Frame::none()
                        .fill(self.palette.background)
                        .inner_margin(Margin::same(16.0)),
                )
                .show(ctx, |ui| {
                    if let Some(conversation) = active_conversation.as_ref() {
                        let chat_output = ChatPanel::show(
                            ui,
                            &self.palette,
                            &mut self.chat_panel_state,
                            conversation,
                            &mut self.markdown_cache,
                        );
                        if chat_output.load_older {
                            self.chat_panel_state
                                .request_more(conversation.messages.len());
                        }
                    } else {
                        ui.centered_and_justified(|ui| {
                            ui.label("Start a conversation to see the transcript here.");
                        });
                    }
                });
        } else {
            egui::CentralPanel::default()
                .frame(
                    egui::Frame::none()
                        .fill(self.palette.background)
                        .inner_margin(Margin::same(32.0)),
                )
                .show(ctx, |ui| {
                    ui.centered_and_justified(|ui| {
                        ui.label("Create or open a project from the File menu to get started.");
                    });
                });
        }
    }

    fn handle_menu_output(&mut self, output: MenuBarOutput) {
        if output.new_project {
            self.prompt_new_project();
        }
        if output.open_project {
            self.prompt_open_project();
        }
        if output.new_chat {
            self.create_new_chat();
        }
        if output.toggle_sidebar {
            self.toggle_sidebar();
        }
        if output.focus_search {
            self.sidebar_state.request_search_focus();
        }
        if output.clear_input {
            self.input_state.draft.clear();
        }
        if output.show_about {
            self.about_mode = Some(AboutMode::Manual {
                opened: Instant::now(),
            });
        }
        if output.show_settings {
            self.settings_panel.open();
        }
        if output.exit {
            self.pending_exit = true;
        }
        if let Some(mode) = output.theme_changed {
            self.menu_state.theme_mode = mode;
            self.settings.data_mut().theme_mode = mode;
            if let Err(err) = self.settings_panel.apply_theme_selection(mode) {
                error!(error = ?err, "Failed to persist theme change");
            }
        }
    }

    fn handle_sidebar_output(&mut self, output: SidebarOutput) {
        let Some(state) = self.state.as_ref().cloned() else {
            return;
        };
        if let Some(id) = output.selected_chat {
            state.select_conversation(id);
            self.update_last_conversation(id);
        }
        if let Some((id, name)) = output.rename {
            if let Err(err) = state.rename_conversation(id, name.clone()) {
                self.error = Some(err.to_string());
            } else {
                self.error = None;
            }
        }
        if let Some(id) = output.delete {
            match state.delete_conversation(id) {
                Ok(true) => {
                    self.unpin_chat(id);
                    if let Some(active) = state.active_conversation() {
                        self.update_last_conversation(active.id);
                    } else {
                        self.settings.data_mut().last_conversation = None;
                    }
                }
                Ok(false) => {}
                Err(err) => self.error = Some(err.to_string()),
            }
        }
        if let Some((dragged, target)) = output.reorder {
            if let Err(err) = state.reorder_conversations(dragged, target) {
                self.error = Some(err.to_string());
            }
        }
        if let Some(id) = output.pin {
            self.pin_chat(id);
        }
        if let Some(id) = output.unpin {
            self.unpin_chat(id);
        }
    }

    fn handle_input_output(&mut self, output: InputBarOutput) {
        if output.send {
            self.submit_message();
            if !self.input_state.retain_input {
                self.input_state.draft.clear();
            }
        }
        if output.clear {
            self.input_state.draft.clear();
        }
        if let Some(model) = output.model_changed {
            self.settings.data_mut().model = model;
        }
        if let Some(temp) = output.temperature_changed {
            self.settings.data_mut().temperature = temp;
        }
        if self.settings.data().retain_input != self.input_state.retain_input {
            self.settings.data_mut().retain_input = self.input_state.retain_input;
        }
    }

    fn submit_message(&mut self) {
        let content = self.input_state.draft.trim();
        if content.is_empty() {
            return;
        }
        let Some(state) = self.state.as_ref().cloned() else {
            return;
        };
        let payload = content.to_owned();
        let tx = self.tx.clone();
        self.runtime.spawn(async move {
            let result = state.send_user_message(payload).await;
            if tx.send(result).is_err() {
                warn!("UI dropped before send completion");
            }
        });
    }

    fn create_new_chat(&mut self) {
        if let Some(state) = self.state.as_ref() {
            let id = state.start_new_conversation();
            self.update_last_conversation(id);
        }
    }

    fn toggle_sidebar(&mut self) {
        self.sidebar_state.collapsed = !self.sidebar_state.collapsed;
        self.set_sidebar_visibility(!self.sidebar_state.collapsed);
    }

    fn set_sidebar_visibility(&mut self, visible: bool) {
        if self.settings.data().sidebar_visible != visible {
            self.settings.data_mut().sidebar_visible = visible;
        }
    }

    fn update_last_conversation(&mut self, id: Uuid) {
        self.settings.data_mut().last_conversation = Some(id);
    }

    fn pin_chat(&mut self, id: Uuid) {
        if !self.settings.data().pinned_chats.contains(&id) {
            let list = &mut self.settings.data_mut().pinned_chats;
            list.insert(0, id);
            self.refresh_pinned_cache();
        }
    }

    fn unpin_chat(&mut self, id: Uuid) {
        if self.settings.data().pinned_chats.contains(&id) {
            let list = &mut self.settings.data_mut().pinned_chats;
            list.retain(|candidate| candidate != &id);
            self.refresh_pinned_cache();
        }
    }

    fn refresh_pinned_cache(&mut self) {
        self.pinned_lookup = self.settings.data().pinned_chats.iter().copied().collect();
    }

    fn activate_project(&mut self, project: ProjectHandle) {
        self.settings_panel.set_project(Some(&project));
        let last_selected = self.settings.data().last_conversation;
        let state = Arc::new(AppState::new(project.clone(), self.driver.clone()));
        if let Some(last) = last_selected {
            state.select_conversation(last);
        }
        self.state = Some(state);
        self.error = None;
        self.remember_project(&project);
        self.refresh_pinned_cache();
        self.pending_title = Some(format!("Patina — {}", project.name()));
        self.current_workspace = Some(project.name().to_string());
        self.sync_last_conversation();
    }

    fn remember_project(&mut self, project: &ProjectHandle) {
        let root = project.paths().root.to_string_lossy().to_string();
        let data = self.settings.data_mut();
        data.current_project = Some(root.clone());
        data.recent_projects.retain(|entry| entry != &root);
        data.recent_projects.insert(0, root);
        if data.recent_projects.len() > 10 {
            data.recent_projects.truncate(10);
        }
    }

    fn sync_last_conversation(&mut self) {
        let active = self
            .state
            .as_ref()
            .and_then(|state| state.active_conversation().map(|c| c.id));
        self.settings.data_mut().last_conversation = active;
    }

    fn prompt_new_project(&mut self) {
        let default_dir = Self::default_project_directory();
        let mut dialog = FileDialog::new();
        dialog = dialog
            .set_title("Create Patina Project")
            .add_filter("Patina Project", &["pat"])
            .set_file_name("NewProject.pat");
        if default_dir.exists() {
            dialog = dialog.set_directory(default_dir);
        }
        if let Some(path) = dialog.save_file() {
            match self.create_project_from_path(&path) {
                Ok(project) => self.activate_project(project),
                Err(err) => self.error = Some(err.to_string()),
            }
        }
    }

    fn prompt_open_project(&mut self) {
        let default_dir = Self::default_project_directory();
        let mut dialog = FileDialog::new();
        dialog = dialog
            .set_title("Open Patina Project")
            .add_filter("Patina Project", &["pat"]);
        if default_dir.exists() {
            dialog = dialog.set_directory(default_dir);
        }
        if let Some(path) = dialog.pick_file() {
            match ProjectHandle::open(&path) {
                Ok(project) => self.activate_project(project),
                Err(err) => self.error = Some(err.to_string()),
            }
        }
    }

    fn create_project_from_path(&self, path: &Path) -> Result<ProjectHandle> {
        let name = if path.extension().and_then(|ext| ext.to_str()) == Some("pat") {
            path.file_stem()
                .and_then(|stem| stem.to_str())
                .ok_or_else(|| anyhow!("project file must have a valid name"))?
                .to_string()
        } else {
            path.file_name()
                .and_then(|stem| stem.to_str())
                .ok_or_else(|| anyhow!("project path must have a valid name"))?
                .to_string()
        };
        ProjectHandle::create(path, &name)
    }

    fn default_project_directory() -> PathBuf {
        if let Some(dirs) = ProjectDirs::from("com", "Patina", "Patina") {
            dirs.data_local_dir().join("projects")
        } else {
            env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
        }
    }

    fn capture_window_size(&mut self, ctx: &egui::Context) {
        if let Some(rect) = ctx.input(|input| input.viewport().inner_rect) {
            let size = rect.size();
            let stored = self.settings.data().window_size;
            if (stored[0] - size.x).abs() > 1.0 || (stored[1] - size.y).abs() > 1.0 {
                self.settings.data_mut().window_size = [size.x, size.y];
            }
        }
    }

    fn flush_settings_if_needed(&mut self) {
        if self.settings.is_dirty() && self.last_settings_flush.elapsed() >= SETTINGS_FLUSH_INTERVAL
        {
            if let Err(err) = self.settings.persist() {
                error!(error = ?err, "Failed to persist UI settings");
            } else {
                self.last_settings_flush = Instant::now();
            }
        }
    }

    fn render(&mut self, ctx: &egui::Context) {
        self.apply_theme(ctx);
        self.process_background_results();
        if !matches!(self.about_mode, Some(AboutMode::Manual { .. })) {
            self.handle_shortcuts(ctx);
        }
        self.ensure_logo_texture(ctx);
        self.layout(ctx);
        self.show_settings_panel(ctx);
        self.draw_about_dialog(ctx);
        self.capture_window_size(ctx);
        self.flush_settings_if_needed();
        if let Some(title) = self.pending_title.take() {
            ctx.send_viewport_cmd(egui::ViewportCommand::Title(title));
        }
        if self.pending_exit {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            self.pending_exit = false;
        }
    }

    fn draw_about_dialog(&mut self, ctx: &egui::Context) {
        let Some(mode) = self.about_mode else {
            return;
        };

        let frame = egui::Frame::none()
            .fill(self.palette.surface)
            .stroke(Stroke::new(1.0, self.palette.border))
            .rounding(egui::Rounding::same(12.0))
            .inner_margin(Margin::symmetric(20.0, 16.0));

        let mut open = true;
        let is_manual = matches!(mode, AboutMode::Manual { .. });

        egui::Window::new("About Patina")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
            .default_width(ABOUT_WINDOW_WIDTH)
            .default_height(ABOUT_WINDOW_HEIGHT)
            .frame(frame)
            .open(&mut open)
            .show(ctx, |ui| {
                ui.set_min_size(egui::vec2(ABOUT_WINDOW_WIDTH, ABOUT_WINDOW_HEIGHT));
                ui.horizontal(|ui| {
                    if let Some(texture) = &self.logo_texture {
                        let dims = assets::logo_dimensions();
                        let mut size = egui::vec2(dims[0] as f32, dims[1] as f32);
                        if size.x > ABOUT_LOGO_MAX_WIDTH {
                            size *= ABOUT_LOGO_MAX_WIDTH / size.x;
                        }
                        ui.add(egui::widgets::Image::new((texture.id(), size)));
                    } else {
                        ui.allocate_space(egui::vec2(ABOUT_LOGO_MAX_WIDTH, ABOUT_LOGO_MAX_WIDTH));
                    }
                    ui.add_space(24.0);
                    ui.vertical(|ui| {
                        ui.heading("Patina Desktop");
                        ui.label(format!("Version {}", env!("CARGO_PKG_VERSION")));
                        ui.add_space(12.0);
                        ui.label(
                            "Patina is a native desktop chat client with OpenAI, Azure OpenAI, and MCP integrations.",
                        );
                        ui.add_space(12.0);
                        ui.label("License: MIT");
                        ui.label("© 2025 Iwan van der Kleijn");
                        if is_manual {
                            ui.add_space(16.0);
                            ui.label(
                                RichText::new("Press any key or click to dismiss")
                                    .italics()
                                    .small(),
                            );
                        }
                    });
                });
            });

        if !open {
            self.about_mode = None;
            return;
        }

        let should_close = match &mut self.about_mode {
            Some(AboutMode::Splash { opened }) => opened.elapsed() >= SPLASH_DURATION,
            Some(AboutMode::Manual { opened }) => {
                if opened.elapsed() < MANUAL_DISMISS_DELAY {
                    false
                } else {
                    ctx.input(|input| {
                        input.events.iter().any(|event| {
                            matches!(
                                event,
                                egui::Event::PointerButton { pressed: true, .. }
                                    | egui::Event::Key { pressed: true, .. }
                            )
                        })
                    })
                }
            }
            None => false,
        };

        if should_close {
            self.about_mode = None;
        }
    }

    fn show_settings_panel(&mut self, ctx: &egui::Context) {
        let response = self.settings_panel.show(ctx, &self.palette);
        if response.app_saved {
            if let Some(theme) = response.theme_changed {
                if self.menu_state.theme_mode != theme {
                    self.menu_state.theme_mode = theme;
                    if self.settings.data().theme_mode != theme {
                        self.settings.data_mut().theme_mode = theme;
                    }
                    self.apply_theme(ctx);
                }
            }
        }
        if response.project_saved {
            // Placeholder for future integration (e.g., reload drivers)
        }
    }
}

impl eframe::App for PatinaEguiApp {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        self.system_theme = frame.info().system_theme;
        render_ui(ctx, self);
        ctx.request_repaint_after(Duration::from_millis(16));
    }

    fn save(&mut self, _storage: &mut dyn eframe::Storage) {
        if let Err(err) = self.settings.persist() {
            error!(error = ?err, "Failed to save settings during shutdown");
        }
    }
}

pub fn render_ui(ctx: &egui::Context, app_state: &mut PatinaEguiApp) {
    app_state.render(ctx);
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiSettings {
    #[serde(default)]
    pub theme_mode: ThemeMode,
    #[serde(default = "UiSettings::default_sidebar_width")]
    pub sidebar_width: f32,
    #[serde(default = "UiSettings::default_sidebar_visible")]
    pub sidebar_visible: bool,
    #[serde(default = "UiSettings::default_window_size")]
    pub window_size: [f32; 2],
    #[serde(default)]
    pub pinned_chats: Vec<Uuid>,
    #[serde(default)]
    pub last_conversation: Option<Uuid>,
    #[serde(default = "UiSettings::default_model")]
    pub model: String,
    #[serde(default = "UiSettings::default_temperature")]
    pub temperature: f32,
    #[serde(default = "UiSettings::default_retain_input")]
    pub retain_input: bool,
    #[serde(default)]
    pub recent_projects: Vec<String>,
    #[serde(default)]
    pub current_project: Option<String>,
}

impl Default for UiSettings {
    fn default() -> Self {
        Self {
            theme_mode: ThemeMode::System,
            sidebar_width: Self::default_sidebar_width(),
            sidebar_visible: true,
            window_size: Self::default_window_size(),
            pinned_chats: Vec::new(),
            last_conversation: None,
            model: Self::default_model(),
            temperature: Self::default_temperature(),
            retain_input: true,
            recent_projects: Vec::new(),
            current_project: None,
        }
    }
}

impl UiSettings {
    fn default_sidebar_width() -> f32 {
        280.0
    }

    fn default_sidebar_visible() -> bool {
        true
    }

    fn default_window_size() -> [f32; 2] {
        [1280.0, 820.0]
    }

    fn default_model() -> String {
        "gpt-4o".to_string()
    }

    fn default_temperature() -> f32 {
        0.6
    }

    fn default_retain_input() -> bool {
        true
    }
}

pub struct UiSettingsStore {
    path: PathBuf,
    data: UiSettings,
    dirty: bool,
}

impl UiSettingsStore {
    pub fn load() -> Self {
        let path = Self::default_path();
        let data = Self::read_from_disk(&path).unwrap_or_default();
        Self {
            path,
            data,
            dirty: false,
        }
    }

    pub fn from_path(path: PathBuf) -> Self {
        let data = Self::read_from_disk(&path).unwrap_or_default();
        Self {
            path,
            data,
            dirty: false,
        }
    }

    pub fn temporary() -> Self {
        let mut path = std::env::temp_dir();
        path.push(format!("patina-ui-{}.json", Uuid::new_v4()));
        Self::from_path(path)
    }

    pub fn data(&self) -> &UiSettings {
        &self.data
    }

    pub fn data_mut(&mut self) -> &mut UiSettings {
        self.dirty = true;
        &mut self.data
    }

    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    pub fn persist(&mut self) -> Result<()> {
        if !self.dirty {
            return Ok(());
        }
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)?;
        }
        let serialized = serde_json::to_string_pretty(&self.data)?;
        fs::write(&self.path, serialized)?;
        self.dirty = false;
        Ok(())
    }

    fn read_from_disk(path: &Path) -> Option<UiSettings> {
        let contents = fs::read_to_string(path).ok()?;
        serde_json::from_str(&contents).ok()
    }

    fn default_path() -> PathBuf {
        ProjectDirs::from("com", "Patina", "Patina")
            .map(|dirs| dirs.config_dir().join("ui_settings.json"))
            .unwrap_or_else(|| PathBuf::from("ui_settings.json"))
    }
}

fn default_mcp_entries() -> Vec<McpSidebarEntry> {
    vec![
        McpSidebarEntry {
            id: "github".into(),
            name: "GitHub".into(),
            description: "Issues & Reviews".into(),
            status: McpStatus::Connected,
        },
        McpSidebarEntry {
            id: "playwright".into(),
            name: "Playwright".into(),
            description: "Browser automation".into(),
            status: McpStatus::Disconnected,
        },
        McpSidebarEntry {
            id: "notion".into(),
            name: "Notion".into(),
            description: "Docs search".into(),
            status: McpStatus::Connecting,
        },
    ]
}
