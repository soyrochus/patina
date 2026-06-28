use std::fs::{self, File, OpenOptions};
use std::path::{Path, PathBuf};
use std::thread;
use std::time::{Duration, Instant};

use anyhow::{Context, Result, bail};

pub struct IndexLock {
    path: PathBuf,
    _file: File,
}

impl IndexLock {
    pub fn acquire(patina_dir: &Path) -> Result<Self> {
        fs::create_dir_all(patina_dir)
            .with_context(|| format!("failed to create {}", patina_dir.display()))?;
        let path = patina_dir.join("index.lock");
        let deadline = Instant::now() + Duration::from_secs(2);

        loop {
            match OpenOptions::new().write(true).create_new(true).open(&path) {
                Ok(file) => return Ok(Self { path, _file: file }),
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                    if Instant::now() >= deadline {
                        bail!(
                            "Patina index is currently locked by another process: {}",
                            path.display()
                        );
                    }
                    thread::sleep(Duration::from_millis(100));
                }
                Err(error) => {
                    return Err(error)
                        .with_context(|| format!("failed to create lock {}", path.display()));
                }
            }
        }
    }
}

impl Drop for IndexLock {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}
