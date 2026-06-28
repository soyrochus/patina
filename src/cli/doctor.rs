use std::path::Path;

use anyhow::Result;
use clap::Args;
use rusqlite::{Connection, OpenFlags};
use serde::Serialize;

use crate::config::PatinaConfig;
use crate::db::init as db_init;
use crate::discovery::{git, gitignore, scope};
use crate::output::{JsonEnvelope, print_json};

#[derive(Debug, Args)]
pub struct DoctorArgs {
    #[arg(long)]
    pub json: bool,
}

#[derive(Debug, Clone, Serialize)]
struct DoctorCheck {
    name: String,
    status: String,
    message: String,
}

#[derive(Debug, Serialize)]
struct DoctorData {
    checks: Vec<DoctorCheck>,
}

pub fn run(args: DoctorArgs, config: &PatinaConfig) -> Result<()> {
    let checks = run_checks(config)?;
    let ok = checks.iter().all(|check| check.status != "error");

    if args.json {
        print_json(&JsonEnvelope::new(
            "doctor",
            ok,
            Some(DoctorData { checks }),
            Vec::new(),
            Vec::new(),
        ))?;
    } else {
        for check in checks {
            println!("{}\t{}\t{}", check.status, check.name, check.message);
        }
    }
    Ok(())
}

fn run_checks(config: &PatinaConfig) -> Result<Vec<DoctorCheck>> {
    let mut checks = Vec::new();
    let worktree = git::detect()?;
    let knowledge_dir = config.knowledge_dir();
    let patina_dir = Path::new(".patina");
    let db_path = patina_dir.join("index.sqlite");

    checks.push(check(
        "git_worktree",
        if worktree.inside { "ok" } else { "warning" },
        if worktree.inside {
            "Git worktree detected"
        } else {
            "No Git worktree detected"
        },
    ));
    checks.push(check(
        "knowledge_dir",
        if knowledge_dir.is_dir() {
            "ok"
        } else {
            "error"
        },
        if knowledge_dir.is_dir() {
            "Knowledge directory exists"
        } else {
            "Knowledge directory is missing"
        },
    ));
    checks.push(path_check(
        "knowledge_readme",
        &knowledge_dir.join("README.md"),
        "warning",
    ));
    checks.push(path_check(
        "knowledge_agents",
        &knowledge_dir.join("AGENTS.md"),
        "warning",
    ));
    checks.push(check(
        "patina_dir",
        if patina_dir.exists() {
            "ok"
        } else if parent_writable(patina_dir) {
            "warning"
        } else {
            "error"
        },
        if patina_dir.exists() {
            ".patina/ exists"
        } else {
            ".patina/ does not exist yet"
        },
    ));
    checks.push(check(
        "patina_gitignore",
        if gitignore::has_entry(Path::new(".gitignore"), ".patina/")? {
            "ok"
        } else {
            "warning"
        },
        ".patina/ should be ignored by Git",
    ));

    if db_path.exists() {
        match Connection::open_with_flags(&db_path, OpenFlags::SQLITE_OPEN_READ_ONLY) {
            Ok(conn) => {
                checks.push(sqlite_integrity_check(&conn));
                checks.push(schema_version_check(&conn));
            }
            Err(error) => checks.push(check(
                "sqlite_open",
                "error",
                format!("failed to open SQLite database: {error}"),
            )),
        }
    } else {
        checks.push(check(
            "sqlite_database",
            "warning",
            "SQLite database does not exist; run patina index --reset",
        ));
    }

    let memory = Connection::open_in_memory()?;
    checks.push(check(
        "fts5_available",
        if db_init::check_fts5_available(&memory) {
            "ok"
        } else {
            "warning"
        },
        "SQLite FTS5 availability checked",
    ));
    checks.push(check(
        "patina_permissions",
        if patina_dir.exists() && patina_dir.metadata()?.permissions().readonly() {
            "error"
        } else {
            "ok"
        },
        ".patina/ permissions checked",
    ));
    checks.push(check(
        "agent_instructions",
        if knowledge_dir.join("AGENTS.md").exists() {
            "ok"
        } else {
            "warning"
        },
        "Agent instruction file checked",
    ));

    let scope = scope::load(&knowledge_dir)?;
    checks.push(check(
        "scope_yaml",
        if scope.warnings.is_empty() {
            "ok"
        } else {
            "warning"
        },
        "scope.yaml validity checked",
    ));
    checks.push(check(
        "large_file_limits",
        if config.limits.max_markdown_file_mb > 0 && config.limits.max_total_markdown_files > 0 {
            "ok"
        } else {
            "warning"
        },
        "Large-file limits configured",
    ));

    Ok(checks)
}

fn sqlite_integrity_check(conn: &Connection) -> DoctorCheck {
    match conn.query_row("PRAGMA integrity_check", [], |row| row.get::<_, String>(0)) {
        Ok(result) if result == "ok" => {
            check("sqlite_integrity", "ok", "SQLite integrity check passed")
        }
        Ok(result) => check(
            "sqlite_integrity",
            "error",
            format!("SQLite integrity check failed: {result}; run patina index --reset"),
        ),
        Err(error) => check(
            "sqlite_integrity",
            "error",
            format!("SQLite integrity check failed: {error}; run patina index --reset"),
        ),
    }
}

fn schema_version_check(conn: &Connection) -> DoctorCheck {
    match db_init::validate_schema_version(conn) {
        Ok(()) => check("schema_version", "ok", "Index schema version is supported"),
        Err(error) => check("schema_version", "error", error.to_string()),
    }
}

fn path_check(name: &str, path: &Path, missing_status: &str) -> DoctorCheck {
    check(
        name,
        if path.exists() { "ok" } else { missing_status },
        if path.exists() {
            format!("{} exists", path.display())
        } else {
            format!("{} is missing", path.display())
        },
    )
}

fn parent_writable(path: &Path) -> bool {
    path.parent()
        .and_then(|parent| parent.metadata().ok())
        .map(|metadata| !metadata.permissions().readonly())
        .unwrap_or(false)
}

fn check(
    name: impl Into<String>,
    status: impl Into<String>,
    message: impl Into<String>,
) -> DoctorCheck {
    DoctorCheck {
        name: name.into(),
        status: status.into(),
        message: message.into(),
    }
}
