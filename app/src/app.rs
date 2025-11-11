const ABOUT_WINDOW_WIDTH: f32 = 640.0;
const ABOUT_WINDOW_HEIGHT: f32 = 360.0;
const ABOUT_LOGO_MAX_WIDTH: f32 = 240.0;

use crate::{
    assets,
    config::{self, ProviderConfig, Scope, UiSettings},
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
use std::collections::HashSet;
use std::env;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::runtime::Runtime;
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};
use tracing::{error, warn};
use uuid::Uuid;

const SPLASH_DURATION: Duration = Duration::from_secs(1);
const MANUAL_DISMISS_DELAY: Duration = Duration::from_millis(150);

#[derive(Clone, Copy)]
enum AboutMode {
    Splash { opened: Instant },
    Manual { opened: Instant },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ModelValidation {
    Ready,
    MissingModels,
    InvalidSelection,
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
    scope: Scope,
    ui_settings: UiSettings,
    provider_config: ProviderConfig,
    settings_panel: SettingsPanel,
    palette: ThemePalette,
    system_theme: Option<eframe::Theme>,
    error: Option<String>,
    mcp_entries: Vec<McpSidebarEntry>,
    pinned_lookup: HashSet<Uuid>,
    logo_texture: Option<egui::TextureHandle>,
    about_mode: Option<AboutMode>,
    pending_exit: bool,
    pending_title: Option<String>,
    current_workspace: Option<String>,
    pending_save: Option<tokio::task::JoinHandle<()>>,
    pending_provider_reload: Option<tokio::task::JoinHandle<Result<ProviderConfig>>>,
    validation_error: Option<String>,
}

impl PatinaEguiApp {
    pub fn new(
        project: Option<ProjectHandle>,
        driver: LlmDriver,
        runtime: Arc<Runtime>,
        scope: Scope,
        mut ui_settings: UiSettings,
        provider_config: ProviderConfig,
    ) -> Self {
        let settings_panel = SettingsPanel::new();
        let global_theme = settings_panel.app_settings().theme;
        if ui_settings.theme_mode != global_theme {
            ui_settings.theme_mode = global_theme;
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
                sidebar.collapsed = !ui_settings.sidebar_visible;
                sidebar
            },
            input_state: InputBarState::new(
                ui_settings.model.clone(),
                ui_settings.temperature,
                ui_settings.retain_input,
            ),
            chat_panel_state: ChatPanelState::default(),
            markdown_cache: CommonMarkCache::default(),
            scope,
            ui_settings,
            provider_config,
            settings_panel,
            palette: match global_theme {
                ThemeMode::Light => ThemePalette::for_light(),
                _ => ThemePalette::for_dark(),
            },
            system_theme: None,
            error: None,
            mcp_entries: default_mcp_entries(),
            pinned_lookup: HashSet::new(),
            logo_texture: None,
            about_mode: Some(AboutMode::Splash {
                opened: Instant::now(),
            }),
            pending_exit: false,
            pending_title: None,
            current_workspace: None,
            pending_save: None,
            pending_provider_reload: None,
            validation_error: None,
        };
        app.refresh_pinned_cache();
        if let Some(project) = project {
            app.activate_project(project);
        } else {
            {
                app.ui_settings.current_project = None;
                app.ui_settings.last_conversation = None;
            }
            app.settings_panel.set_project(None);
            app.pending_title = Some("Patina".to_string());
            app.current_workspace = None;
            app.spawn_save();
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

    fn poll_provider_config_reload(&mut self) {
        if let Some(handle) = self.pending_provider_reload.take() {
            if handle.is_finished() {
                match self.runtime.block_on(handle) {
                    Ok(Ok(config)) => {
                        self.provider_config = config;
                        self.error = None;
                        self.validation_error = None;
                    }
                    Ok(Err(err)) => {
                        error!(error = ?err, "Failed to reload provider config");
                        self.error = Some(format!("Failed to reload provider config: {err}"));
                    }
                    Err(err) => {
                        error!(error = ?err, "Provider config task failed");
                        self.error = Some(format!("Provider config task failed: {err}"));
                    }
                }
            } else {
                self.pending_provider_reload = Some(handle);
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
                        RichText::new("Update patina.yaml to configure AI access.")
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
                let pinned_order = self.ui_settings.pinned_chats.clone();

                let response = egui::SidePanel::left("sidebar")
                    .resizable(true)
                    .min_width(220.0)
                    .default_width(self.ui_settings.sidebar_width)
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
                if (self.ui_settings.sidebar_width - width).abs() > 1.0 {
                    self.ui_settings.sidebar_width = width;
                    self.spawn_save();
                }
            }

            egui::TopBottomPanel::bottom("chat_input")
                .frame(
                    egui::Frame::none()
                        .fill(self.palette.surface)
                        .inner_margin(Margin::same(12.0)),
                )
                .show(ctx, |ui| {
                    let model_valid = matches!(self.model_validation(), ModelValidation::Ready);
                    let input_output = InputBar::show(
                        ui,
                        &mut self.input_state,
                        &self.palette,
                        &self.provider_config.available_models,
                        model_valid,
                    );
                    self.handle_input_output(input_output);
                    self.input_state.selected_model = self.ui_settings.model.clone();
                    self.input_state.temperature = self.ui_settings.temperature;
                    self.input_state.retain_input = self.ui_settings.retain_input;
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
            self.ui_settings.theme_mode = mode;
            self.spawn_save();
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
                        self.ui_settings.last_conversation = None;
                        self.spawn_save();
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
            self.ui_settings.model = model;
            self.spawn_save();
        }
        if let Some(temp) = output.temperature_changed {
            self.ui_settings.temperature = temp;
            self.spawn_save();
        }
        if self.ui_settings.retain_input != self.input_state.retain_input {
            self.ui_settings.retain_input = self.input_state.retain_input;
            self.spawn_save();
        }
    }

    fn submit_message(&mut self) {
        let content = self.input_state.draft.trim();
        if content.is_empty() {
            return;
        }
        match self.model_validation() {
            ModelValidation::Ready => {}
            ModelValidation::MissingModels => {
                self.validation_error = Some(
                    "No models are configured. Edit Settings to add models in patina.yaml.".into(),
                );
                return;
            }
            ModelValidation::InvalidSelection => {
                self.validation_error = Some(
                    "Selected model is not available. Pick a model from the list in patina.yaml."
                        .into(),
                );
                return;
            }
        }
        let Some(state) = self.state.as_ref().cloned() else {
            return;
        };
        let payload = content.to_owned();
        let model = self.ui_settings.model.clone();
        let temperature = self.ui_settings.temperature;
        let tx = self.tx.clone();
        self.runtime.spawn(async move {
            let result = state.send_user_message(payload, model, temperature).await;
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
        if self.ui_settings.sidebar_visible != visible {
            self.ui_settings.sidebar_visible = visible;
            self.spawn_save();
        }
    }

    fn update_last_conversation(&mut self, id: Uuid) {
        self.ui_settings.last_conversation = Some(id);
        self.spawn_save();
    }

    fn pin_chat(&mut self, id: Uuid) {
        if !self.ui_settings.pinned_chats.contains(&id) {
            let list = &mut self.ui_settings.pinned_chats;
            list.insert(0, id);
            self.refresh_pinned_cache();
            self.spawn_save();
        }
    }

    fn unpin_chat(&mut self, id: Uuid) {
        if self.ui_settings.pinned_chats.contains(&id) {
            let list = &mut self.ui_settings.pinned_chats;
            list.retain(|candidate| candidate != &id);
            self.refresh_pinned_cache();
            self.spawn_save();
        }
    }

    fn refresh_pinned_cache(&mut self) {
        self.pinned_lookup = self.ui_settings.pinned_chats.iter().copied().collect();
    }

    fn model_validation(&self) -> ModelValidation {
        if self.provider_config.available_models.is_empty() {
            return ModelValidation::MissingModels;
        }
        let selection = self.ui_settings.model.trim();
        if selection.is_empty() {
            return ModelValidation::InvalidSelection;
        }
        if self
            .provider_config
            .available_models
            .iter()
            .any(|model| model == selection)
        {
            ModelValidation::Ready
        } else {
            ModelValidation::InvalidSelection
        }
    }

    fn spawn_save(&mut self) {
        let scope = self.scope.clone();
        let settings = self.ui_settings.clone();
        if let Some(handle) = self.pending_save.take() {
            handle.abort();
        }
        let runtime = self.runtime.clone();
        self.pending_save = Some(runtime.spawn(async move {
            if let Err(err) = config::save_ui_settings(&scope, &settings).await {
                error!(error = ?err, "Failed to save UI settings");
            }
        }));
    }

    fn persist_now(&mut self) {
        if let Some(handle) = self.pending_save.take() {
            handle.abort();
        }
        let scope = self.scope.clone();
        let settings = self.ui_settings.clone();
        if let Err(err) = self
            .runtime
            .block_on(config::save_ui_settings(&scope, &settings))
        {
            error!(error = ?err, "Failed to save UI settings");
        }
    }

    fn reload_provider_config(&mut self) {
        let scope = self.scope.clone();
        if let Some(handle) = self.pending_provider_reload.take() {
            handle.abort();
        }
        let runtime = self.runtime.clone();
        self.pending_provider_reload =
            Some(runtime.spawn(async move { config::load_provider_config(&scope).await }));
    }

    fn activate_project(&mut self, project: ProjectHandle) {
        self.settings_panel.set_project(Some(&project));
        let last_selected = self.ui_settings.last_conversation;
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
        self.ui_settings.current_project = Some(root.clone());
        self.ui_settings
            .recent_projects
            .retain(|entry| entry != &root);
        self.ui_settings.recent_projects.insert(0, root);
        if self.ui_settings.recent_projects.len() > 10 {
            self.ui_settings.recent_projects.truncate(10);
        }
        self.spawn_save();
    }

    fn sync_last_conversation(&mut self) {
        let active = self
            .state
            .as_ref()
            .and_then(|state| state.active_conversation().map(|c| c.id));
        self.ui_settings.last_conversation = active;
        self.spawn_save();
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
            let stored = self.ui_settings.window_size;
            if (stored[0] - size.x).abs() > 1.0 || (stored[1] - size.y).abs() > 1.0 {
                self.ui_settings.window_size = [size.x, size.y];
                self.spawn_save();
            }
        }
    }

    fn render(&mut self, ctx: &egui::Context) {
        self.apply_theme(ctx);
        self.process_background_results();
        self.poll_provider_config_reload();
        if !matches!(self.about_mode, Some(AboutMode::Manual { .. })) {
            self.handle_shortcuts(ctx);
        }
        self.ensure_logo_texture(ctx);
        self.layout(ctx);
        self.show_settings_panel(ctx);
        self.draw_about_dialog(ctx);
        self.show_validation_modal(ctx);
        self.capture_window_size(ctx);
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

    fn show_validation_modal(&mut self, ctx: &egui::Context) {
        let Some(message) = self.validation_error.clone() else {
            return;
        };
        let mut open = true;
        egui::Window::new("Unable to send")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
            .show(ctx, |ui| {
                ui.add(egui::Label::new(RichText::new(message.clone())).wrap(true));
                ui.add_space(12.0);
                if ui.button("OK").clicked() {
                    open = false;
                }
            });
        if !open {
            self.validation_error = None;
        }
    }

    fn show_settings_panel(&mut self, ctx: &egui::Context) {
        let response = self.settings_panel.show(ctx, &self.palette);
        if response.app_saved {
            self.reload_provider_config();
            if let Some(theme) = response.theme_changed {
                if self.menu_state.theme_mode != theme {
                    self.menu_state.theme_mode = theme;
                    if self.ui_settings.theme_mode != theme {
                        self.ui_settings.theme_mode = theme;
                        self.spawn_save();
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
        self.persist_now();
    }
}

pub fn render_ui(ctx: &egui::Context, app_state: &mut PatinaEguiApp) {
    app_state.render(ctx);
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
