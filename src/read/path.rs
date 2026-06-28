use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};

use crate::config::SecurityConfig;

pub fn resolve_read_path(
    requested: &Path,
    knowledge_root: &Path,
    security: &SecurityConfig,
) -> Result<PathBuf> {
    if !requested.exists() {
        bail!("file not found: {}", requested.display());
    }

    let canonical_root = knowledge_root.canonicalize().with_context(|| {
        format!(
            "failed to resolve knowledge root {}",
            knowledge_root.display()
        )
    })?;
    let canonical_requested = requested
        .canonicalize()
        .with_context(|| format!("failed to resolve {}", requested.display()))?;

    if !canonical_requested.starts_with(&canonical_root) {
        bail!("path traversal rejected: path resolves outside the knowledge root");
    }

    if has_symlink_component(requested)? {
        if !security.allow_internal_symlinks {
            bail!("symlink rejected: internal symlinks are disabled");
        }
    }

    Ok(canonical_requested)
}

fn has_symlink_component(path: &Path) -> Result<bool> {
    let mut current = if path.is_absolute() {
        PathBuf::new()
    } else {
        std::env::current_dir().context("failed to read current directory")?
    };

    for component in path.components() {
        current.push(component.as_os_str());
        match current.symlink_metadata() {
            Ok(metadata) if metadata.file_type().is_symlink() => return Ok(true),
            Ok(_) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(false),
            Err(error) => {
                return Err(error)
                    .with_context(|| format!("failed to inspect {}", current.display()));
            }
        }
    }

    Ok(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::SecurityConfig;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_root(name: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!(
            "patina-{name}-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("time should work")
                .as_nanos()
        ));
        fs::create_dir_all(&path).expect("temp root should be created");
        path
    }

    fn security() -> SecurityConfig {
        SecurityConfig {
            allow_internal_symlinks: false,
            allow_external_symlinks: false,
        }
    }

    #[test]
    fn resolves_valid_path() {
        let root = temp_root("read-valid");
        let knowledge = root.join("knowledge");
        fs::create_dir_all(&knowledge).expect("knowledge should exist");
        let page = knowledge.join("page.md");
        fs::write(&page, "ok").expect("page should be written");

        let resolved =
            resolve_read_path(&page, &knowledge, &security()).expect("path should resolve");

        assert_eq!(resolved, page.canonicalize().expect("canonical path"));
    }

    #[test]
    fn rejects_traversal() {
        let root = temp_root("read-traversal");
        let knowledge = root.join("knowledge");
        let outside = root.join("outside.md");
        fs::create_dir_all(&knowledge).expect("knowledge should exist");
        fs::write(&outside, "secret").expect("outside should be written");

        let error = resolve_read_path(&knowledge.join("../outside.md"), &knowledge, &security())
            .expect_err("outside path should be rejected");

        assert!(error.to_string().contains("outside the knowledge root"));
    }

    #[test]
    fn rejects_absolute_outside_path() {
        let root = temp_root("read-absolute");
        let knowledge = root.join("knowledge");
        let outside = root.join("outside.md");
        fs::create_dir_all(&knowledge).expect("knowledge should exist");
        fs::write(&outside, "secret").expect("outside should be written");

        let error = resolve_read_path(&outside, &knowledge, &security())
            .expect_err("absolute outside path should be rejected");

        assert!(error.to_string().contains("outside the knowledge root"));
    }
}
