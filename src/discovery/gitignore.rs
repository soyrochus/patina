use std::fs;
use std::path::Path;
use std::process::Command;

use anyhow::{Context, Result};

pub fn has_entry(file: &Path, entry: &str) -> Result<bool> {
    if !file.exists() {
        return Ok(false);
    }

    let contents =
        fs::read_to_string(file).with_context(|| format!("failed to read {}", file.display()))?;
    Ok(contents.lines().any(|line| line.trim() == entry))
}

pub fn is_ignored(path: &Path) -> Result<bool> {
    let output = Command::new("git")
        .arg("check-ignore")
        .arg("--quiet")
        .arg(path)
        .status();

    match output {
        Ok(status) => Ok(status.success()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(error).context("failed to run git check-ignore"),
    }
}
