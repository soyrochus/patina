## Why

Patina currently installs passive agent instruction text, but modern coding tools support task-scoped Agent Skills through `SKILL.md` directories. Patina should install reusable Patina skills for querying and checking the knowledge base, while keeping shared instructions in `knowledge/AGENTS.md`.

## What Changes

- Add `patina install-skills [--for <target>]... [--force] [--json]`.
- Install two generated Agent Skills, `patina-query` and `patina-check`, for GitHub Copilot, OpenAI Codex, and Claude Code.
- Always maintain shared Patina operating instructions at `<knowledge_dir>/AGENTS.md`.
- For Codex, also maintain a short root `AGENTS.md` passive-context section that points to `.agents/skills/`.
- Implement deterministic write policies for Patina-managed files, non-managed files, and `--force`.
- Emit warnings for skipped non-managed skill files and for multi-host installs that may create duplicate visible skill names.
- Return the standard JSON envelope with `files_written`, `files_skipped`, and `targets`.
- Change `patina install-agent` into a deprecated compatibility alias for `install-skills` that emits a deprecation warning.
- Do not generate GitHub Copilot prompt files by default.
- Do not include `allowed-tools` in generated skill front matter by default.

## Capabilities

### New Capabilities

- `install-skills-command`: `patina install-skills` writes shared Patina instructions and host-specific `SKILL.md` Agent Skill files.

### Modified Capabilities

- `install-agent-command`: replace the previous passive-context installer behavior with a deprecated alias to `install-skills`.

## Impact

- Affected code: CLI command definitions and dispatch, current agent installer implementation, a new or replacement skills installer module, and command tests.
- Affected generated files: `<knowledge_dir>/AGENTS.md`, `.github/skills/*/SKILL.md`, `.agents/skills/*/SKILL.md`, `.claude/skills/*/SKILL.md`, and root `AGENTS.md` for Codex.
- No new dependencies are expected.
- Existing `install-agent --agent <name>` behavior is superseded by `install-skills --for <target>`.
