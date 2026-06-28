## Context

The existing `install-agent` command writes passive context files, primarily `knowledge/AGENTS.md` and a Claude-specific snippet. The new Patina skill integration must install actual Agent Skills where supported: project-local directories containing `SKILL.md` files with YAML front matter and task-specific instructions.

The source spec has one internal conflict: the command rename section says no deprecated alias is necessary, while the later file-change list, acceptance criteria, and test list require `install-agent` to behave as a deprecated alias. This design follows the later acceptance criteria because they are more specific and testable.

## Goals / Non-Goals

**Goals:**

- Add `patina install-skills [--for <target>]... [--force] [--json]`.
- Generate `patina-query` and `patina-check` Agent Skill files for GitHub Copilot, Codex, and Claude Code.
- Always create or update shared Patina operating instructions at `<knowledge_dir>/AGENTS.md`.
- Add supplementary root `AGENTS.md` context when installing Codex skills.
- Implement predictable write policies for Patina-managed files, non-managed files, and `--force`.
- Keep generated skills safe by omitting `allowed-tools` by default.
- Keep `install-agent` as a deprecated alias to `install-skills`.

**Non-Goals:**

- No GitHub Copilot prompt files by default.
- No `--allow-shell` or generated `allowed-tools` support.
- No host-specific slash-command behavior.
- No new dependency unless the existing CLI parser cannot support repeated `--for` values.

## Decisions

1. Implement a new `src/cli/skills.rs` module and delegate `install-agent` to it.

   This keeps the skill installer cohesive: target normalization, file planning, templates, write policy, warnings, and JSON output belong together. The existing `src/cli/agent.rs` can either delegate to the new module or be replaced once dispatch is updated.

2. Model targets as `github-copilot`, `codex`, and `claude-code`, with `all` expanding to all three.

   Multiple `--for` flags should be accepted and de-duplicated. If `--for` is omitted, only the shared `<knowledge_dir>/AGENTS.md` file is planned.

3. Use generated-file markers for ownership.

   `SKILL.md` files use `<!-- PATINA GENERATED SKILL -->`. Shared `knowledge/AGENTS.md` uses `<!-- BEGIN PATINA AGENT INSTRUCTIONS -->` and `<!-- END PATINA AGENT INSTRUCTIONS -->`. Root Codex `AGENTS.md` uses `<!-- BEGIN PATINA CODEX CONTEXT -->` and `<!-- END PATINA CODEX CONTEXT -->`. Marker-based ownership avoids overwriting unrelated user-authored files.

4. Plan files first, then apply write policy.

   A small plan model should include path, content, and file kind. Applying the plan should collect `files_written`, `files_skipped`, warnings, and errors consistently for text and JSON output.

5. Treat passive context as supplementary.

   `knowledge/AGENTS.md` remains the shared operating policy. Root `AGENTS.md` is written only for Codex and only points to real skills under `.agents/skills/`.

## Risks / Trade-offs

- Existing non-managed skill files may be user-authored -> Skip by default, warn, and require `--force` to overwrite.
- Multiple host directories can create duplicate visible skill names -> Warn when installing more than one host target while still completing the command.
- Replacing `install-agent` behavior can surprise users -> Keep a deprecated alias and warning while routing to the new implementation.
- Generated skill content can drift across hosts -> Use one shared template for each skill and only substitute `<knowledge_dir>`.

## Migration Plan

1. Add CLI parsing for `install-skills`, repeated `--for`, `--force`, and `--json`.
2. Add target normalization and file planning in `src/cli/skills.rs`.
3. Move shared `knowledge/AGENTS.md` content into a managed-section writer.
4. Add `SKILL.md` templates for `patina-query` and `patina-check`.
5. Add Codex root `AGENTS.md` managed-section support.
6. Route `install-agent` through the new handler with a deprecation warning.
7. Replace or update existing install-agent tests with install-skills coverage.
