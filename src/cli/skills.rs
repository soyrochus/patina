use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use clap::Args;
use serde::Serialize;

use crate::config::PatinaConfig;
use crate::output::{JsonEnvelope, WarningEntry, print_json};

const GENERATED_SKILL_MARKER: &str = "<!-- PATINA GENERATED SKILL -->";
const SHARED_BEGIN: &str = "<!-- BEGIN PATINA AGENT INSTRUCTIONS -->";
const SHARED_END: &str = "<!-- END PATINA AGENT INSTRUCTIONS -->";
const CODEX_BEGIN: &str = "<!-- BEGIN PATINA CODEX CONTEXT -->";
const CODEX_END: &str = "<!-- END PATINA CODEX CONTEXT -->";

#[derive(Debug, Args)]
pub struct InstallSkillsArgs {
    #[arg(long = "for", value_name = "target")]
    pub targets: Vec<String>,
    #[arg(long)]
    pub json: bool,
    #[arg(long)]
    pub force: bool,
}

#[derive(Debug)]
pub struct InstallRequest {
    pub targets: Vec<String>,
    pub json: bool,
    pub force: bool,
    pub command_name: &'static str,
    pub deprecated_install_agent: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum SkillTarget {
    GithubCopilot,
    Codex,
    ClaudeCode,
}

impl SkillTarget {
    fn as_str(self) -> &'static str {
        match self {
            Self::GithubCopilot => "github-copilot",
            Self::Codex => "codex",
            Self::ClaudeCode => "claude-code",
        }
    }

    fn skill_root(self) -> &'static str {
        match self {
            Self::GithubCopilot => ".github/skills",
            Self::Codex => ".agents/skills",
            Self::ClaudeCode => ".claude/skills",
        }
    }
}

#[derive(Debug)]
struct PlannedFile {
    path: PathBuf,
    content: String,
    kind: PlannedFileKind,
}

#[derive(Debug, Clone, Copy)]
enum PlannedFileKind {
    SharedAgents,
    AgentSkill,
    CodexRootAgents,
}

#[derive(Debug, Serialize)]
struct InstallSkillsData {
    files_written: Vec<String>,
    files_skipped: Vec<String>,
    targets: Vec<String>,
}

#[derive(Debug)]
struct ApplyResult {
    files_written: Vec<String>,
    files_skipped: Vec<String>,
    warnings: Vec<WarningEntry>,
}

pub fn run(args: InstallSkillsArgs, config: &PatinaConfig) -> Result<()> {
    run_request(
        InstallRequest {
            targets: args.targets,
            json: args.json,
            force: args.force,
            command_name: "install-skills",
            deprecated_install_agent: false,
        },
        config,
    )
}

pub fn run_request(request: InstallRequest, config: &PatinaConfig) -> Result<()> {
    let targets = normalize_targets(&request.targets)?;
    let mut warnings = Vec::new();

    if request.deprecated_install_agent {
        warnings.push(WarningEntry::new(
            "install_agent_deprecated",
            "install-agent is deprecated; use install-skills instead",
        ));
    }

    if targets.len() > 1 {
        warnings.push(WarningEntry::new(
            "duplicate_skill_names_possible",
            "installing Patina skills for multiple hosts may create duplicate skill names in tools that scan several skill roots",
        ));
    }

    let plans = plan_files(config, &targets);
    let mut result = apply_plan(&plans, request.force)?;
    warnings.append(&mut result.warnings);

    let data = InstallSkillsData {
        files_written: result.files_written.clone(),
        files_skipped: result.files_skipped.clone(),
        targets: targets
            .iter()
            .map(|target| target.as_str().to_string())
            .collect(),
    };

    if request.json {
        print_json(&JsonEnvelope::success(request.command_name, data, warnings))?;
    } else {
        for warning in &warnings {
            eprintln!("warning: {}", warning.message);
        }
        for path in result.files_written {
            println!("wrote {path}");
        }
        for path in result.files_skipped {
            println!("skipped {path}");
        }
    }

    Ok(())
}

fn normalize_targets(values: &[String]) -> Result<Vec<SkillTarget>> {
    let mut targets = BTreeSet::new();

    for value in values {
        match value.as_str() {
            "all" => {
                targets.insert(SkillTarget::GithubCopilot);
                targets.insert(SkillTarget::Codex);
                targets.insert(SkillTarget::ClaudeCode);
            }
            "github-copilot" => {
                targets.insert(SkillTarget::GithubCopilot);
            }
            "codex" => {
                targets.insert(SkillTarget::Codex);
            }
            "claude-code" => {
                targets.insert(SkillTarget::ClaudeCode);
            }
            unknown => bail!(
                "unrecognised skill target `{unknown}`; supported targets: github-copilot, codex, claude-code, all"
            ),
        };
    }

    Ok(targets.into_iter().collect())
}

fn plan_files(config: &PatinaConfig, targets: &[SkillTarget]) -> Vec<PlannedFile> {
    let knowledge_dir = config.knowledge_dir();
    let knowledge_dir_text = knowledge_dir.display().to_string();
    let mut plans = vec![PlannedFile {
        path: knowledge_dir.join("AGENTS.md"),
        content: shared_agents_content(),
        kind: PlannedFileKind::SharedAgents,
    }];

    for target in targets {
        for skill in [Skill::Query, Skill::Check] {
            plans.push(PlannedFile {
                path: PathBuf::from(target.skill_root())
                    .join(skill.name())
                    .join("SKILL.md"),
                content: skill.content(&knowledge_dir_text),
                kind: PlannedFileKind::AgentSkill,
            });
        }

        if *target == SkillTarget::Codex {
            plans.push(PlannedFile {
                path: PathBuf::from("AGENTS.md"),
                content: codex_agents_content(&knowledge_dir_text),
                kind: PlannedFileKind::CodexRootAgents,
            });
        }
    }

    plans
}

fn apply_plan(plans: &[PlannedFile], force: bool) -> Result<ApplyResult> {
    let mut result = ApplyResult {
        files_written: Vec::new(),
        files_skipped: Vec::new(),
        warnings: Vec::new(),
    };

    for plan in plans {
        match plan.kind {
            PlannedFileKind::SharedAgents => {
                if apply_shared_agents(plan, force)? {
                    result.files_written.push(display_path(&plan.path));
                }
            }
            PlannedFileKind::AgentSkill => {
                apply_skill_file(plan, force, &mut result)?;
            }
            PlannedFileKind::CodexRootAgents => {
                if apply_section_file(&plan.path, &plan.content, CODEX_BEGIN, CODEX_END, true)? {
                    result.files_written.push(display_path(&plan.path));
                }
            }
        }
    }

    Ok(result)
}

fn apply_shared_agents(plan: &PlannedFile, force: bool) -> Result<bool> {
    if !plan.path.exists() {
        write_file(
            &plan.path,
            &format!("# Agent Instructions\n\n{}", plan.content),
        )?;
        return Ok(true);
    }

    let existing = fs::read_to_string(&plan.path)
        .with_context(|| format!("failed to read {}", plan.path.display()))?;
    if contains_section(&existing, SHARED_BEGIN, SHARED_END) {
        let updated = replace_section(&existing, SHARED_BEGIN, SHARED_END, &plan.content);
        write_file_if_changed(&plan.path, &existing, &updated)
    } else if force {
        let updated = append_section(&existing, &plan.content);
        write_file_if_changed(&plan.path, &existing, &updated)
    } else {
        Ok(false)
    }
}

fn apply_skill_file(plan: &PlannedFile, force: bool, result: &mut ApplyResult) -> Result<()> {
    if !plan.path.exists() {
        write_file(&plan.path, &plan.content)?;
        result.files_written.push(display_path(&plan.path));
        return Ok(());
    }

    let existing = fs::read_to_string(&plan.path)
        .with_context(|| format!("failed to read {}", plan.path.display()))?;
    if force || existing.contains(GENERATED_SKILL_MARKER) {
        if write_file_if_changed(&plan.path, &existing, &plan.content)? {
            result.files_written.push(display_path(&plan.path));
        }
    } else {
        result.files_skipped.push(display_path(&plan.path));
        result.warnings.push(
            WarningEntry::new(
                "skill_file_exists_not_managed",
                "Skill file exists and is not Patina-managed; skipped. Use --force to overwrite.",
            )
            .with_path(&plan.path),
        );
    }

    Ok(())
}

fn apply_section_file(
    path: &Path,
    section: &str,
    begin: &str,
    end: &str,
    append_without_marker: bool,
) -> Result<bool> {
    if !path.exists() {
        write_file(path, section)?;
        return Ok(true);
    }

    let existing =
        fs::read_to_string(path).with_context(|| format!("failed to read {}", path.display()))?;
    let updated = if contains_section(&existing, begin, end) {
        replace_section(&existing, begin, end, section)
    } else if append_without_marker {
        append_section(&existing, section)
    } else {
        existing.clone()
    };
    write_file_if_changed(path, &existing, &updated)
}

fn contains_section(contents: &str, begin: &str, end: &str) -> bool {
    contents.contains(begin) && contents.contains(end)
}

fn replace_section(contents: &str, begin: &str, end: &str, replacement: &str) -> String {
    let Some(start) = contents.find(begin) else {
        return append_section(contents, replacement);
    };
    let Some(end_relative) = contents[start..].find(end) else {
        return append_section(contents, replacement);
    };
    let end_index = start + end_relative + end.len();
    format!(
        "{}{}{}",
        &contents[..start],
        replacement.trim_end(),
        &contents[end_index..]
    )
}

fn append_section(contents: &str, section: &str) -> String {
    let mut updated = contents.trim_end().to_string();
    if !updated.is_empty() {
        updated.push_str("\n\n");
    }
    updated.push_str(section.trim_end());
    updated.push('\n');
    updated
}

fn write_file(path: &Path, contents: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }
    fs::write(path, contents).with_context(|| format!("failed to write {}", path.display()))
}

fn write_file_if_changed(path: &Path, existing: &str, updated: &str) -> Result<bool> {
    if existing == updated {
        return Ok(false);
    }
    write_file(path, updated)?;
    Ok(true)
}

fn display_path(path: &Path) -> String {
    path.display().to_string()
}

#[derive(Debug, Clone, Copy)]
enum Skill {
    Query,
    Check,
}

impl Skill {
    fn name(self) -> &'static str {
        match self {
            Self::Query => "patina-query",
            Self::Check => "patina-check",
        }
    }

    fn content(self, knowledge_dir: &str) -> String {
        match self {
            Self::Query => patina_query_skill(knowledge_dir),
            Self::Check => patina_check_skill(knowledge_dir),
        }
    }
}

fn shared_agents_content() -> String {
    format!(
        r#"{SHARED_BEGIN}

## Patina Knowledge Base

This knowledge base is managed with Patina.

Patina is a local-first, Git-compatible Markdown knowledge tool. The Git-tracked knowledge directory is the source of truth. The `.patina/` directory is generated local state and must not be cited or committed.

### Finding knowledge

Run:

```bash
patina query "<terms>" --json --limit 5
```

Read the highest-scoring results with:

```bash
patina read <path> --json
```

Base answers on Git-tracked Markdown files, not on generated index data.

### Checking knowledge health

Run:

```bash
patina lint --json
patina stale --json
```

Report errors and warnings clearly. Do not edit knowledge files while lint errors are present unless the task is explicitly to fix those errors.

### Before adding or updating pages

1. Run `patina lint --json` to confirm the current state.
2. Search first with `patina query "<terms>" --json --limit 5`.
3. Prefer updating existing pages over adding overlapping new ones.
4. Preserve front matter, links, and `source_refs`.
5. After edits, run `patina lint --json`.
6. Run `patina index` after significant changes.

### Page conventions

Required front matter:

```yaml
title: "Page Title"
type: concept
status: active
```

Declare source files in `source_refs` so Patina can detect stale synthesis.

Use small, reviewable changes.

Do not rewrite broad areas of the knowledge base unless explicitly requested.

{SHARED_END}
"#
    )
}

fn patina_query_skill(knowledge_dir: &str) -> String {
    format!(
        r#"---
name: patina-query
description: Search the Patina knowledge base and read the most relevant pages. Use when answering questions about project context, architecture, decisions, domain knowledge, prior repository knowledge, or anything that may already be documented under the knowledge directory.
license: MIT
compatibility: Requires the patina CLI on PATH and a Patina knowledge directory in this repository.
---

{GENERATED_SKILL_MARKER}

# Patina Query

Use this skill when the user asks about project knowledge, architecture, decisions, domain concepts, previous notes, repository context, or anything likely to be documented in the Patina knowledge base.

The shared Patina operating instructions are in:

```text
{knowledge_dir}/AGENTS.md
```

## Workflow

1. Convert the user's request into a concise search query.

2. Run:

   ```bash
   patina query "<terms>" --json --limit 5
   ```

3. Inspect the JSON response.

   - If `ok` is `false`, report the errors.
   - If no results are returned, try one broader query.
   - If results are returned, read the highest-scoring pages.

4. For each relevant result, run:

   ```bash
   patina read <path> --json
   ```

5. Answer from the Git-tracked Markdown content returned by `patina read`.

6. Cite repository-relative page paths in the answer so the user can inspect the source.

## Rules

- Do not answer from `.patina/` generated index files.
- Do not cite generated local cache data.
- Prefer reading actual Markdown pages before answering.
- Use `patina query "<terms>" --json --limit 5 --explain` if ranking looks unexpected and the installed Patina version supports `--explain`.
- If the knowledge base does not contain the answer, say so clearly.
"#
    )
}

fn patina_check_skill(knowledge_dir: &str) -> String {
    format!(
        r#"---
name: patina-check
description: Validate the Patina knowledge base before or after editing. Use when asked to audit knowledge health, check stale pages, validate metadata, or before changing files under the knowledge directory.
license: MIT
compatibility: Requires the patina CLI on PATH and a Patina knowledge directory in this repository.
---

{GENERATED_SKILL_MARKER}

# Patina Check

Use this skill before editing Patina knowledge files, after editing them, or when asked to audit the health of the knowledge base.

The shared Patina operating instructions are in:

```text
{knowledge_dir}/AGENTS.md
```

## Workflow

1. Run:

   ```bash
   patina lint --json
   ```

2. Inspect the JSON response.

   - If `ok` is `false`, report each error with its `code`, `message`, and `path`.
   - Report warnings separately.
   - Do not proceed with unrelated knowledge edits while lint errors are present.

3. Run:

   ```bash
   patina stale --json
   ```

4. Inspect `data.stale_pages`.

   For each stale page, report:

   - page path;
   - reason code;
   - severity;
   - related source path if present.

5. If knowledge files were changed during the task, run:

   ```bash
   patina lint --json
   patina index
   ```

6. Summarise the result as one of:

   - clean;
   - warnings only;
   - errors found;
   - stale pages require review.

## Rules

- Do not ignore lint errors.
- Do not treat stale pages as necessarily wrong; they require review.
- Do not rewrite pages unless the user explicitly asks for fixes.
- Keep changes small and reviewable.
"#
    )
}

fn codex_agents_content(knowledge_dir: &str) -> String {
    format!(
        r#"{CODEX_BEGIN}

## Patina Knowledge Base

This repository uses Patina for local-first Markdown knowledge management.

Real Patina Agent Skills are installed under:

```text
.agents/skills/
```

Use the `patina-query` skill when answering questions about project context, architecture, decisions, domain knowledge, or prior repository knowledge.

Use the `patina-check` skill when validating or editing knowledge files.

Shared operating instructions:

```text
{knowledge_dir}/AGENTS.md
```

Core commands:

```bash
patina query "<terms>" --json --limit 5
patina read <path> --json
patina lint --json
patina stale --json
```

{CODEX_END}
"#
    )
}
