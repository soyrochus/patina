use anyhow::{Context, Result};
use directories::BaseDirs;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tracing::warn;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Scope {
    User,
    Project(PathBuf),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiSettings {
    #[serde(default = "UiSettings::default_theme_mode")]
    pub theme_mode: crate::ui::ThemeMode,
    #[serde(default = "UiSettings::default_sidebar_width")]
    pub sidebar_width: f32,
    #[serde(default = "UiSettings::default_sidebar_visible")]
    pub sidebar_visible: bool,
    #[serde(default = "UiSettings::default_window_size")]
    pub window_size: [f32; 2],
    #[serde(default)]
    pub pinned_chats: Vec<uuid::Uuid>,
    #[serde(default)]
    pub last_conversation: Option<uuid::Uuid>,
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
            theme_mode: UiSettings::default_theme_mode(),
            sidebar_width: UiSettings::default_sidebar_width(),
            sidebar_visible: UiSettings::default_sidebar_visible(),
            window_size: UiSettings::default_window_size(),
            pinned_chats: Vec::new(),
            last_conversation: None,
            model: UiSettings::default_model(),
            temperature: UiSettings::default_temperature(),
            retain_input: UiSettings::default_retain_input(),
            recent_projects: Vec::new(),
            current_project: None,
        }
    }
}

impl UiSettings {
    fn default_theme_mode() -> crate::ui::ThemeMode {
        crate::ui::ThemeMode::System
    }

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

#[derive(Debug, Clone, Deserialize)]
pub struct ProviderConfig {
    #[serde(default)]
    pub available_models: Vec<String>,
}

pub async fn load_ui_settings(scope: &Scope) -> Result<UiSettings> {
    let path = ui_settings_path(scope);
    match tokio::fs::read_to_string(&path).await {
        Ok(contents) => match serde_json::from_str(&contents) {
            Ok(settings) => Ok(settings),
            Err(err) => {
                let defaults = UiSettings::default();
                save_ui_settings(scope, &defaults).await?;
                warn!(
                    error = ?err,
                    "failed to parse ui_settings.json, resetting to defaults"
                );
                Ok(defaults)
            }
        },
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            let defaults = UiSettings::default();
            save_ui_settings(scope, &defaults).await?;
            Ok(defaults)
        }
        Err(err) => Err(err).context("failed to read ui_settings.json"),
    }
}

pub async fn save_ui_settings(scope: &Scope, settings: &UiSettings) -> Result<()> {
    let path = ui_settings_path(scope);
    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent)
            .await
            .with_context(|| format!("failed to create directory {}", parent.display()))?;
    }
    let serialized = serde_json::to_string_pretty(settings)?;
    tokio::fs::write(&path, serialized)
        .await
        .with_context(|| format!("failed to write ui_settings.json at {}", path.display()))
}

pub async fn load_provider_config(scope: &Scope) -> Result<ProviderConfig> {
    for path in provider_config_candidates(scope) {
        match tokio::fs::read_to_string(&path).await {
            Ok(contents) => match parse_provider_config(&contents) {
                Ok(config) => return Ok(config),
                Err(err) => {
                    warn!(
                        error = ?err,
                        "failed to decode patina.yaml at {}",
                        path.display()
                    );
                    return Ok(ProviderConfig {
                        available_models: Vec::new(),
                    });
                }
            },
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
                continue;
            }
            Err(err) => {
                warn!(
                    error = ?err,
                    "failed to read patina.yaml at {}",
                    path.display()
                );
                return Ok(ProviderConfig {
                    available_models: Vec::new(),
                });
            }
        }
    }

    Ok(ProviderConfig {
        available_models: Vec::new(),
    })
}

fn parse_provider_config(contents: &str) -> Result<ProviderConfig> {
    let raw: RawConfig = serde_yaml::from_str(contents)?;
    let models = raw
        .app
        .map(|section| select_models(&section))
        .unwrap_or_default();
    Ok(ProviderConfig {
        available_models: normalize_models(models),
    })
}

fn normalize_models(models: Vec<String>) -> Vec<String> {
    let mut seen = std::collections::BTreeSet::new();
    let mut output = Vec::new();
    for model in models {
        let trimmed = model.trim();
        if trimmed.is_empty() {
            continue;
        }
        let key = trimmed.to_ascii_lowercase();
        if seen.insert(key) {
            output.push(trimmed.to_string());
        }
    }
    output
}

fn select_models(app: &RawAppSection) -> Vec<String> {
    let provider = app
        .provider
        .as_deref()
        .unwrap_or("openai")
        .to_ascii_lowercase();
    let mut models = match provider.as_str() {
        "azure_openai" | "azure-openai" => app
            .azure_openai
            .as_ref()
            .map(|section| section.available_models.clone())
            .unwrap_or_default(),
        "mock" => app.available_models.clone(),
        _ => app
            .openai
            .as_ref()
            .map(|section| section.available_models.clone())
            .unwrap_or_default(),
    };

    if models.is_empty() {
        if !app.available_models.is_empty() {
            models = app.available_models.clone();
        } else {
            let mut fallback = Vec::new();
            if let Some(section) = &app.openai {
                fallback.extend(section.available_models.clone());
            }
            if let Some(section) = &app.azure_openai {
                fallback.extend(section.available_models.clone());
            }
            models = fallback;
        }
    }

    models
}

fn ui_settings_path(scope: &Scope) -> PathBuf {
    match scope {
        Scope::User => config_dir().join("ui_settings.json"),
        Scope::Project(path) => project_dir(path).join("ui_settings.json"),
    }
}

fn provider_config_candidates(scope: &Scope) -> Vec<PathBuf> {
    let mut paths = Vec::new();
    match scope {
        Scope::User => {
            let dir = config_dir();
            paths.push(dir.join("patina.yaml"));
            paths.push(dir.join("patina.yml"));
        }
        Scope::Project(path) => {
            let base = project_dir(path);
            paths.push(base.join("patina.yaml"));
            paths.push(base.join("patina.yml"));
        }
    }
    paths
}

fn project_dir(path: &Path) -> PathBuf {
    path.join(".patina")
}

fn config_dir() -> PathBuf {
    if let Some(base) = BaseDirs::new() {
        base.config_dir().join("patina")
    } else {
        PathBuf::from(".patina")
    }
}

#[derive(Debug, Deserialize)]
struct RawConfig {
    app: Option<RawAppSection>,
}

#[derive(Debug, Deserialize)]
struct RawAppSection {
    #[serde(default)]
    provider: Option<String>,
    #[serde(default)]
    available_models: Vec<String>,
    #[serde(default)]
    openai: Option<RawProviderSection>,
    #[serde(default, rename = "azure_openai")]
    azure_openai: Option<RawProviderSection>,
}

#[derive(Debug, Default, Deserialize)]
struct RawProviderSection {
    #[serde(default)]
    available_models: Vec<String>,
}
