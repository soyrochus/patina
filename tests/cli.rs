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

fn read_file(path: impl AsRef<Path>) -> String {
    fs::read_to_string(path).expect("file should be readable")
}

fn assert_skill_file(path: &Path, expected_name: &str) {
    let contents = read_file(path);
    assert!(contents.contains("---\n"));
    assert!(contents.contains(&format!("name: {expected_name}")));
    assert!(contents.contains("description:"));
    assert!(contents.contains("license: MIT"));
    assert!(contents.contains("<!-- PATINA GENERATED SKILL -->"));
    assert!(!contents.contains("allowed-tools"));
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
    let controlled_autonomy = query["data"]["results"]
        .as_array()
        .expect("results")
        .iter()
        .find(|result| {
            result["path"]
                .as_str()
                .is_some_and(|path| path.contains("controlled-autonomy.md"))
        })
        .expect("controlled-autonomy result should exist");
    assert_eq!(controlled_autonomy["score_components"]["alias"], 1.0);
    assert_eq!(controlled_autonomy["score_components"]["freshness"], 1.0);

    let tag_query = json(&run(&dir, &["query", "agents", "--json", "--explain"]));
    let agent_boundaries = tag_query["data"]["results"]
        .as_array()
        .expect("results")
        .iter()
        .find(|result| {
            result["path"]
                .as_str()
                .is_some_and(|path| path.contains("agent-boundaries.md"))
        })
        .expect("agent-boundaries result should exist");
    assert_eq!(agent_boundaries["score_components"]["tag"], 1.0);
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
fn doctor_human_output_aligns_columns() {
    let dir = temp_dir("doctor-columns");
    init_git(&dir);
    copy_fixture("valid_repo", &dir);
    fs::write(dir.join(".gitignore"), ".patina/\n").expect(".gitignore should write");
    fs::write(dir.join("knowledge/README.md"), "# Knowledge\n").expect("README should write");
    fs::write(dir.join("knowledge/AGENTS.md"), "# Agents\n").expect("AGENTS should write");
    let _ = run(&dir, &["index", "--reset", "--json"]);

    let output = run(&dir, &["doctor"]);
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    assert!(!stdout.contains('\t'));

    let mut column_starts = stdout
        .lines()
        .map(|line| {
            line.char_indices()
                .filter_map(|(index, character)| {
                    if !character.is_whitespace()
                        && (index == 0
                            || line[..index]
                                .chars()
                                .next_back()
                                .is_some_and(|character| character.is_whitespace()))
                    {
                        Some(index)
                    } else {
                        None
                    }
                })
                .take(3)
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();

    assert!(!column_starts.is_empty());
    let expected = column_starts.remove(0);
    assert_eq!(expected.len(), 3);
    for starts in column_starts {
        assert_eq!(starts[..3], expected[..3]);
    }
}

#[test]
fn install_skills_no_targets_writes_shared_agents_only() {
    let dir = temp_dir("install-skills-shared");

    let output = run(&dir, &["install-skills"]);

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let agents = read_file(dir.join("knowledge/AGENTS.md"));
    assert!(agents.contains("<!-- BEGIN PATINA AGENT INSTRUCTIONS -->"));
    assert!(agents.contains("patina query"));
    assert!(agents.contains("patina index"));
    assert!(!dir.join(".github/skills/patina-query/SKILL.md").exists());
    assert!(!dir.join(".agents/skills/patina-query/SKILL.md").exists());
    assert!(!dir.join(".claude/skills/patina-query/SKILL.md").exists());
}

#[test]
fn install_skills_github_copilot_writes_github_skills() {
    let dir = temp_dir("install-skills-github");

    let output = run(&dir, &["install-skills", "--for", "github-copilot"]);

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(dir.join("knowledge/AGENTS.md").exists());
    assert_skill_file(
        &dir.join(".github/skills/patina-query/SKILL.md"),
        "patina-query",
    );
    assert_skill_file(
        &dir.join(".github/skills/patina-check/SKILL.md"),
        "patina-check",
    );
    assert!(!dir.join(".github/prompts/patina-query.prompt.md").exists());
    assert!(!dir.join(".github/prompts/patina-check.prompt.md").exists());
}

#[test]
fn install_skills_codex_writes_agents_skills_and_root_agents() {
    let dir = temp_dir("install-skills-codex");

    let output = run(&dir, &["install-skills", "--for", "codex"]);

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_skill_file(
        &dir.join(".agents/skills/patina-query/SKILL.md"),
        "patina-query",
    );
    assert_skill_file(
        &dir.join(".agents/skills/patina-check/SKILL.md"),
        "patina-check",
    );
    let root_agents = read_file(dir.join("AGENTS.md"));
    assert!(root_agents.contains("<!-- BEGIN PATINA CODEX CONTEXT -->"));
    assert!(root_agents.contains(".agents/skills/"));
    assert!(root_agents.contains("knowledge/AGENTS.md"));
}

#[test]
fn install_skills_claude_code_writes_claude_skills() {
    let dir = temp_dir("install-skills-claude");

    let output = run(&dir, &["install-skills", "--for", "claude-code"]);

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_skill_file(
        &dir.join(".claude/skills/patina-query/SKILL.md"),
        "patina-query",
    );
    assert_skill_file(
        &dir.join(".claude/skills/patina-check/SKILL.md"),
        "patina-check",
    );
    let query = read_file(dir.join(".claude/skills/patina-query/SKILL.md"));
    assert!(!query.contains("slash command"));
}

#[test]
fn install_skills_all_writes_all_targets() {
    let dir = temp_dir("install-skills-all");

    let value = json(&run(&dir, &["install-skills", "--for", "all", "--json"]));

    assert_eq!(value["ok"], true);
    assert!(dir.join(".github/skills/patina-query/SKILL.md").exists());
    assert!(dir.join(".agents/skills/patina-query/SKILL.md").exists());
    assert!(dir.join(".claude/skills/patina-query/SKILL.md").exists());
    assert!(
        value["warnings"]
            .as_array()
            .expect("warnings")
            .iter()
            .any(|warning| warning["code"] == "duplicate_skill_names_possible")
    );
}

#[test]
fn install_skills_repeated_for_flags() {
    let dir = temp_dir("install-skills-repeated");

    let output = run(
        &dir,
        &[
            "install-skills",
            "--for",
            "github-copilot",
            "--for",
            "codex",
        ],
    );

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(dir.join(".github/skills/patina-query/SKILL.md").exists());
    assert!(dir.join(".agents/skills/patina-query/SKILL.md").exists());
    assert!(!dir.join(".claude/skills/patina-query/SKILL.md").exists());
}

#[test]
fn install_skills_substitutes_custom_knowledge_dir() {
    let dir = temp_dir("install-skills-custom-knowledge");
    fs::write(
        dir.join("patina.toml"),
        "[knowledge]\ndir = \"docs/knowledge\"\n",
    )
    .expect("config should write");

    let output = run(&dir, &["install-skills", "--for", "codex"]);

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(dir.join("docs/knowledge/AGENTS.md").exists());
    let skill = read_file(dir.join(".agents/skills/patina-query/SKILL.md"));
    let root_agents = read_file(dir.join("AGENTS.md"));
    assert!(skill.contains("docs/knowledge/AGENTS.md"));
    assert!(root_agents.contains("docs/knowledge/AGENTS.md"));
}

#[test]
fn install_skills_json_reports_written_files() {
    let dir = temp_dir("install-skills-json");

    let value = json(&run(
        &dir,
        &["install-skills", "--for", "github-copilot", "--json"],
    ));

    assert_eq!(value["ok"], true);
    assert_eq!(value["command"], "install-skills");
    assert_eq!(
        value["data"]["targets"],
        serde_json::json!(["github-copilot"])
    );
    let files = value["data"]["files_written"].as_array().expect("files");
    assert!(files.iter().any(|path| path == "knowledge/AGENTS.md"));
    assert!(
        files
            .iter()
            .any(|path| path == ".github/skills/patina-query/SKILL.md")
    );
    assert!(
        files
            .iter()
            .any(|path| path == ".github/skills/patina-check/SKILL.md")
    );
}

#[test]
fn install_agent_alias_emits_deprecation_warning() {
    let dir = temp_dir("install-agent-alias");

    let value = json(&run(&dir, &["install-agent", "--json"]));

    assert_eq!(value["ok"], true);
    assert_eq!(value["command"], "install-agent");
    assert!(dir.join("knowledge/AGENTS.md").exists());
    assert!(
        value["warnings"]
            .as_array()
            .expect("warnings")
            .iter()
            .any(|warning| warning["code"] == "install_agent_deprecated")
    );
}

#[test]
fn install_skills_skips_non_managed_existing_file() {
    let dir = temp_dir("install-skills-skip");
    let skill_path = dir.join(".github/skills/patina-query/SKILL.md");
    fs::create_dir_all(skill_path.parent().expect("skill parent")).expect("parent should create");
    fs::write(&skill_path, "user-authored skill").expect("skill should write");

    let value = json(&run(
        &dir,
        &["install-skills", "--for", "github-copilot", "--json"],
    ));

    assert_eq!(read_file(&skill_path), "user-authored skill");
    assert!(
        value["data"]["files_skipped"]
            .as_array()
            .expect("skipped")
            .iter()
            .any(|path| path == ".github/skills/patina-query/SKILL.md")
    );
    assert!(
        value["warnings"]
            .as_array()
            .expect("warnings")
            .iter()
            .any(|warning| warning["code"] == "skill_file_exists_not_managed")
    );
}

#[test]
fn install_skills_force_overwrites_non_managed_existing_file() {
    let dir = temp_dir("install-skills-force");
    let skill_path = dir.join(".github/skills/patina-query/SKILL.md");
    fs::create_dir_all(skill_path.parent().expect("skill parent")).expect("parent should create");
    fs::write(&skill_path, "user-authored skill").expect("skill should write");

    let output = run(
        &dir,
        &["install-skills", "--for", "github-copilot", "--force"],
    );

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_skill_file(&skill_path, "patina-query");
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
fn schema_version_one_suggests_reset() {
    let dir = temp_dir("schema-version-one");
    fs::create_dir_all(dir.join(".patina")).expect(".patina should exist");
    let conn =
        rusqlite::Connection::open(dir.join(".patina/index.sqlite")).expect("db should open");
    conn.execute(
        "CREATE TABLE meta (key TEXT PRIMARY KEY, value TEXT NOT NULL)",
        [],
    )
    .expect("meta should create");
    conn.execute(
        "INSERT INTO meta (key, value) VALUES ('schema_version', '1')",
        [],
    )
    .expect("schema version should insert");

    let output = run(&dir, &["query", "term", "--json"]);

    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("patina index --reset"));
}

#[test]
fn fresh_index_has_schema_version_two() {
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

    assert_eq!(version, "2");
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
