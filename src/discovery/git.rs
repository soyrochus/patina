use std::path::PathBuf;
use std::process::Command;

use anyhow::{Context, Result};

#[derive(Debug, Clone)]
pub struct GitWorktree {
    pub inside: bool,
    pub root: Option<PathBuf>,
}

pub fn detect() -> Result<GitWorktree> {
    let cwd = std::env::current_dir().context("failed to read current directory")?;
    for ancestor in cwd.ancestors() {
        if ancestor.join(".git").exists() {
            return Ok(GitWorktree {
                inside: true,
                root: Some(ancestor.to_path_buf()),
            });
        }
    }

    let inside = Command::new("git")
        .current_dir(&cwd)
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .args(["rev-parse", "--is-inside-work-tree"])
        .output();

    if let Ok(output) = inside {
        if output.status.success() && String::from_utf8_lossy(&output.stdout).trim() == "true" {
            let root = Command::new("git")
                .current_dir(&cwd)
                .env_remove("GIT_DIR")
                .env_remove("GIT_WORK_TREE")
                .args(["rev-parse", "--show-toplevel"])
                .output()
                .ok()
                .filter(|output| output.status.success())
                .map(|output| PathBuf::from(String::from_utf8_lossy(&output.stdout).trim()));
            return Ok(GitWorktree { inside: true, root });
        }
    }

    Ok(GitWorktree {
        inside: false,
        root: None,
    })
}
