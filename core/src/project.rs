use crate::store::TranscriptStore;
use anyhow::{anyhow, Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::{self, Read, Seek, Write};
use std::path::{Component, Path, PathBuf};
use walkdir::WalkDir;
use zip::write::FileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProjectPaths {
    pub root: PathBuf,
    pub pat_file: PathBuf,
    pub internal: PathBuf,
    pub conversations: PathBuf,
}

impl ProjectPaths {
    pub fn new(
        root: PathBuf,
        pat_file: PathBuf,
        internal: PathBuf,
        conversations: PathBuf,
    ) -> Self {
        Self {
            root,
            pat_file,
            internal,
            conversations,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct ProjectManifestPaths {
    internal: String,
    conversations: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct ProjectManifest {
    version: u32,
    name: String,
    created_utc: DateTime<Utc>,
    paths: ProjectManifestPaths,
}

#[derive(Clone, Debug)]
pub struct ProjectHandle {
    manifest: ProjectManifest,
    paths: ProjectPaths,
}

impl ProjectHandle {
    pub fn create(at: &Path, name: &str) -> Result<Self> {
        let trimmed = name.trim();
        if trimmed.is_empty() {
            return Err(anyhow!("project name cannot be empty"));
        }

        let is_manifest_path = at
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| ext.eq_ignore_ascii_case("pat"))
            .unwrap_or(false);

        let (root, manifest_name) = if is_manifest_path {
            let stem = at
                .file_stem()
                .and_then(|stem| stem.to_str())
                .ok_or_else(|| anyhow!("project manifest must have a valid file name"))?;
            if stem != trimmed {
                return Err(anyhow!(
                    "project name must match the selected .pat file name"
                ));
            }
            (at.with_extension(""), stem.to_string())
        } else {
            let root = if at
                .file_name()
                .and_then(|os| os.to_str())
                .map(|existing| existing == trimmed)
                .unwrap_or(false)
            {
                at.to_path_buf()
            } else {
                at.join(trimmed)
            };
            (root, trimmed.to_string())
        };

        if root.exists() {
            if !root.is_dir() {
                return Err(anyhow!("project path exists and is not a directory"));
            }
            if root.read_dir()?.next().is_some() {
                return Err(anyhow!("project directory is not empty"));
            }
        } else {
            fs::create_dir_all(&root).with_context(|| {
                format!("failed to create project directory at {}", root.display())
            })?;
        }

        let pat_path = root.join(format!("{}.pat", manifest_name));
        let manifest = ProjectManifest {
            version: 1,
            name: manifest_name.clone(),
            created_utc: Utc::now(),
            paths: ProjectManifestPaths {
                internal: ".patina".to_string(),
                conversations: ".patina/conversations".to_string(),
            },
        };

        let internal_dir = root.join(&manifest.paths.internal);
        let conversations_dir = root.join(&manifest.paths.conversations);
        fs::create_dir_all(&conversations_dir).with_context(|| {
            format!(
                "failed to create conversations directory at {}",
                conversations_dir.display()
            )
        })?;

        fs::create_dir_all(&internal_dir).with_context(|| {
            format!(
                "failed to create internal directory at {}",
                internal_dir.display()
            )
        })?;

        let serialized = toml::to_string_pretty(&manifest)?;
        fs::write(&pat_path, serialized)
            .with_context(|| format!("failed to write manifest at {}", pat_path.display()))?;

        let paths = ProjectPaths::new(root.clone(), pat_path, internal_dir, conversations_dir);
        Ok(Self { manifest, paths })
    }

    pub fn open(from: &Path) -> Result<Self> {
        let pat_file = if from.is_dir() {
            let dir_name = from
                .file_name()
                .ok_or_else(|| anyhow!("project directory is missing a name"))?;
            let expected = from.join(format!("{}.pat", dir_name.to_string_lossy()));
            expected
        } else {
            from.to_path_buf()
        };

        if pat_file.extension().and_then(|ext| ext.to_str()) != Some("pat") {
            return Err(anyhow!("project manifest must have .pat extension"));
        }

        if !pat_file.exists() {
            return Err(anyhow!(
                "project manifest does not exist at {}",
                pat_file.display()
            ));
        }

        let root = pat_file
            .parent()
            .ok_or_else(|| anyhow!("project manifest must reside inside a directory"))?
            .to_path_buf();

        let contents = fs::read_to_string(&pat_file).with_context(|| {
            format!("failed to read project manifest at {}", pat_file.display())
        })?;
        let manifest: ProjectManifest = toml::from_str(&contents)
            .with_context(|| format!("invalid project manifest at {}", pat_file.display()))?;

        let internal = normalize_relative_path(&root, &manifest.paths.internal)?;
        let conversations = normalize_relative_path(&root, &manifest.paths.conversations)?;
        if !internal.starts_with(&root) {
            return Err(anyhow!("internal path escapes project root"));
        }
        if !conversations.starts_with(&root) {
            return Err(anyhow!("conversations path escapes project root"));
        }

        fs::create_dir_all(&internal).with_context(|| {
            format!(
                "failed to create internal directory at {}",
                internal.display()
            )
        })?;
        fs::create_dir_all(&conversations).with_context(|| {
            format!(
                "failed to ensure conversations directory exists at {}",
                conversations.display()
            )
        })?;

        let paths = ProjectPaths::new(root.clone(), pat_file, internal, conversations);

        Ok(Self { manifest, paths })
    }

    pub fn import_zip<R: Read + Seek>(reader: R, into_dir: &Path) -> Result<Self> {
        if into_dir.exists() {
            if !into_dir.is_dir() {
                return Err(anyhow!("import destination is not a directory"));
            }
            if into_dir.read_dir()?.next().is_some() {
                return Err(anyhow!("import destination must be empty"));
            }
        } else {
            fs::create_dir_all(into_dir).with_context(|| {
                format!(
                    "failed to create destination directory at {}",
                    into_dir.display()
                )
            })?;
        }

        let mut archive = ZipArchive::new(reader)?;
        let mut root_component: Option<PathBuf> = None;

        for i in 0..archive.len() {
            let mut file = archive.by_index(i)?;
            let name = file.mangled_name();
            let mut components = name.components();
            let first_component = components.next();
            if let Some(component) = first_component {
                if let Component::Normal(value) = component {
                    let candidate = PathBuf::from(value);
                    match &root_component {
                        Some(existing) if existing != &candidate => {
                            return Err(anyhow!("archive contains multiple root directories"));
                        }
                        None => {
                            root_component = Some(candidate);
                        }
                        _ => {}
                    }
                }
            }

            let out_path = into_dir.join(&name);
            if file.is_dir() {
                fs::create_dir_all(&out_path)?;
            } else {
                if let Some(parent) = out_path.parent() {
                    fs::create_dir_all(parent)?;
                }
                let mut output = fs::File::create(&out_path)?;
                io::copy(&mut file, &mut output)?;
            }
        }

        let root_name =
            root_component.ok_or_else(|| anyhow!("archive did not contain a project directory"))?;
        let project_root = into_dir.join(root_name);
        Self::open(&project_root)
    }

    pub fn export_zip<W: Write + Seek>(&self, writer: W) -> Result<()> {
        let mut zip = ZipWriter::new(writer);
        let options = FileOptions::default().compression_method(CompressionMethod::Deflated);
        let root_name = self
            .paths
            .root
            .file_name()
            .ok_or_else(|| anyhow!("project root is missing a name"))?
            .to_string_lossy()
            .to_string();

        zip.add_directory(format!("{}/", root_name), options)?;

        for entry in WalkDir::new(&self.paths.root).into_iter() {
            let entry = entry?;
            let path = entry.path();
            if path == self.paths.root {
                continue;
            }
            let relative = path.strip_prefix(&self.paths.root)?;
            let mut zip_path = PathBuf::from(&root_name);
            if !relative.as_os_str().is_empty() {
                zip_path.push(relative);
            }

            if entry.file_type().is_dir() {
                let mut name = zip_path.to_string_lossy().replace("\\", "/");
                if !name.ends_with('/') {
                    name.push('/');
                }
                zip.add_directory(name, options)?;
            } else {
                let mut file = fs::File::open(path)?;
                zip.start_file(zip_path.to_string_lossy().replace("\\", "/"), options)?;
                io::copy(&mut file, &mut zip)?;
            }
        }

        zip.finish()?;
        Ok(())
    }

    pub fn name(&self) -> &str {
        &self.manifest.name
    }

    pub fn created_at(&self) -> DateTime<Utc> {
        self.manifest.created_utc
    }

    pub fn paths(&self) -> &ProjectPaths {
        &self.paths
    }

    pub fn transcript_store(&self) -> TranscriptStore {
        TranscriptStore::new(self.paths.internal.clone())
    }

    pub fn metadata_path(&self) -> &Path {
        &self.paths.pat_file
    }
}

fn normalize_relative_path(root: &Path, relative: &str) -> Result<PathBuf> {
    let mut result = PathBuf::from(root);
    for component in Path::new(relative).components() {
        match component {
            Component::CurDir => {}
            Component::Normal(part) => result.push(part),
            Component::ParentDir => {
                if !result.pop() {
                    return Err(anyhow!("relative path escapes project root"));
                }
            }
            Component::RootDir | Component::Prefix(_) => {
                return Err(anyhow!("project paths must be relative"));
            }
        }
    }
    Ok(result)
}
