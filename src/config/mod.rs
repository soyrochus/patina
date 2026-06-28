use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct PatinaConfig {
    pub knowledge: KnowledgeConfig,
    pub index: IndexConfig,
    pub limits: LimitsConfig,
    pub security: SecurityConfig,
    pub workspace: WorkspaceConfig,
    pub lint: LintConfig,
}

impl Default for PatinaConfig {
    fn default() -> Self {
        Self {
            knowledge: KnowledgeConfig::default(),
            index: IndexConfig::default(),
            limits: LimitsConfig::default(),
            security: SecurityConfig::default(),
            workspace: WorkspaceConfig::default(),
            lint: LintConfig::default(),
        }
    }
}

impl PatinaConfig {
    pub fn load_for_current_dir() -> Result<Self> {
        let cwd = std::env::current_dir().context("failed to read current directory")?;
        Self::load_from(&cwd)
    }

    pub fn load_from(start: &Path) -> Result<Self> {
        let config_path = find_config_path(start);
        match config_path {
            Some(path) => {
                let contents = fs::read_to_string(&path)
                    .with_context(|| format!("failed to read config file {}", path.display()))?;
                toml::from_str::<PatinaConfig>(&contents)
                    .with_context(|| format!("failed to parse TOML config {}", path.display()))
            }
            None => Ok(Self::default()),
        }
    }

    pub fn ensure_supported(&self) -> Result<()> {
        if self.workspace.enabled {
            bail!("multi-root workspaces are not supported in this version");
        }
        Ok(())
    }

    pub fn knowledge_dir(&self) -> PathBuf {
        PathBuf::from(&self.knowledge.dir)
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct KnowledgeConfig {
    pub dir: String,
}

impl Default for KnowledgeConfig {
    fn default() -> Self {
        Self {
            dir: "knowledge".to_string(),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct IndexConfig {
    pub chunk_size: usize,
    pub chunk_overlap: usize,
    pub chunk_strategy: String,
}

impl Default for IndexConfig {
    fn default() -> Self {
        Self {
            chunk_size: 1200,
            chunk_overlap: 150,
            chunk_strategy: "heading-aware".to_string(),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct LimitsConfig {
    pub max_markdown_file_mb: u64,
    pub max_source_file_mb: u64,
    pub max_total_markdown_files: u64,
    pub max_chunk_token_estimate: u64,
}

impl Default for LimitsConfig {
    fn default() -> Self {
        Self {
            max_markdown_file_mb: 10,
            max_source_file_mb: 50,
            max_total_markdown_files: 50_000,
            max_chunk_token_estimate: 1200,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct SecurityConfig {
    pub allow_internal_symlinks: bool,
    pub allow_external_symlinks: bool,
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            allow_internal_symlinks: false,
            allow_external_symlinks: false,
        }
    }
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(default)]
pub struct WorkspaceConfig {
    pub enabled: bool,
    pub roots: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(default)]
pub struct LintConfig {
    pub page_types: BTreeMap<String, PageTypeRule>,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(default)]
pub struct PageTypeRule {
    pub required: Vec<String>,
}

fn find_config_path(start: &Path) -> Option<PathBuf> {
    let current = start.join("patina.toml");
    if current.is_file() {
        return Some(current);
    }

    let repo_root = find_repo_root(start)?;
    let repo_config = repo_root.join("patina.toml");
    repo_config.is_file().then_some(repo_config)
}

fn find_repo_root(start: &Path) -> Option<PathBuf> {
    for ancestor in start.ancestors() {
        if ancestor.join(".git").exists() {
            return Some(ancestor.to_path_buf());
        }
    }
    None
}
