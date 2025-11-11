use crate::llm::LlmProviderKind;
use directories::BaseDirs;
use serde::Deserialize;
use std::fs;
use std::path::PathBuf;

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
    #[error("AI not configured—create patina.yaml with provider credentials.")]
    Missing,
    #[error("AI configuration invalid: {0}")]
    Invalid(String),
}

impl AiConfigError {
    pub fn user_message(&self) -> String {
        match self {
            Self::Missing => {
                "AI not configured—create patina.yaml with provider credentials.".to_string()
            }
            Self::Invalid(detail) => {
                format!("AI not configured—{detail}. Update patina.yaml.")
            }
        }
    }
}

impl AiRuntimeSettings {
    pub fn load() -> Result<Self, AiConfigError> {
        let path = locate_config_file().ok_or(AiConfigError::Missing)?;
        let contents = fs::read_to_string(&path).map_err(|err| {
            AiConfigError::Invalid(format!("failed to read {}: {err}", path.display()))
        })?;
        let config: PatinaConfig = serde_yaml::from_str(&contents)
            .map_err(|err| AiConfigError::Invalid(format!("invalid patina.yaml: {err}")))?;
        let app = config
            .app
            .ok_or_else(|| AiConfigError::Invalid("missing `app` section".to_string()))?;
        resolve_app_settings(app)
    }
}

fn resolve_app_settings(app: AppSection) -> Result<AiRuntimeSettings, AiConfigError> {
    let provider = app.provider.unwrap_or(LlmProviderKind::OpenAi);
    match provider {
        LlmProviderKind::OpenAi => {
            let section = app.openai.unwrap_or_default();
            let api_key = section.api_key.trim().to_string();
            if api_key.is_empty() {
                return Err(AiConfigError::Invalid(
                    "missing OpenAI api key in patina.yaml".to_string(),
                ));
            }
            Ok(AiRuntimeSettings {
                provider,
                openai: Some(OpenAiSettings {
                    api_key,
                    model: None,
                }),
                azure: None,
                model: None,
            })
        }
        LlmProviderKind::AzureOpenAi => {
            let section = app.azure_openai.unwrap_or_default();
            let api_key = section.api_key.trim().to_string();
            if api_key.is_empty() {
                return Err(AiConfigError::Invalid(
                    "missing Azure OpenAI api key in patina.yaml".to_string(),
                ));
            }
            let endpoint = section.endpoint.trim().to_string();
            if endpoint.is_empty() {
                return Err(AiConfigError::Invalid(
                    "missing Azure endpoint in patina.yaml".to_string(),
                ));
            }
            let api_version = section.api_version.trim().to_string();
            if api_version.is_empty() {
                return Err(AiConfigError::Invalid(
                    "missing Azure api version in patina.yaml".to_string(),
                ));
            }
            let deployment_name = section.deployment_name.trim().to_string();
            if deployment_name.is_empty() {
                return Err(AiConfigError::Invalid(
                    "missing Azure deployment name in patina.yaml".to_string(),
                ));
            }
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
        LlmProviderKind::Mock => Ok(AiRuntimeSettings {
            provider,
            openai: None,
            azure: None,
            model: None,
        }),
    }
}

fn locate_config_file() -> Option<PathBuf> {
    patina_yaml_candidates()
        .into_iter()
        .find(|path| path.exists())
}

fn patina_yaml_candidates() -> Vec<PathBuf> {
    let mut paths = Vec::new();
    if let Some(base) = BaseDirs::new() {
        let config_dir = base.config_dir().join("patina");
        paths.push(config_dir.join("patina.yaml"));
        paths.push(config_dir.join("patina.yml"));
        let home_dir = base.home_dir();
        paths.push(home_dir.join(".patina").join("patina.yaml"));
        paths.push(home_dir.join(".patina").join("patina.yml"));
    } else {
        paths.push(PathBuf::from("patina.yaml"));
        paths.push(PathBuf::from("patina.yml"));
    }
    paths
}

#[derive(Debug, Deserialize)]
struct PatinaConfig {
    app: Option<AppSection>,
}

#[derive(Debug, Deserialize)]
struct AppSection {
    provider: Option<LlmProviderKind>,
    openai: Option<OpenAiSection>,
    #[serde(rename = "azure_openai")]
    azure_openai: Option<AzureSection>,
}

#[derive(Debug, Default, Deserialize)]
struct OpenAiSection {
    #[serde(default)]
    api_key: String,
}

#[derive(Debug, Default, Deserialize)]
struct AzureSection {
    #[serde(default)]
    api_key: String,
    #[serde(default)]
    endpoint: String,
    #[serde(default, rename = "api_version")]
    api_version: String,
    #[serde(default, rename = "deployment_name")]
    deployment_name: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_openai_settings() {
        let app = AppSection {
            provider: Some(LlmProviderKind::OpenAi),
            openai: Some(OpenAiSection {
                api_key: "test-key".into(),
            }),
            azure_openai: None,
        };
        let settings = resolve_app_settings(app).expect("openai settings");
        assert!(matches!(settings.provider, LlmProviderKind::OpenAi));
        assert_eq!(settings.openai.as_ref().unwrap().api_key, "test-key");
    }

    #[test]
    fn resolves_azure_settings() {
        let app = AppSection {
            provider: Some(LlmProviderKind::AzureOpenAi),
            openai: None,
            azure_openai: Some(AzureSection {
                api_key: "azure-key".into(),
                endpoint: "https://example.azure.com".into(),
                api_version: "2024-12-01-preview".into(),
                deployment_name: "gpt-4o".into(),
            }),
        };
        let settings = resolve_app_settings(app).expect("azure settings");
        assert!(matches!(settings.provider, LlmProviderKind::AzureOpenAi));
        let azure = settings.azure.as_ref().unwrap();
        assert_eq!(azure.api_key, "azure-key");
        assert_eq!(azure.endpoint, "https://example.azure.com");
        assert_eq!(azure.api_version, "2024-12-01-preview");
        assert_eq!(azure.deployment_name, "gpt-4o");
    }

    #[test]
    fn errors_without_credentials() {
        let app = AppSection {
            provider: Some(LlmProviderKind::OpenAi),
            openai: Some(OpenAiSection {
                api_key: String::new(),
            }),
            azure_openai: None,
        };
        let err = resolve_app_settings(app).unwrap_err();
        assert!(matches!(err, AiConfigError::Invalid(_)));
    }
}
