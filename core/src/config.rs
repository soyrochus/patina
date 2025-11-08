use crate::llm::LlmProviderKind;
use directories::BaseDirs;
use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use tracing::warn;

#[derive(Debug, Clone)]
pub struct AiRuntimeSettings {
    pub provider: LlmProviderKind,
    pub openai: Option<OpenAiSettings>,
    pub azure: Option<AzureOpenAiSettings>,
    pub model: Option<String>,
}

#[derive(Debug, Clone)]
pub struct OpenAiSettings {
    pub api_key: String,
    pub model: Option<String>,
}

#[derive(Debug, Clone)]
pub struct AzureOpenAiSettings {
    pub api_key: String,
    pub endpoint: String,
    pub api_version: String,
    pub deployment_name: String,
}

#[derive(thiserror::Error, Debug)]
pub enum AiConfigError {
    #[error("AI not configured—create patina.yaml or set env vars.")]
    Missing,
    #[error("AI configuration invalid: {0}")]
    Invalid(String),
}

impl AiConfigError {
    pub fn user_message(&self) -> String {
        match self {
            Self::Missing => {
                "AI not configured—create patina.yaml or set env vars.".to_string()
            }
            Self::Invalid(detail) => format!(
                "AI not configured—{detail}. Update patina.yaml or set the corresponding environment variables."
            ),
        }
    }
}

impl AiRuntimeSettings {
    pub fn load() -> Result<Self, AiConfigError> {
        let dotenv = load_dotenv_map();
        let yaml = load_patina_yaml();
        resolve_config(
            |key| std::env::var(key).ok(),
            &dotenv,
            yaml.as_ref().and_then(|cfg| cfg.ai.as_ref()),
        )
    }
}

fn resolve_config<F>(
    env_lookup: F,
    dotenv: &HashMap<String, String>,
    yaml_ai: Option<&AiSection>,
) -> Result<AiRuntimeSettings, AiConfigError>
where
    F: Fn(&str) -> Option<String>,
{
    let provider = read_provider(&env_lookup, dotenv, yaml_ai)?;
    match provider {
        LlmProviderKind::OpenAi => {
            let api_key = read_value("OPENAI_API_KEY", &env_lookup, dotenv)
                .or_else(|| {
                    yaml_ai
                        .and_then(|ai| ai.openai.as_ref())
                        .and_then(|cfg| cfg.api_key.clone())
                })
                .ok_or_else(|| {
                    AiConfigError::Invalid("missing OpenAI api key (OPENAI_API_KEY)".to_string())
                })?;
            let model = read_value("OPENAI_MODEL", &env_lookup, dotenv)
                .or_else(|| {
                    yaml_ai
                        .and_then(|ai| ai.openai.as_ref())
                        .and_then(|cfg| cfg.model.clone())
                })
                .or_else(|| Some("gpt-4o-mini".to_string()));
            Ok(AiRuntimeSettings {
                provider,
                openai: Some(OpenAiSettings {
                    api_key,
                    model: model.clone(),
                }),
                azure: None,
                model,
            })
        }
        LlmProviderKind::AzureOpenAi => {
            let azure = yaml_ai.and_then(|ai| ai.azure_openai.as_ref());
            let api_key = read_value("AZURE_OPENAI_API_KEY", &env_lookup, dotenv)
                .or_else(|| azure.and_then(|cfg| cfg.api_key.clone()))
                .ok_or_else(|| {
                    AiConfigError::Invalid(
                        "missing Azure OpenAI api key (AZURE_OPENAI_API_KEY)".to_string(),
                    )
                })?;
            let endpoint = read_value("AZURE_OPENAI_ENDPOINT", &env_lookup, dotenv)
                .or_else(|| azure.and_then(|cfg| cfg.endpoint.clone()))
                .ok_or_else(|| {
                    AiConfigError::Invalid(
                        "missing Azure endpoint (AZURE_OPENAI_ENDPOINT)".to_string(),
                    )
                })?;
            let api_version = read_value("AZURE_OPENAI_API_VERSION", &env_lookup, dotenv)
                .or_else(|| azure.and_then(|cfg| cfg.api_version.clone()))
                .ok_or_else(|| {
                    AiConfigError::Invalid(
                        "missing Azure api version (AZURE_OPENAI_API_VERSION)".to_string(),
                    )
                })?;
            let deployment_name = read_value("AZURE_OPENAI_DEPLOYMENT_NAME", &env_lookup, dotenv)
                .or_else(|| azure.and_then(|cfg| cfg.deployment_name.clone()))
                .ok_or_else(|| {
                    AiConfigError::Invalid(
                        "missing Azure deployment name (AZURE_OPENAI_DEPLOYMENT_NAME)".to_string(),
                    )
                })?;

            Ok(AiRuntimeSettings {
                provider,
                openai: None,
                azure: Some(AzureOpenAiSettings {
                    api_key,
                    endpoint,
                    api_version,
                    deployment_name: deployment_name.clone(),
                }),
                model: Some(deployment_name),
            })
        }
        LlmProviderKind::Mock => Err(AiConfigError::Invalid(
            "mock provider is reserved for tests".to_string(),
        )),
    }
}

fn read_provider<F>(
    env_lookup: &F,
    dotenv: &HashMap<String, String>,
    yaml_ai: Option<&AiSection>,
) -> Result<LlmProviderKind, AiConfigError>
where
    F: Fn(&str) -> Option<String>,
{
    let raw = read_value("LLM_PROVIDER", env_lookup, dotenv)
        .or_else(|| yaml_ai.and_then(|ai| ai.provider.clone()))
        .ok_or(AiConfigError::Missing)?;
    match raw.to_ascii_lowercase().as_str() {
        "openai" => Ok(LlmProviderKind::OpenAi),
        "azure_openai" => Ok(LlmProviderKind::AzureOpenAi),
        other => Err(AiConfigError::Invalid(format!(
            "unrecognized provider '{other}'"
        ))),
    }
}

fn read_value<F>(key: &str, env_lookup: &F, dotenv: &HashMap<String, String>) -> Option<String>
where
    F: Fn(&str) -> Option<String>,
{
    env_lookup(key)
        .filter(|value| !value.trim().is_empty())
        .or_else(|| {
            dotenv
                .get(key)
                .cloned()
                .filter(|value| !value.trim().is_empty())
        })
}

fn load_dotenv_map() -> HashMap<String, String> {
    let path = Path::new(".env");
    if !path.exists() {
        return HashMap::new();
    }
    match dotenvy::from_path_iter(path) {
        Ok(iter) => iter
            .flatten()
            .map(|(key, value)| (key, value))
            .collect::<HashMap<_, _>>(),
        Err(err) => {
            warn!("Failed to parse .env file: {err}");
            HashMap::new()
        }
    }
}

fn load_patina_yaml() -> Option<PatinaConfig> {
    for path in patina_yaml_candidates() {
        if path.exists() {
            match fs::read_to_string(&path) {
                Ok(contents) => match serde_yaml::from_str::<PatinaConfig>(&contents) {
                    Ok(cfg) => return Some(cfg),
                    Err(err) => {
                        warn!("Failed to parse {}: {err}", path.display());
                    }
                },
                Err(err) => warn!("Failed to read {}: {err}", path.display()),
            }
        }
    }
    None
}

fn patina_yaml_candidates() -> Vec<PathBuf> {
    #[cfg(target_os = "linux")]
    {
        linux_paths()
    }
    #[cfg(target_os = "macos")]
    {
        mac_paths()
    }
    #[cfg(target_os = "windows")]
    {
        windows_paths()
    }
    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
    {
        Vec::new()
    }
}

#[cfg(target_os = "linux")]
fn linux_paths() -> Vec<PathBuf> {
    let mut paths = Vec::new();
    if let Some(base) = BaseDirs::new() {
        paths.push(base.config_dir().join("patina").join("patina.yaml"));
        paths.push(base.home_dir().join(".patina").join("patina.yaml"));
    }
    paths
}

#[cfg(target_os = "macos")]
fn mac_paths() -> Vec<PathBuf> {
    let mut paths = Vec::new();
    if let Some(base) = BaseDirs::new() {
        paths.push(
            base.home_dir()
                .join("Library")
                .join("Application Support")
                .join("Patina")
                .join("patina.yaml"),
        );
        paths.push(base.home_dir().join(".patina").join("patina.yaml"));
    }
    paths
}

#[cfg(target_os = "windows")]
fn windows_paths() -> Vec<PathBuf> {
    let mut paths = Vec::new();
    if let Some(appdata) = std::env::var_os("APPDATA") {
        paths.push(PathBuf::from(appdata).join("Patina").join("patina.yaml"));
    }
    if let Some(home) = std::env::var_os("USERPROFILE") {
        paths.push(PathBuf::from(home).join(".patina").join("patina.yaml"));
    }
    paths
}

#[derive(Debug, Deserialize)]
struct PatinaConfig {
    ai: Option<AiSection>,
}

#[derive(Debug, Deserialize)]
struct AiSection {
    provider: Option<String>,
    openai: Option<OpenAiSection>,
    #[serde(rename = "azure_openai")]
    azure_openai: Option<AzureSection>,
}

#[derive(Debug, Deserialize)]
struct OpenAiSection {
    #[serde(rename = "api_key")]
    api_key: Option<String>,
    #[serde(default)]
    model: Option<String>,
}

#[derive(Debug, Deserialize)]
struct AzureSection {
    #[serde(rename = "api_key")]
    api_key: Option<String>,
    endpoint: Option<String>,
    #[serde(rename = "api_version")]
    api_version: Option<String>,
    #[serde(rename = "deployment_name")]
    deployment_name: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn env_lookup_factory(
        map: HashMap<&'static str, &'static str>,
    ) -> impl Fn(&str) -> Option<String> {
        move |key| map.get(key).map(|value| value.to_string())
    }

    #[test]
    fn selects_openai_from_env() {
        let env = HashMap::from([
            ("LLM_PROVIDER", "openai"),
            ("OPENAI_API_KEY", "test-key"),
            ("OPENAI_MODEL", "gpt-4o-mini"),
        ]);
        let result = resolve_config(env_lookup_factory(env), &HashMap::new(), None).unwrap();
        assert!(matches!(result.provider, LlmProviderKind::OpenAi));
        assert_eq!(
            result.openai.as_ref().unwrap().api_key,
            "test-key".to_string()
        );
        assert_eq!(result.model.as_deref(), Some("gpt-4o-mini"));
    }

    #[test]
    fn uses_dotenv_when_env_missing() {
        let env = HashMap::new();
        let dotenv = HashMap::from([
            ("LLM_PROVIDER".to_string(), "openai".to_string()),
            ("OPENAI_API_KEY".to_string(), "dotenv-key".to_string()),
        ]);
        let result = resolve_config(env_lookup_factory(env), &dotenv, None).unwrap();
        assert!(matches!(result.provider, LlmProviderKind::OpenAi));
        assert_eq!(
            result.openai.as_ref().unwrap().api_key,
            "dotenv-key".to_string()
        );
    }

    #[test]
    fn falls_back_to_yaml_when_no_env_or_dotenv() {
        let env = HashMap::new();
        let dotenv = HashMap::new();
        let yaml = AiSection {
            provider: Some("azure_openai".to_string()),
            openai: None,
            azure_openai: Some(AzureSection {
                api_key: Some("yaml-key".to_string()),
                endpoint: Some("https://example.azure.com".to_string()),
                api_version: Some("2024-12-01-preview".to_string()),
                deployment_name: Some("gpt-4o".to_string()),
            }),
        };
        let result = resolve_config(env_lookup_factory(env), &dotenv, Some(&yaml)).unwrap();
        assert!(matches!(result.provider, LlmProviderKind::AzureOpenAi));
        let azure = result.azure.unwrap();
        assert_eq!(azure.api_key, "yaml-key");
        assert_eq!(azure.endpoint, "https://example.azure.com");
        assert_eq!(azure.api_version, "2024-12-01-preview");
        assert_eq!(azure.deployment_name, "gpt-4o");
    }

    #[test]
    fn errors_on_missing_provider() {
        let env = HashMap::new();
        let err = resolve_config(env_lookup_factory(env), &HashMap::new(), None).unwrap_err();
        assert!(matches!(err, AiConfigError::Missing));
    }
}
