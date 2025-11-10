use crate::ui::{ThemeMode, ThemePalette};
use anyhow::{Context, Result};
use directories::BaseDirs;
use egui::{
    self, Align, Color32, Frame, Grid, Id, Label, Layout, Margin, RichText, ScrollArea, Stroke,
    Vec2,
};
use patina_core::llm::LlmProviderKind;
use patina_core::project::ProjectHandle;
use serde::{Deserialize, Serialize};
use serde_yaml::{Mapping, Value};
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};
use tracing::warn;
use url::Url;

const DEFAULT_MODEL_NAMES: [&str; 3] = ["gpt-5", "gpt-5-mini", "gpt-5 nano"];

fn default_model_names() -> Vec<String> {
    DEFAULT_MODEL_NAMES
        .iter()
        .map(|name| name.to_string())
        .collect()
}

fn normalized_models(list: Vec<String>) -> Vec<String> {
    let mut seen = HashSet::new();
    let mut output = Vec::new();
    for entry in list {
        let trimmed = entry.trim();
        if trimmed.is_empty() {
            continue;
        }
        if seen.insert(trimmed.to_ascii_lowercase()) {
            output.push(trimmed.to_string());
        }
    }
    if output.is_empty() {
        default_model_names()
    } else {
        output
    }
}

fn parse_models_input(input: &str) -> Vec<String> {
    let mut items = Vec::new();
    let mut seen = HashSet::new();
    for segment in input.split(|c| c == ',' || c == ';') {
        let trimmed = segment.trim();
        if trimmed.is_empty() {
            continue;
        }
        if seen.insert(trimmed.to_ascii_lowercase()) {
            items.push(trimmed.to_string());
        }
    }
    if items.is_empty() {
        default_model_names()
    } else {
        items
    }
}

fn models_to_input(models: &[String]) -> String {
    if models.is_empty() {
        String::new()
    } else {
        models.join(", ")
    }
}

fn ensure_mapping(value: &mut Value) -> &mut Mapping {
    if !matches!(value, Value::Mapping(_)) {
        *value = Value::Mapping(Mapping::new());
    }
    match value {
        Value::Mapping(map) => map,
        _ => unreachable!(),
    }
}

fn default_provider() -> LlmProviderKind {
    LlmProviderKind::OpenAi
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderSelection {
    pub provider: LlmProviderKind,
    pub openai: OpenAiSettingsData,
    pub azure: AzureSettingsData,
}

impl Default for ProviderSelection {
    fn default() -> Self {
        Self {
            provider: default_provider(),
            openai: OpenAiSettingsData::default(),
            azure: AzureSettingsData::default(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpenAiSettingsData {
    pub api_key: String,
    pub available_models: Vec<String>,
}

impl Default for OpenAiSettingsData {
    fn default() -> Self {
        Self {
            api_key: String::new(),
            available_models: default_model_names(),
        }
    }
}

impl OpenAiSettingsData {
    fn from_file(file: FileOpenAiSettings) -> Self {
        Self {
            api_key: file.api_key,
            available_models: normalized_models(file.available_models),
        }
    }

    fn to_file(&self) -> FileOpenAiSettings {
        FileOpenAiSettings {
            api_key: self.api_key.clone(),
            available_models: if self.available_models.is_empty() {
                default_model_names()
            } else {
                self.available_models.clone()
            },
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AzureSettingsData {
    pub api_key: String,
    pub endpoint: String,
    pub api_version: String,
    pub deployment_name: String,
    pub available_models: Vec<String>,
}

impl Default for AzureSettingsData {
    fn default() -> Self {
        Self {
            api_key: String::new(),
            endpoint: String::new(),
            api_version: String::new(),
            deployment_name: String::new(),
            available_models: default_model_names(),
        }
    }
}

impl AzureSettingsData {
    fn from_file(file: FileAzureSettings) -> Self {
        Self {
            api_key: file.api_key,
            endpoint: file.endpoint,
            api_version: file.api_version,
            deployment_name: file.deployment_name,
            available_models: normalized_models(file.available_models),
        }
    }

    fn to_file(&self) -> FileAzureSettings {
        FileAzureSettings {
            api_key: self.api_key.clone(),
            endpoint: self.endpoint.clone(),
            api_version: self.api_version.clone(),
            deployment_name: self.deployment_name.clone(),
            available_models: if self.available_models.is_empty() {
                default_model_names()
            } else {
                self.available_models.clone()
            },
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppSettingsData {
    pub theme: ThemeMode,
    pub provider: ProviderSelection,
}

impl Default for AppSettingsData {
    fn default() -> Self {
        Self {
            theme: ThemeMode::System,
            provider: ProviderSelection::default(),
        }
    }
}

impl AppSettingsData {
    fn from_file(file: AppSettingsFile) -> Self {
        let provider = ProviderSelection {
            provider: file.provider,
            openai: OpenAiSettingsData::from_file(file.openai),
            azure: AzureSettingsData::from_file(file.azure),
        };
        Self {
            theme: file.theme,
            provider,
        }
    }

    fn to_file(&self) -> AppSettingsFile {
        AppSettingsFile {
            theme: self.theme,
            provider: self.provider.provider,
            openai: self.provider.openai.to_file(),
            azure: self.provider.azure.to_file(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectSettingsData {
    pub inherit_app: bool,
    pub provider: ProviderSelection,
}

impl Default for ProjectSettingsData {
    fn default() -> Self {
        Self {
            inherit_app: true,
            provider: ProviderSelection::default(),
        }
    }
}

impl ProjectSettingsData {
    fn from_file(file: ProjectSettingsFile) -> Self {
        let provider_kind = file.provider.unwrap_or_else(default_provider);
        let openai = file.openai.unwrap_or_default();
        let azure = file.azure.unwrap_or_default();
        Self {
            inherit_app: file.inherit_app,
            provider: ProviderSelection {
                provider: provider_kind,
                openai: OpenAiSettingsData::from_file(openai),
                azure: AzureSettingsData::from_file(azure),
            },
        }
    }

    fn to_file(&self) -> ProjectSettingsFile {
        if self.inherit_app {
            ProjectSettingsFile {
                inherit_app: true,
                provider: None,
                openai: None,
                azure: None,
            }
        } else {
            ProjectSettingsFile {
                inherit_app: false,
                provider: Some(self.provider.provider),
                openai: Some(self.provider.openai.to_file()),
                azure: Some(self.provider.azure.to_file()),
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct FileOpenAiSettings {
    #[serde(default)]
    api_key: String,
    #[serde(default = "default_model_names")]
    available_models: Vec<String>,
}

impl Default for FileOpenAiSettings {
    fn default() -> Self {
        Self {
            api_key: String::new(),
            available_models: default_model_names(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct FileAzureSettings {
    #[serde(default)]
    api_key: String,
    #[serde(default)]
    endpoint: String,
    #[serde(default)]
    api_version: String,
    #[serde(default)]
    deployment_name: String,
    #[serde(default = "default_model_names")]
    available_models: Vec<String>,
}

impl Default for FileAzureSettings {
    fn default() -> Self {
        Self {
            api_key: String::new(),
            endpoint: String::new(),
            api_version: String::new(),
            deployment_name: String::new(),
            available_models: default_model_names(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct AppSettingsFile {
    #[serde(default)]
    theme: ThemeMode,
    #[serde(default = "default_provider")]
    provider: LlmProviderKind,
    #[serde(default)]
    openai: FileOpenAiSettings,
    #[serde(default, rename = "azure_openai")]
    azure: FileAzureSettings,
}

impl Default for AppSettingsFile {
    fn default() -> Self {
        Self {
            theme: ThemeMode::System,
            provider: default_provider(),
            openai: FileOpenAiSettings::default(),
            azure: FileAzureSettings::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ProjectSettingsFile {
    #[serde(default)]
    inherit_app: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    provider: Option<LlmProviderKind>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    openai: Option<FileOpenAiSettings>,
    #[serde(
        default,
        rename = "azure_openai",
        skip_serializing_if = "Option::is_none"
    )]
    azure: Option<FileAzureSettings>,
}

impl Default for ProjectSettingsFile {
    fn default() -> Self {
        Self {
            inherit_app: true,
            provider: None,
            openai: None,
            azure: None,
        }
    }
}

pub struct GlobalSettingsStore {
    path: PathBuf,
    document: Value,
    data: AppSettingsData,
    dirty: bool,
}

impl GlobalSettingsStore {
    pub fn load() -> Self {
        let path = global_config_path();
        let document = load_document(&path);
        let data = extract_app_settings(&document);
        Self {
            path,
            document,
            data,
            dirty: false,
        }
    }

    pub fn data(&self) -> &AppSettingsData {
        &self.data
    }

    pub fn set(&mut self, data: AppSettingsData) {
        self.data = data;
        self.dirty = true;
    }

    pub fn persist(&mut self) -> Result<()> {
        if !self.dirty {
            return Ok(());
        }
        let mut value = self.document.clone();
        let mapping = ensure_mapping(&mut value);
        let serialized = serde_yaml::to_value(self.data.to_file())?;
        mapping.insert(Value::String("app".to_string()), serialized);
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent).with_context(|| {
                format!("failed to create config directory at {}", parent.display())
            })?;
        }
        let contents = serde_yaml::to_string(&value)?;
        fs::write(&self.path, contents)
            .with_context(|| format!("failed to write settings to {}", self.path.display()))?;
        self.document = value;
        self.dirty = false;
        Ok(())
    }
}

pub struct ProjectSettingsStore {
    path: PathBuf,
    document: Value,
    data: ProjectSettingsData,
    dirty: bool,
}

impl ProjectSettingsStore {
    pub fn load(path: PathBuf) -> Self {
        let document = load_document(&path);
        let data = extract_project_settings(&document);
        Self {
            path,
            document,
            data,
            dirty: false,
        }
    }

    pub fn data(&self) -> &ProjectSettingsData {
        &self.data
    }

    pub fn set(&mut self, data: ProjectSettingsData) {
        self.data = data;
        self.dirty = true;
    }

    pub fn persist(&mut self) -> Result<()> {
        if !self.dirty {
            return Ok(());
        }
        let mut value = self.document.clone();
        let mapping = ensure_mapping(&mut value);
        let serialized = serde_yaml::to_value(self.data.to_file())?;
        mapping.insert(Value::String("settings".to_string()), serialized);
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent).with_context(|| {
                format!(
                    "failed to create project settings directory at {}",
                    parent.display()
                )
            })?;
        }
        let contents = serde_yaml::to_string(&value)?;
        fs::write(&self.path, contents).with_context(|| {
            format!(
                "failed to write project settings to {}",
                self.path.display()
            )
        })?;
        self.document = value;
        self.dirty = false;
        Ok(())
    }
}

fn load_document(path: &Path) -> Value {
    match fs::read_to_string(path) {
        Ok(contents) => match serde_yaml::from_str::<Value>(&contents) {
            Ok(value) => value,
            Err(err) => {
                warn!("Failed to parse {}: {err}", path.display());
                Value::Mapping(Mapping::new())
            }
        },
        Err(err) => {
            if path.exists() {
                warn!("Failed to read {}: {err}", path.display());
            }
            Value::Mapping(Mapping::new())
        }
    }
}

fn extract_app_settings(document: &Value) -> AppSettingsData {
    let section = document
        .get("app")
        .cloned()
        .unwrap_or(Value::Mapping(Mapping::new()));
    let file: AppSettingsFile = serde_yaml::from_value(section).unwrap_or_default();
    AppSettingsData::from_file(file)
}

fn extract_project_settings(document: &Value) -> ProjectSettingsData {
    let section = document
        .get("settings")
        .cloned()
        .unwrap_or(Value::Mapping(Mapping::new()));
    let file: ProjectSettingsFile = serde_yaml::from_value(section).unwrap_or_default();
    ProjectSettingsData::from_file(file)
}

fn global_config_path() -> PathBuf {
    #[cfg(target_os = "linux")]
    {
        if let Some(base) = BaseDirs::new() {
            return base.config_dir().join("patina").join("patina.yml");
        }
    }
    #[cfg(target_os = "macos")]
    {
        if let Some(base) = BaseDirs::new() {
            return base
                .home_dir()
                .join("Library")
                .join("Application Support")
                .join("Patina")
                .join("patina.yml");
        }
    }
    #[cfg(target_os = "windows")]
    {
        if let Some(base) = BaseDirs::new() {
            return base.config_dir().join("Patina").join("patina.yml");
        }
    }
    PathBuf::from("patina.yml")
}

struct Feedback {
    message: String,
    success: bool,
    created: Instant,
}

impl Feedback {
    fn success(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            success: true,
            created: Instant::now(),
        }
    }

    fn failure(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            success: false,
            created: Instant::now(),
        }
    }

    fn is_fresh(&self) -> bool {
        self.created.elapsed() < Duration::from_secs(4)
    }
}

pub struct SettingsResponse {
    pub app_saved: bool,
    pub project_saved: bool,
    pub theme_changed: Option<ThemeMode>,
}

impl Default for SettingsResponse {
    fn default() -> Self {
        Self {
            app_saved: false,
            project_saved: false,
            theme_changed: None,
        }
    }
}

pub struct SettingsPanel {
    global: GlobalSettingsStore,
    project: Option<ProjectSettingsStore>,
    project_name: Option<String>,
    state: ModalState,
}

impl SettingsPanel {
    pub fn new() -> Self {
        let global = GlobalSettingsStore::load();
        let app_form = AppFormState::from_data(global.data().clone());
        Self {
            global,
            project: None,
            project_name: None,
            state: ModalState {
                open: false,
                app: app_form,
                project: None,
            },
        }
    }

    pub fn app_settings(&self) -> &AppSettingsData {
        self.global.data()
    }

    pub fn open(&mut self) {
        self.state.app.reset(self.global.data().clone());
        if let Some(project_store) = self.project.as_ref() {
            let form = ProjectFormState::from_data(project_store.data().clone());
            self.state.project = Some(form);
        } else {
            self.state.project = None;
        }
        self.state.open = true;
    }

    pub fn close(&mut self) {
        self.state.open = false;
    }

    pub fn is_open(&self) -> bool {
        self.state.open
    }

    pub fn set_project(&mut self, project: Option<&ProjectHandle>) {
        if let Some(handle) = project {
            let path = handle.metadata_path().to_path_buf();
            let store = ProjectSettingsStore::load(path);
            self.project = Some(store);
            self.project_name = Some(handle.name().to_string());
            if self.state.open {
                self.state.project = Some(ProjectFormState::from_data(
                    self.project.as_ref().unwrap().data().clone(),
                ));
            }
        } else {
            self.project = None;
            self.project_name = None;
            self.state.project = None;
        }
    }

    pub fn show(&mut self, ctx: &egui::Context, palette: &ThemePalette) -> SettingsResponse {
        let mut result = SettingsResponse::default();
        if !self.state.open {
            return result;
        }

        let mut open = self.state.open;
        egui::Window::new("Settings")
            .id(Id::new("settings_modal"))
            .collapsible(false)
            .resizable(true)
            .default_width(720.0)
            .open(&mut open)
            .show(ctx, |ui| {
                ui.set_min_height(520.0);
                ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        let app_section = self.render_app_settings(ui, palette);
                        if app_section.saved {
                            result.app_saved = true;
                        }
                        if app_section.theme.is_some() {
                            result.theme_changed = app_section.theme;
                        }
                        ui.add_space(24.0);
                        let project_section = self.render_project_settings(ui, palette);
                        if project_section.saved {
                            result.project_saved = true;
                        }
                    });
            });
        if !open {
            self.state.open = false;
        }

        result
    }

    fn render_app_settings(
        &mut self,
        ui: &mut egui::Ui,
        palette: &ThemePalette,
    ) -> AppSectionResult {
        let mut outcome = AppSectionResult::unsaved();
        let mut save_request: Option<AppSettingsData> = None;
        let mut cancel_requested = false;
        let mut validation = ProviderValidation::default();
        let frame = Frame::none()
            .fill(palette.surface)
            .stroke(Stroke::new(1.0, palette.border))
            .rounding(egui::Rounding::from(8.0))
            .inner_margin(Margin::symmetric(20.0, 16.0));
        frame.show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.heading("App settings");
                if let Some(feedback) = self.state.app.feedback.as_ref() {
                    if feedback.is_fresh() {
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            let color = if feedback.success {
                                Color32::from_rgb(46, 125, 50)
                            } else {
                                palette.warning
                            };
                            ui.colored_label(color, &feedback.message);
                        });
                    }
                }
            });
            ui.add_space(12.0);
            let mut dirty = false;
            Grid::new("app_settings_grid")
                .num_columns(2)
                .spacing(Vec2::new(24.0, 12.0))
                .striped(false)
                .show(ui, |ui| {
                    ui.label(RichText::new("Theme").strong());
                    let previous_theme = self.state.app.editor.theme;
                    egui::ComboBox::from_id_source("theme_mode")
                        .selected_text(self.state.app.editor.theme.label())
                        .show_ui(ui, |ui| {
                            for mode in ThemeMode::ALL {
                                if ui
                                    .selectable_value(
                                        &mut self.state.app.editor.theme,
                                        mode,
                                        mode.label(),
                                    )
                                    .changed()
                                {}
                            }
                        });
                    if self.state.app.editor.theme != previous_theme {
                        dirty = true;
                    }
                    ui.end_row();

                    ui.label(RichText::new("LLM provider").strong());
                    let previous_provider = self.state.app.editor.provider.provider;
                    let mut selection = previous_provider;
                    egui::ComboBox::from_id_source("app_provider")
                        .selected_text(provider_before_label(previous_provider))
                        .show_ui(ui, |ui| {
                            for candidate in [LlmProviderKind::OpenAi, LlmProviderKind::AzureOpenAi]
                            {
                                let label = provider_before_label(candidate);
                                if ui
                                    .selectable_value(&mut selection, candidate, label)
                                    .changed()
                                {
                                    dirty = true;
                                }
                            }
                        });
                    if selection != previous_provider {
                        self.state.app.editor.provider.provider = selection;
                    }
                    ui.end_row();
                });

            ui.add_space(16.0);
            let active_provider = self.state.app.editor.provider.provider;
            validation = render_provider_panel(
                ui,
                palette,
                &mut self.state.app.editor.provider,
                active_provider,
                false,
                &mut dirty,
            );

            ui.add_space(20.0);
            let data = self.state.app.current_data();
            let is_dirty = dirty || data != self.state.app.original;
            let can_save = validation.is_valid();
            ui.horizontal(|ui| {
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    let save_enabled = is_dirty && can_save;
                    if ui
                        .add_enabled(save_enabled, egui::Button::new("Save"))
                        .clicked()
                    {
                        save_request = Some(data.clone());
                    }
                    if ui.button("Cancel").clicked() {
                        cancel_requested = true;
                    }
                });
            });
        });
        if cancel_requested {
            self.state.app.reset(self.global.data().clone());
        }
        if let Some(data) = save_request {
            match self.save_app_settings(data.clone()) {
                Ok(_) => {
                    self.state.app.original = data.clone();
                    self.state.app.feedback = Some(Feedback::success("App settings saved"));
                    outcome.saved = true;
                    outcome.theme = Some(data.theme);
                }
                Err(err) => {
                    self.state.app.feedback = Some(Feedback::failure(err.to_string()));
                }
            }
        }
        outcome
    }

    fn render_project_settings(
        &mut self,
        ui: &mut egui::Ui,
        palette: &ThemePalette,
    ) -> ProjectSectionResult {
        let mut outcome = ProjectSectionResult::unsaved();
        let mut save_request: Option<ProjectSettingsData> = None;
        let mut cancel_requested = false;
        let mut validation = ProviderValidation::default();
        let frame = Frame::none()
            .fill(palette.surface)
            .stroke(Stroke::new(1.0, palette.border))
            .rounding(egui::Rounding::from(8.0))
            .inner_margin(Margin::symmetric(20.0, 16.0));
        frame.show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.heading("Project settings");
                if let Some(name) = self.project_name.as_ref() {
                    ui.label(RichText::new(format!("— {}", name)).color(palette.text_secondary));
                }
                if let Some(form) = self.state.project.as_ref() {
                    if let Some(feedback) = form.feedback.as_ref() {
                        if feedback.is_fresh() {
                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                let color = if feedback.success {
                                    Color32::from_rgb(46, 125, 50)
                                } else {
                                    palette.warning
                                };
                                ui.colored_label(color, &feedback.message);
                            });
                        }
                    }
                }
            });
            ui.add_space(12.0);

            if self.project.is_none() {
                ui.label(
                    RichText::new("Open a project to configure project-level overrides.")
                        .color(palette.text_secondary),
                );
                return;
            }

            let form = self.state.project.get_or_insert_with(|| {
                ProjectFormState::from_data(self.project.as_ref().unwrap().data().clone())
            });

            let mut dirty = false;

            let mut inherit_changed = false;
            ui.horizontal(|ui| {
                let mut inherit = form.editor.inherit_app;
                if ui
                    .checkbox(&mut inherit, "Inherit system setting")
                    .changed()
                {
                    form.editor.inherit_app = inherit;
                    dirty = true;
                    inherit_changed = true;
                }
                if form.editor.inherit_app {
                    ui.label(RichText::new("Using App settings").color(palette.text_secondary));
                }
            });

            ui.add_space(8.0);

            Grid::new("project_settings_grid")
                .num_columns(2)
                .spacing(Vec2::new(24.0, 12.0))
                .show(ui, |ui| {
                    ui.label(RichText::new("LLM provider").strong());
                    let previous_provider = form.editor.provider.provider;
                    let mut selection = previous_provider;
                    egui::ComboBox::from_id_source("project_provider")
                        .selected_text(provider_before_label(previous_provider))
                        .show_ui(ui, |ui| {
                            for candidate in [LlmProviderKind::OpenAi, LlmProviderKind::AzureOpenAi]
                            {
                                ui.selectable_value(
                                    &mut selection,
                                    candidate,
                                    provider_before_label(candidate),
                                );
                            }
                        });
                    if selection != previous_provider {
                        form.editor.provider.provider = selection;
                        dirty = true;
                    }
                    ui.end_row();
                });

            ui.add_space(16.0);

            let active_provider = form.editor.provider.provider;
            validation = render_provider_panel(
                ui,
                palette,
                &mut form.editor.provider,
                active_provider,
                form.editor.inherit_app,
                &mut dirty,
            );

            ui.add_space(20.0);
            let data = form.current_data();
            let is_dirty = dirty || inherit_changed || Some(&data) != form.original.as_ref();
            let can_save = form.editor.inherit_app || validation.is_valid();

            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                let save_enabled = is_dirty && can_save;
                if ui
                    .add_enabled(save_enabled, egui::Button::new("Save"))
                    .clicked()
                {
                    save_request = Some(data.clone());
                }
                if ui.button("Cancel").clicked() {
                    cancel_requested = true;
                }
            });
        });
        if cancel_requested {
            if let (Some(project_store), Some(form)) =
                (self.project.as_ref(), self.state.project.as_mut())
            {
                form.reset(project_store.data().clone());
            }
        }
        if let Some(data) = save_request {
            match self.save_project_settings(data.clone()) {
                Ok(_) => {
                    if let Some(form) = self.state.project.as_mut() {
                        form.original = Some(data.clone());
                        form.feedback = Some(Feedback::success("Project settings saved"));
                    }
                    outcome.saved = true;
                }
                Err(err) => {
                    if let Some(form) = self.state.project.as_mut() {
                        form.feedback = Some(Feedback::failure(err.to_string()));
                    }
                }
            }
        }
        outcome
    }

    pub fn apply_theme_selection(&mut self, theme: ThemeMode) -> Result<()> {
        let mut data = self.global.data().clone();
        if data.theme != theme {
            data.theme = theme;
            self.global.set(data);
            self.global.persist()?;
            self.state.app.reset(self.global.data().clone());
        }
        Ok(())
    }

    fn save_app_settings(&mut self, data: AppSettingsData) -> Result<()> {
        self.global.set(data);
        self.global.persist()
    }

    fn save_project_settings(&mut self, data: ProjectSettingsData) -> Result<()> {
        if let Some(store) = self.project.as_mut() {
            store.set(data.clone());
            store.persist()?;
        }
        Ok(())
    }
}

struct ModalState {
    open: bool,
    app: AppFormState,
    project: Option<ProjectFormState>,
}

struct AppFormState {
    original: AppSettingsData,
    editor: AppFormEditor,
    feedback: Option<Feedback>,
}

impl AppFormState {
    fn from_data(data: AppSettingsData) -> Self {
        Self {
            original: data.clone(),
            editor: AppFormEditor::from_data(&data),
            feedback: None,
        }
    }

    fn reset(&mut self, data: AppSettingsData) {
        self.original = data.clone();
        self.editor = AppFormEditor::from_data(&data);
        self.feedback = None;
    }

    fn current_data(&self) -> AppSettingsData {
        self.editor.to_data()
    }
}

struct ProjectFormState {
    original: Option<ProjectSettingsData>,
    editor: ProjectFormEditor,
    feedback: Option<Feedback>,
}

impl ProjectFormState {
    fn from_data(data: ProjectSettingsData) -> Self {
        Self {
            original: Some(data.clone()),
            editor: ProjectFormEditor::from_data(&data),
            feedback: None,
        }
    }

    fn current_data(&self) -> ProjectSettingsData {
        self.editor.to_data()
    }

    fn reset(&mut self, data: ProjectSettingsData) {
        self.original = Some(data.clone());
        self.editor = ProjectFormEditor::from_data(&data);
        self.feedback = None;
    }
}

struct AppFormEditor {
    theme: ThemeMode,
    provider: ProviderEditor,
}

impl AppFormEditor {
    fn from_data(data: &AppSettingsData) -> Self {
        Self {
            theme: data.theme,
            provider: ProviderEditor::from_selection(&data.provider),
        }
    }

    fn to_data(&self) -> AppSettingsData {
        AppSettingsData {
            theme: self.theme,
            provider: self.provider.to_selection(),
        }
    }
}

struct ProjectFormEditor {
    inherit_app: bool,
    provider: ProviderEditor,
}

impl ProjectFormEditor {
    fn from_data(data: &ProjectSettingsData) -> Self {
        Self {
            inherit_app: data.inherit_app,
            provider: ProviderEditor::from_selection(&data.provider),
        }
    }

    fn to_data(&self) -> ProjectSettingsData {
        ProjectSettingsData {
            inherit_app: self.inherit_app,
            provider: self.provider.to_selection(),
        }
    }
}

struct ProviderEditor {
    provider: LlmProviderKind,
    openai: OpenAiEditor,
    azure: AzureEditor,
    details_expanded: bool,
}

impl ProviderEditor {
    fn from_selection(selection: &ProviderSelection) -> Self {
        Self {
            provider: selection.provider,
            openai: OpenAiEditor::from_data(&selection.openai),
            azure: AzureEditor::from_data(&selection.azure),
            details_expanded: true,
        }
    }

    fn to_selection(&self) -> ProviderSelection {
        ProviderSelection {
            provider: self.provider,
            openai: self.openai.to_data(),
            azure: self.azure.to_data(),
        }
    }
}

struct OpenAiEditor {
    api_key: String,
    reveal: bool,
    models_input: String,
}

impl OpenAiEditor {
    fn from_data(data: &OpenAiSettingsData) -> Self {
        Self {
            api_key: data.api_key.clone(),
            reveal: false,
            models_input: models_to_input(&data.available_models),
        }
    }

    fn to_data(&self) -> OpenAiSettingsData {
        OpenAiSettingsData {
            api_key: self.api_key.trim().to_string(),
            available_models: parse_models_input(&self.models_input),
        }
    }
}

struct AzureEditor {
    api_key: String,
    reveal: bool,
    endpoint: String,
    api_version: String,
    deployment_name: String,
    models_input: String,
}

impl AzureEditor {
    fn from_data(data: &AzureSettingsData) -> Self {
        Self {
            api_key: data.api_key.clone(),
            reveal: false,
            endpoint: data.endpoint.clone(),
            api_version: data.api_version.clone(),
            deployment_name: data.deployment_name.clone(),
            models_input: models_to_input(&data.available_models),
        }
    }

    fn to_data(&self) -> AzureSettingsData {
        AzureSettingsData {
            api_key: self.api_key.trim().to_string(),
            endpoint: self.endpoint.trim().to_string(),
            api_version: self.api_version.trim().to_string(),
            deployment_name: self.deployment_name.trim().to_string(),
            available_models: parse_models_input(&self.models_input),
        }
    }
}

#[derive(Default)]
struct ProviderValidation {
    openai_key_warning: Option<String>,
    azure_key_warning: Option<String>,
    azure_endpoint_error: Option<String>,
    azure_version_error: Option<String>,
    azure_deployment_error: Option<String>,
}

impl ProviderValidation {
    fn is_valid(&self) -> bool {
        self.azure_endpoint_error.is_none()
            && self.azure_version_error.is_none()
            && self.azure_deployment_error.is_none()
    }
}

struct AppSectionResult {
    saved: bool,
    theme: Option<ThemeMode>,
}

impl AppSectionResult {
    fn unsaved() -> Self {
        Self {
            saved: false,
            theme: None,
        }
    }
}

struct ProjectSectionResult {
    saved: bool,
}

impl ProjectSectionResult {
    fn unsaved() -> Self {
        Self { saved: false }
    }
}

fn validate_provider(provider: LlmProviderKind, editor: &ProviderEditor) -> ProviderValidation {
    let mut validation = ProviderValidation::default();
    match provider {
        LlmProviderKind::OpenAi => {
            if editor.openai.api_key.trim().is_empty() {
                validation.openai_key_warning = Some("API key is empty".to_string());
            }
        }
        LlmProviderKind::AzureOpenAi => {
            if editor.azure.api_key.trim().is_empty() {
                validation.azure_key_warning = Some("API key is empty".to_string());
            }
            let endpoint = editor.azure.endpoint.trim();
            if endpoint.is_empty() {
                validation.azure_endpoint_error = Some("Endpoint is required".to_string());
            } else if Url::parse(endpoint).is_err() {
                validation.azure_endpoint_error = Some("Endpoint must be a valid URL".to_string());
            }
            if editor.azure.api_version.trim().is_empty() {
                validation.azure_version_error = Some("API version is required".to_string());
            }
            let deployment_raw = editor.azure.deployment_name.as_str();
            if !deployment_raw.is_empty() && deployment_raw.trim().is_empty() {
                validation.azure_deployment_error =
                    Some("Deployment name cannot be whitespace".to_string());
            }
        }
        LlmProviderKind::Mock => {}
    }
    validation
}

fn provider_before_label(provider: LlmProviderKind) -> &'static str {
    match provider {
        LlmProviderKind::OpenAi => "OpenAI",
        LlmProviderKind::AzureOpenAi => "Azure OpenAI",
        LlmProviderKind::Mock => "Mock",
    }
}

fn render_provider_panel(
    ui: &mut egui::Ui,
    palette: &ThemePalette,
    provider: &mut ProviderEditor,
    active_provider: LlmProviderKind,
    inherit: bool,
    dirty: &mut bool,
) -> ProviderValidation {
    let disabled = inherit;
    let header_color = palette.text_primary;
    ui.horizontal(|ui| {
        ui.add(Label::new(
            RichText::new("Provider details")
                .color(header_color)
                .strong(),
        ));
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            let label = if provider.details_expanded {
                "Collapse"
            } else {
                "Expand"
            };
            if ui.link(label).clicked() {
                provider.details_expanded = !provider.details_expanded;
            }
        });
    });
    ui.add_space(8.0);

    if provider.details_expanded {
        ui.scope(|ui| {
            if disabled {
                ui.set_enabled(false);
            }
            match active_provider {
                LlmProviderKind::OpenAi => {
                    render_openai_fields(ui, palette, &mut provider.openai, dirty);
                }
                LlmProviderKind::AzureOpenAi => {
                    render_azure_fields(ui, palette, &mut provider.azure, dirty);
                }
                LlmProviderKind::Mock => {}
            }
        });
        if disabled {
            ui.label(RichText::new("Using App settings").color(palette.text_secondary));
        }
    }
    let validation = if disabled {
        ProviderValidation::default()
    } else {
        validate_provider(active_provider, provider)
    };
    if provider.details_expanded && !disabled {
        match active_provider {
            LlmProviderKind::OpenAi => show_openai_validation(ui, palette, &validation),
            LlmProviderKind::AzureOpenAi => show_azure_validation(ui, palette, &validation),
            LlmProviderKind::Mock => {}
        }
    }
    validation
}

fn render_openai_fields(
    ui: &mut egui::Ui,
    palette: &ThemePalette,
    editor: &mut OpenAiEditor,
    dirty: &mut bool,
) {
    ui.label(RichText::new("OpenAI API key").strong());
    ui.horizontal(|ui| {
        let available_width = ui.available_width() - 40.0;
        let response = ui.add_sized(
            [available_width.max(120.0), 28.0],
            egui::TextEdit::singleline(&mut editor.api_key).password(!editor.reveal),
        );
        if response.changed() {
            *dirty = true;
        }
        let button = egui::Button::new(if editor.reveal { "🙈" } else { "👁" }).frame(false);
        if ui.add(button).clicked() {
            editor.reveal = !editor.reveal;
        }
    });
    ui.add_space(12.0);
    ui.label(RichText::new("Available model names").strong());
    let response = ui.add(
        egui::TextEdit::multiline(&mut editor.models_input)
            .desired_rows(3)
            .hint_text("Comma or semicolon separated"),
    );
    if response.changed() {
        *dirty = true;
    }
    ui.label(
        RichText::new("Comma or semicolon separated")
            .color(palette.text_secondary)
            .small(),
    );
    ui.add_space(6.0);
    ui.horizontal_wrapped(|ui| {
        for model in parse_models_input(&editor.models_input) {
            let _ = ui.selectable_label(false, model);
        }
    });
}

fn render_azure_fields(
    ui: &mut egui::Ui,
    palette: &ThemePalette,
    editor: &mut AzureEditor,
    dirty: &mut bool,
) {
    ui.label(RichText::new("Azure API key").strong());
    ui.horizontal(|ui| {
        let available_width = ui.available_width() - 40.0;
        let response = ui.add_sized(
            [available_width.max(120.0), 28.0],
            egui::TextEdit::singleline(&mut editor.api_key).password(!editor.reveal),
        );
        if response.changed() {
            *dirty = true;
        }
        if ui
            .add(egui::Button::new(if editor.reveal { "🙈" } else { "👁" }).frame(false))
            .clicked()
        {
            editor.reveal = !editor.reveal;
        }
    });
    ui.add_space(12.0);

    field_with_label(ui, "Endpoint", &mut editor.endpoint, dirty);
    field_with_label(ui, "API version", &mut editor.api_version, dirty);
    field_with_label(ui, "Deployment name", &mut editor.deployment_name, dirty);

    ui.add_space(12.0);
    ui.label(RichText::new("Available model names").strong());
    let response = ui.add(
        egui::TextEdit::multiline(&mut editor.models_input)
            .desired_rows(3)
            .hint_text("Comma or semicolon separated"),
    );
    if response.changed() {
        *dirty = true;
    }
    ui.label(
        RichText::new("Comma or semicolon separated")
            .color(palette.text_secondary)
            .small(),
    );
    ui.add_space(6.0);
    ui.horizontal_wrapped(|ui| {
        for model in parse_models_input(&editor.models_input) {
            let _ = ui.selectable_label(false, model);
        }
    });
}

fn show_openai_validation(
    ui: &mut egui::Ui,
    palette: &ThemePalette,
    validation: &ProviderValidation,
) {
    if let Some(warning) = validation.openai_key_warning.as_ref() {
        ui.colored_label(palette.warning, warning);
    }
}

fn show_azure_validation(
    ui: &mut egui::Ui,
    palette: &ThemePalette,
    validation: &ProviderValidation,
) {
    let error_color = Color32::from_rgb(198, 60, 60);
    if let Some(warning) = validation.azure_key_warning.as_ref() {
        ui.colored_label(palette.warning, warning);
    }
    if let Some(err) = validation.azure_endpoint_error.as_ref() {
        ui.colored_label(error_color, err);
    }
    if let Some(err) = validation.azure_version_error.as_ref() {
        ui.colored_label(error_color, err);
    }
    if let Some(err) = validation.azure_deployment_error.as_ref() {
        ui.colored_label(error_color, err);
    }
}

fn field_with_label(ui: &mut egui::Ui, label: &str, value: &mut String, dirty: &mut bool) {
    ui.label(RichText::new(label).strong());
    if ui
        .add(egui::TextEdit::singleline(value).desired_width(f32::INFINITY))
        .changed()
    {
        *dirty = true;
    }
    ui.add_space(10.0);
}
