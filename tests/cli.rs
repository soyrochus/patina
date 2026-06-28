use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::{SystemTime, UNIX_EPOCH};

fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_patina")
}

fn temp_dir(name: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!(
        "patina-{name}-{}",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time should work")
            .as_nanos()
    ));
    fs::create_dir_all(&path).expect("temp dir should be created");
    path
}

fn run(dir: &Path, args: &[&str]) -> Output {
    Command::new(bin())
        .args(args)
        .current_dir(dir)
        .output()
        .expect("patina command should run")
}

fn json(output: &Output) -> serde_json::Value {
    serde_json::from_slice(&output.stdout).unwrap_or_else(|error| {
        panic!(
            "stdout should be JSON: {error}\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        )
    })
}

fn copy_fixture(name: &str, dest: &Path) {
    copy_dir(Path::new("tests/fixtures").join(name).as_path(), dest);
}

fn copy_dir(source: &Path, dest: &Path) {
    fs::create_dir_all(dest).expect("destination should exist");
    for entry in fs::read_dir(source).expect("fixture should be readable") {
        let entry = entry.expect("fixture entry should be readable");
        let source_path = entry.path();
        let dest_path = dest.join(entry.file_name());
        if source_path.is_dir() {
            copy_dir(&source_path, &dest_path);
        } else {
            fs::copy(&source_path, &dest_path).expect("fixture file should copy");
        }
    }
}

fn init_git(dir: &Path) {
    let status = Command::new("git")
        .arg("init")
        .current_dir(dir)
        .status()
        .expect("git init should run");
    assert!(status.success());
}

#[test]
fn init_creates_structure_and_gitignore() {
    let dir = temp_dir("init");
    init_git(&dir);

    let output = run(&dir, &["init"]);

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(dir.join("knowledge/wiki").is_dir());
    assert!(dir.join("knowledge/sources").is_dir());
    assert!(dir.join("knowledge/schemas").is_dir());
    assert!(dir.join("knowledge/README.md").is_file());
    assert!(dir.join("knowledge/AGENTS.md").is_file());
    assert!(
        fs::read_to_string(dir.join(".gitignore"))
            .expect(".gitignore should exist")
            .contains(".patina/")
    );
}

#[test]
fn lint_clean_fixture_returns_ok() {
    let dir = temp_dir("lint-clean");
    copy_fixture("valid_repo", &dir);

    let output = run(&dir, &["lint", "--json"]);
    let value = json(&output);

    assert_eq!(value["ok"], true);
    assert_eq!(value["errors"], serde_json::json!([]));
}

#[test]
fn lint_missing_front_matter_reports_required_field() {
    let dir = temp_dir("lint-missing");
    copy_fixture("missing_front_matter", &dir);

    let value = json(&run(&dir, &["lint", "--json"]));

    assert_eq!(value["ok"], false);
    assert!(
        value["errors"]
            .as_array()
            .expect("errors array")
            .iter()
            .any(|error| error["code"] == "missing_required_field")
    );
}

#[test]
fn lint_broken_link_reports_error() {
    let dir = temp_dir("lint-broken");
    copy_fixture("broken_link", &dir);

    let value = json(&run(&dir, &["lint", "--json"]));

    assert!(
        value["errors"]
            .as_array()
            .expect("errors array")
            .iter()
            .any(|error| error["code"] == "broken_link")
    );
}

#[test]
fn lint_duplicate_alias_reports_error() {
    let dir = temp_dir("lint-alias");
    copy_fixture("duplicate_alias", &dir);

    let value = json(&run(&dir, &["lint", "--json"]));

    assert!(
        value["errors"]
            .as_array()
            .expect("errors array")
            .iter()
            .any(|error| error["code"] == "duplicate_alias")
    );
}

#[test]
fn index_reset_and_query_work() {
    let dir = temp_dir("index-query");
    copy_fixture("valid_repo", &dir);

    let index = json(&run(&dir, &["index", "--reset", "--json"]));
    assert_eq!(index["ok"], true);
    assert!(
        index["data"]["files_processed"]
            .as_u64()
            .expect("file count")
            > 0
    );
    assert!(index["data"]["chunk_count"].as_u64().expect("chunk count") > 0);

    let query = json(&run(
        &dir,
        &["query", "controlled autonomy", "--json", "--explain"],
    ));
    assert_eq!(query["ok"], true);
    assert_eq!(query["data"]["mode"], "fts5");
    assert!(
        !query["data"]["results"]
            .as_array()
            .expect("results")
            .is_empty()
    );
}

#[test]
fn index_full_rebuild_after_patina_delete_allows_query() {
    let dir = temp_dir("index-full");
    copy_fixture("valid_repo", &dir);

    let _ = run(&dir, &["index", "--reset", "--json"]);
    fs::remove_dir_all(dir.join(".patina")).expect(".patina should be removable");

    let index = json(&run(&dir, &["index", "--full", "--json"]));
    let query = json(&run(&dir, &["query", "autonomy", "--json"]));

    assert_eq!(index["ok"], true);
    assert_eq!(query["ok"], true);
    assert!(
        !query["data"]["results"]
            .as_array()
            .expect("results")
            .is_empty()
    );
}

#[test]
fn stale_reports_review_after_passed() {
    let dir = temp_dir("stale");
    copy_fixture("valid_repo", &dir);
    let _ = run(&dir, &["index", "--reset", "--json"]);

    let value = json(&run(&dir, &["stale", "--json"]));

    assert!(
        value["data"]["stale_pages"]
            .as_array()
            .expect("stale pages")
            .iter()
            .flat_map(|page| page["reasons"].as_array().expect("reasons"))
            .any(|reason| reason["code"] == "review_after_passed")
    );
}

#[test]
fn doctor_json_on_initialized_repo_has_checks() {
    let dir = temp_dir("doctor");
    init_git(&dir);
    copy_fixture("valid_repo", &dir);
    fs::write(dir.join(".gitignore"), ".patina/\n").expect(".gitignore should write");
    fs::write(dir.join("knowledge/README.md"), "# Knowledge\n").expect("README should write");
    fs::write(dir.join("knowledge/AGENTS.md"), "# Agents\n").expect("AGENTS should write");
    let _ = run(&dir, &["index", "--reset", "--json"]);

    let value = json(&run(&dir, &["doctor", "--json"]));

    assert!(value["data"]["checks"].is_array());
}

#[test]
fn golden_init_output_matches_after_normalizing_temp_path() {
    let dir = temp_dir("golden-init");
    init_git(&dir);

    let output = run(&dir, &["init"]);
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    let normalized = stdout.replace(dir.to_str().expect("temp path utf8"), "<TMP>");
    let golden = fs::read_to_string("tests/golden/init.txt").expect("golden should exist");

    assert_eq!(normalized.trim(), golden.trim());
}

#[test]
fn golden_lint_clean_json_matches() {
    let dir = temp_dir("golden-lint");
    copy_fixture("valid_repo", &dir);

    let output = run(&dir, &["lint", "--json"]);
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    let golden = fs::read_to_string("tests/golden/lint_clean.json").expect("golden should exist");

    assert_eq!(stdout.trim(), golden.trim());
}

#[test]
fn read_rejects_path_traversal() {
    let base = temp_dir("read-traversal");
    let repo = base.join("repo");
    fs::create_dir_all(repo.join("knowledge")).expect("knowledge should exist");
    fs::create_dir_all(base.join("etc")).expect("etc should exist");
    fs::write(base.join("etc/passwd"), "secret").expect("outside file should exist");

    let output = run(&repo, &["read", "knowledge/../../etc/passwd"]);

    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("outside the knowledge root"));
}

#[cfg(unix)]
#[test]
fn read_rejects_external_symlink() {
    let dir = temp_dir("read-symlink");
    fs::create_dir_all(dir.join("knowledge")).expect("knowledge should exist");
    fs::write(dir.join("outside.md"), "secret").expect("outside should exist");
    std::os::unix::fs::symlink("../outside.md", dir.join("knowledge/link.md"))
        .expect("symlink should be created");

    let output = run(&dir, &["read", "knowledge/link.md"]);

    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("outside the knowledge root"));
}

#[test]
fn unsupported_schema_version_suggests_reset() {
    let dir = temp_dir("schema-version");
    fs::create_dir_all(dir.join(".patina")).expect(".patina should exist");
    let conn =
        rusqlite::Connection::open(dir.join(".patina/index.sqlite")).expect("db should open");
    conn.execute(
        "CREATE TABLE meta (key TEXT PRIMARY KEY, value TEXT NOT NULL)",
        [],
    )
    .expect("meta should create");
    conn.execute(
        "INSERT INTO meta (key, value) VALUES ('schema_version', '999')",
        [],
    )
    .expect("schema version should insert");

    let output = run(&dir, &["query", "term", "--json"]);

    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("patina index --reset"));
}

#[test]
fn fresh_index_has_schema_version_one() {
    let dir = temp_dir("schema-fresh");
    copy_fixture("valid_repo", &dir);
    let _ = run(&dir, &["index", "--reset", "--json"]);
    let conn =
        rusqlite::Connection::open(dir.join(".patina/index.sqlite")).expect("db should open");
    let version: String = conn
        .query_row(
            "SELECT value FROM meta WHERE key = 'schema_version'",
            [],
            |row| row.get(0),
        )
        .expect("schema_version should exist");

    assert_eq!(version, "1");
}

#[test]
fn title_match_ranks_above_body_only_match() {
    let dir = temp_dir("ranking");
    fs::create_dir_all(dir.join("knowledge/wiki")).expect("knowledge should exist");
    fs::write(
        dir.join("knowledge/wiki/title.md"),
        "---\ntitle: Controlled Autonomy\ntype: concept\nstatus: active\n---\n# Controlled Autonomy\n",
    )
    .expect("title page should write");
    fs::write(
        dir.join("knowledge/wiki/body.md"),
        "---\ntitle: Other Page\ntype: concept\nstatus: active\n---\n# Other\n\ncontrolled autonomy appears here\n",
    )
    .expect("body page should write");
    let _ = run(&dir, &["index", "--reset", "--json"]);

    let value = json(&run(&dir, &["query", "controlled autonomy", "--json"]));
    let first = &value["data"]["results"]
        .as_array()
        .expect("results should exist")[0];

    assert_eq!(first["path"], "knowledge/wiki/title.md");
}
