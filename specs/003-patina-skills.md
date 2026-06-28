# Patina — Agent Skill Integration

## 1. Status

Pending implementation.

This specification replaces the previous `install-agent` implementation and corrects the earlier draft of this skill integration spec.

The earlier draft treated GitHub Copilot prompt files and Codex root `AGENTS.md` files as the primary integration mechanism. That is no longer correct for the intended implementation. GitHub Copilot and OpenAI Codex both support Agent Skills using the `SKILL.md` directory format. Prompt files and `AGENTS.md` remain useful as supplementary context, but they are not the primary skill mechanism.

This spec supersedes the `install-agent` command defined in `specs/001-base-implementation.md`.

## 2. External references

The implementation is based on the current Agent Skills model used by modern AI coding tools:

- OpenAI Codex Agent Skills: `https://developers.openai.com/codex/skills`
- GitHub Copilot Agent Skills: `https://docs.github.com/en/copilot/concepts/agents/about-agent-skills`
- GitHub Copilot skill authoring and installation: `https://docs.github.com/en/copilot/how-tos/copilot-on-github/customize-copilot/customize-cloud-agent/add-skills`
- Agent Skills open specification: `https://agentskills.io/specification`

These references are documentation context for the implementation. The generated skill files must be self-contained and must not require the coding agent to browse the web.

## 3. Problem

The current Patina integration writes a short instruction paragraph to a passive context file, such as `.claude/CLAUDE.md`.

That is insufficient.

Passive context is loaded at session start, competes with all other context, and does not create a focused, reusable workflow. It also does not give the coding agent a clearly scoped procedural capability for common Patina operations such as searching the knowledge base, validating knowledge health, or refreshing the local index.

Patina needs to install real Agent Skills where the host supports them.

A Patina skill must be:

- discoverable by the host agent;
- scoped to a specific repeatable task;
- described clearly enough for implicit invocation;
- usable through explicit skill selection where the host supports that;
- thin enough not to duplicate the full knowledge-base policy;
- grounded in the shared `knowledge/AGENTS.md` instructions;
- safe by default, with no pre-approved shell execution unless explicitly configured.

## 4. Design correction

The implementation must distinguish between three different integration mechanisms.

### 4.1 Agent Skills

Agent Skills are the primary integration mechanism.

A skill is a directory containing a `SKILL.md` file with YAML front matter and Markdown instructions.

Typical structure:

```text
<skills-root>/
  patina-query/
    SKILL.md
  patina-check/
    SKILL.md
```

The `SKILL.md` file must contain, at minimum:

```yaml
---
name: patina-query
description: Search the Patina knowledge base and read the most relevant pages. Use when answering questions about project context, architecture, decisions, domain knowledge, or prior repository knowledge.
---
```

The skill body contains the concrete operating steps.

### 4.2 Prompt files

GitHub Copilot prompt files under `.github/prompts/*.prompt.md` are not the default Patina integration in this spec.

They may be added later as optional convenience prompts, but they must not replace Agent Skills.

The previous `.github/prompts/patina-query.prompt.md` and `.github/prompts/patina-check.prompt.md` targets are removed from the default implementation.

### 4.3 Passive context files

`AGENTS.md`, `CLAUDE.md`, and similar files are passive context.

Patina shall still maintain `knowledge/AGENTS.md` as the single source of truth for Patina operating instructions.

For Codex, a root `AGENTS.md` section may also be written as supplementary passive context, because Codex uses repository `AGENTS.md` files. This is not a substitute for Codex skills.

## 5. Command rename

`install-agent` is renamed to `install-skills`.

The old command shall be removed as NO deprecated alias is necesarry.



## 6. Command interface

```bash
patina install-skills [--for <target>]... [--force] [--json]
```

`--for` accepts:

```text
claude-code
github-copilot
codex
all
```

Multiple `--for` flags are accepted.

Examples:

```bash
patina install-skills
patina install-skills --for github-copilot
patina install-skills --for codex
patina install-skills --for claude-code
patina install-skills --for github-copilot --for codex
patina install-skills --for all
patina install-skills --for github-copilot --force
patina install-skills --for codex --json
```

If `--for` is omitted, Patina writes or updates only the shared instruction file:

```text
<knowledge_dir>/AGENTS.md
```

No host-specific skill directories are written unless `--for` is provided.

`--for all` is equivalent to:

```bash
patina install-skills --for github-copilot --for codex --for claude-code
```

## 7. Shared operating instructions: `<knowledge_dir>/AGENTS.md`

Every `install-skills` invocation shall ensure that `<knowledge_dir>/AGENTS.md` exists.

This file is the shared operating instruction reference for humans and agents.

Skill files shall point to this file for full Patina conventions.

The path `<knowledge_dir>` resolves to the configured knowledge directory in `patina.toml`. If no config exists, the default is `knowledge`.

### 7.1 Write policy

If `<knowledge_dir>/AGENTS.md` does not exist, create it.

If it exists and `--force` is not provided, do not overwrite it.

If it exists and `--force` is provided, replace only the Patina-managed section if present. If no Patina-managed section exists, append the section.

Use managed markers:

```markdown
<!-- BEGIN PATINA AGENT INSTRUCTIONS -->
...
<!-- END PATINA AGENT INSTRUCTIONS -->
```

### 7.2 Default content

```markdown
# Agent Instructions

<!-- BEGIN PATINA AGENT INSTRUCTIONS -->

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

<!-- END PATINA AGENT INSTRUCTIONS -->
```

## 8. Skill names

Patina shall install two initial skills:

```text
patina-query
patina-check
```

The names are lowercase and hyphenated to satisfy the Agent Skills naming convention.

The same names shall be used across all supported hosts.

### 8.1 `patina-query`

Purpose:

```text
Search the Patina knowledge base and read the most relevant pages before answering questions about project context, architecture, decisions, domain knowledge, or prior repository knowledge.
```

Primary commands used:

```bash
patina query "<terms>" --json --limit 5
patina read <path> --json
```

### 8.2 `patina-check`

Purpose:

```text
Validate the Patina knowledge base before or after editing. Run lint and stale checks, report errors and warnings, and stop unsafe edits when the knowledge base is not clean.
```

Primary commands used:

```bash
patina lint --json
patina stale --json
patina index
```

## 9. Host-specific skill locations

Patina shall write `SKILL.md` files into host-specific project skill directories.

### 9.1 GitHub Copilot

Installed by:

```bash
patina install-skills --for github-copilot
```

Default project skill locations:

```text
.github/skills/patina-query/SKILL.md
.github/skills/patina-check/SKILL.md
```

GitHub Copilot also recognises `.agents/skills` and `.claude/skills`, but Patina shall use `.github/skills` as the default Copilot-specific project directory.

### 9.2 OpenAI Codex

Installed by:

```bash
patina install-skills --for codex
```

Default project skill locations:

```text
.agents/skills/patina-query/SKILL.md
.agents/skills/patina-check/SKILL.md
```

Codex scans `.agents/skills` in the working directory and parent directories up to the repository root. Patina shall install repository-level Codex skills under the repository root.

For Codex, Patina shall also create or update the repository root `AGENTS.md` with a short Patina section. This is supplementary passive context, not the skill implementation.

### 9.3 Claude Code

Installed by:

```bash
patina install-skills --for claude-code
```

Default project skill locations:

```text
.claude/skills/patina-query/SKILL.md
.claude/skills/patina-check/SKILL.md
```

Claude Code support is implemented as project-local Agent Skill files.

Do not describe these as ordinary slash commands in the spec. Agent hosts differ in how they expose explicit invocation. The stable contract is the `SKILL.md` directory and its `name` and `description` metadata.

## 10. Duplicate skill handling

Some agent hosts may scan more than one project skill directory. For example, a host may recognise `.github/skills`, `.agents/skills`, and `.claude/skills`.

If a repository contains multiple generated Patina skill copies with the same `name`, a host may show duplicates.

Patina shall therefore warn when installing multiple host targets that may be visible to the same agent.

Example warning:

```text
warning: installing Patina skills for multiple hosts may create duplicate skill names in tools that scan several skill roots
```

This warning is informational. The command shall still complete.

The generated skill content shall be identical in all host-specific directories except for optional host notes.

## 11. Skill write policy

For each generated `SKILL.md`:

1. Create the directory if it does not exist.
2. If `SKILL.md` does not exist, create it.
3. If `SKILL.md` exists and contains the marker `<!-- PATINA GENERATED SKILL -->`, replace the entire file.
4. If `SKILL.md` exists and does not contain the marker:
   - skip it by default;
   - emit a warning;
   - overwrite only if `--force` is provided.
5. With `--force`, overwrite the file.

This avoids fragile detection based on whether a file happens to contain `patina query`.

All generated skills shall contain this marker near the top of the Markdown body:

```markdown
<!-- PATINA GENERATED SKILL -->
```

## 12. Security policy for generated skills

Generated Patina skills shall not use `allowed-tools` by default.

Reason: pre-approving shell or bash execution can remove host confirmation steps. Patina commands are local CLI commands and may read or write repository files. The agent host should ask for permission according to its normal policy.

Patina may later support:

```bash
patina install-skills --allow-shell
```

That is out of scope for this spec.

The v0.1 generated skill front matter shall omit `allowed-tools`.

## 13. `patina-query` SKILL.md template

The same template shall be used for GitHub Copilot, Codex, and Claude Code.

The `<knowledge_dir>` placeholder shall be replaced at write time with the configured knowledge directory.

```markdown
---
name: patina-query
description: Search the Patina knowledge base and read the most relevant pages. Use when answering questions about project context, architecture, decisions, domain knowledge, prior repository knowledge, or anything that may already be documented under the knowledge directory.
license: MIT
compatibility: Requires the patina CLI on PATH and a Patina knowledge directory in this repository.
---

<!-- PATINA GENERATED SKILL -->

# Patina Query

Use this skill when the user asks about project knowledge, architecture, decisions, domain concepts, previous notes, repository context, or anything likely to be documented in the Patina knowledge base.

The shared Patina operating instructions are in:

```text
<knowledge_dir>/AGENTS.md
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
```

## 14. `patina-check` SKILL.md template

The same template shall be used for GitHub Copilot, Codex, and Claude Code.

The `<knowledge_dir>` placeholder shall be replaced at write time with the configured knowledge directory.

```markdown
---
name: patina-check
description: Validate the Patina knowledge base before or after editing. Use when asked to audit knowledge health, check stale pages, validate metadata, or before changing files under the knowledge directory.
license: MIT
compatibility: Requires the patina CLI on PATH and a Patina knowledge directory in this repository.
---

<!-- PATINA GENERATED SKILL -->

# Patina Check

Use this skill before editing Patina knowledge files, after editing them, or when asked to audit the health of the knowledge base.

The shared Patina operating instructions are in:

```text
<knowledge_dir>/AGENTS.md
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
```

## 15. Codex root `AGENTS.md` supplement

When installing with:

```bash
patina install-skills --for codex
```

Patina shall also create or update `AGENTS.md` at the repository root.

This file is passive context for Codex. It is not the Codex skill implementation.

### 15.1 Write policy

If root `AGENTS.md` does not exist, create it with the Patina section.

If it exists and does not contain the Patina managed section, append the Patina section.

If it contains the Patina managed section, replace that section only.

Use markers:

```markdown
<!-- BEGIN PATINA CODEX CONTEXT -->
...
<!-- END PATINA CODEX CONTEXT -->
```

### 15.2 Content

```markdown
<!-- BEGIN PATINA CODEX CONTEXT -->

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
<knowledge_dir>/AGENTS.md
```

Core commands:

```bash
patina query "<terms>" --json --limit 5
patina read <path> --json
patina lint --json
patina stale --json
```

<!-- END PATINA CODEX CONTEXT -->
```

## 16. Optional Copilot prompt files

Copilot prompt files are explicitly out of scope for the default implementation.

Do not write:

```text
.github/prompts/patina-query.prompt.md
.github/prompts/patina-check.prompt.md
```

unless a future explicit option is added, for example:

```bash
patina install-skills --for github-copilot --include-prompts
```

This future option is not part of the v0.1 acceptance criteria.

## 17. JSON output

`patina install-skills --json` shall return the standard Patina JSON envelope.

Example:

```json
{
  "version": "0.1",
  "command": "install-skills",
  "ok": true,
  "data": {
    "files_written": [
      "knowledge/AGENTS.md",
      ".github/skills/patina-query/SKILL.md",
      ".github/skills/patina-check/SKILL.md"
    ],
    "files_skipped": [],
    "targets": ["github-copilot"]
  },
  "warnings": [],
  "errors": []
}
```

If files are skipped because they already exist and are not Patina-managed:

```json
{
  "version": "0.1",
  "command": "install-skills",
  "ok": true,
  "data": {
    "files_written": ["knowledge/AGENTS.md"],
    "files_skipped": [
      ".github/skills/patina-query/SKILL.md"
    ],
    "targets": ["github-copilot"]
  },
  "warnings": [
    {
      "code": "skill_file_exists_not_managed",
      "message": "Skill file exists and is not Patina-managed; skipped. Use --force to overwrite.",
      "path": ".github/skills/patina-query/SKILL.md"
    }
  ],
  "errors": []
}
```

## 18. Files to change

| File | Change |
| ---- | ------ |
| `src/cli/mod.rs` | Add `InstallSkills` subcommand; keep `InstallAgent` as deprecated alias. `--for` must support multiple values and `all`. |
| `src/cli/agent.rs` | Replace with or delegate to `src/cli/skills.rs`. Existing passive-context logic should be migrated into the new skill installer. |
| `src/cli/skills.rs` | New implementation module. Contains target resolution, template constants, placeholder substitution, write policy, JSON reporting, and deprecation handling. |
| `src/main.rs` | Wire `InstallSkills` and deprecated `InstallAgent` to the same handler. |
| `Cargo.toml` | No new dependencies required unless existing CLI argument parsing cannot support repeated `--for`. |

## 19. Suggested Rust implementation structure

Create a small target model:

```rust
enum SkillTarget {
    GithubCopilot,
    Codex,
    ClaudeCode,
}
```

Create a file plan model:

```rust
struct PlannedSkillFile {
    path: PathBuf,
    content: String,
    kind: SkillFileKind,
}

enum SkillFileKind {
    SharedAgents,
    AgentSkill,
    CodexRootAgents,
}
```

The handler shall:

1. load configuration;
2. resolve `<knowledge_dir>`;
3. normalize targets;
4. always plan `<knowledge_dir>/AGENTS.md`;
5. add host-specific skill files;
6. add root `AGENTS.md` only for Codex;
7. apply write policy;
8. collect `files_written`, `files_skipped`, warnings, and errors;
9. print text or JSON output.

## 20. Acceptance criteria

### 20.1 Shared behaviour

- `patina install-skills` with no flags writes `<knowledge_dir>/AGENTS.md` only.
- `patina install-skills --for all` installs GitHub Copilot, Codex, and Claude Code targets.
- `--for` can be specified multiple times.
- `<knowledge_dir>` is correctly substituted in all generated files.
- Generated `SKILL.md` files include valid YAML front matter with `name` and `description`.
- Generated `SKILL.md` files include the marker `<!-- PATINA GENERATED SKILL -->`.
- Generated skills do not include `allowed-tools` by default.
- A second run without `--force` replaces Patina-managed generated files and skips non-managed files.
- `--force` overwrites existing target files.
- `patina install-agent` behaves identically to `install-skills` but emits a deprecation warning.
- `patina install-skills --json` returns the standard envelope.

### 20.2 GitHub Copilot

The command:

```bash
patina install-skills --for github-copilot
```

writes:

```text
<knowledge_dir>/AGENTS.md
.github/skills/patina-query/SKILL.md
.github/skills/patina-check/SKILL.md
```

It does not write `.github/prompts/*.prompt.md`.

### 20.3 OpenAI Codex

The command:

```bash
patina install-skills --for codex
```

writes:

```text
<knowledge_dir>/AGENTS.md
.agents/skills/patina-query/SKILL.md
.agents/skills/patina-check/SKILL.md
AGENTS.md
```

The root `AGENTS.md` contains only a short Patina passive-context section and points to the real Codex skills under `.agents/skills/`.

### 20.4 Claude Code

The command:

```bash
patina install-skills --for claude-code
```

writes:

```text
<knowledge_dir>/AGENTS.md
.claude/skills/patina-query/SKILL.md
.claude/skills/patina-check/SKILL.md
```

The spec shall not assert a fixed slash-command syntax for Claude Code. The generated files are Agent Skill files and rely on host discovery through their `name` and `description`.

## 21. Test cases

Add fixture-based tests for:

```text
install_skills_no_targets_writes_shared_agents_only
install_skills_github_copilot_writes_github_skills
install_skills_codex_writes_agents_skills_and_root_agents
install_skills_claude_code_writes_claude_skills
install_skills_all_writes_all_targets
install_skills_repeated_for_flags
install_skills_substitutes_custom_knowledge_dir
install_skills_json_reports_written_files
install_agent_alias_emits_deprecation_warning
install_skills_skips_non_managed_existing_file
install_skills_force_overwrites_non_managed_existing_file
```

Each test shall verify file paths, front matter, managed markers, and command output.

## 22. Summary

The correct implementation is:

```text
GitHub Copilot  -> .github/skills/<skill>/SKILL.md
OpenAI Codex    -> .agents/skills/<skill>/SKILL.md
Claude Code     -> .claude/skills/<skill>/SKILL.md
Shared policy   -> <knowledge_dir>/AGENTS.md
Codex context   -> root AGENTS.md, supplementary only
```

The incorrect implementation is:

```text
GitHub Copilot  -> only .github/prompts/*.prompt.md
OpenAI Codex    -> only root AGENTS.md
Claude Code     -> only .claude/CLAUDE.md
```

Patina must install actual `SKILL.md`-based Agent Skills, not merely passive context or prompt files.
