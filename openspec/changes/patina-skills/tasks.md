## 1. CLI Surface

- [ ] 1.1 Add an `install-skills` subcommand with `--for <target>` repeat support, `--force`, and `--json`
- [ ] 1.2 Define accepted target values: `github-copilot`, `codex`, `claude-code`, and `all`
- [ ] 1.3 Route `install-skills` from `src/main.rs` and command metadata so JSON envelopes use command `install-skills`
- [ ] 1.4 Preserve `install-agent` as a deprecated compatibility alias routed to the new skills handler

## 2. Skills Installer Core

- [ ] 2.1 Create or replace a skills installer module with target normalization and `all` expansion
- [ ] 2.2 Add file planning models for shared instructions, host skill files, and Codex root context
- [ ] 2.3 Always plan `<knowledge_dir>/AGENTS.md`, resolving custom knowledge directories from config
- [ ] 2.4 Plan GitHub Copilot skill files under `.github/skills/<skill>/SKILL.md`
- [ ] 2.5 Plan Codex skill files under `.agents/skills/<skill>/SKILL.md` plus root `AGENTS.md`
- [ ] 2.6 Plan Claude Code skill files under `.claude/skills/<skill>/SKILL.md`
- [ ] 2.7 Emit an informational duplicate-skill warning when multiple host targets are selected

## 3. Generated Content

- [ ] 3.1 Implement the managed shared `<knowledge_dir>/AGENTS.md` Patina section with begin/end markers
- [ ] 3.2 Implement the `patina-query` `SKILL.md` template with required front matter, marker, workflow, and rules
- [ ] 3.3 Implement the `patina-check` `SKILL.md` template with required front matter, marker, workflow, and rules
- [ ] 3.4 Substitute the configured `<knowledge_dir>` path in every generated file
- [ ] 3.5 Ensure generated skill front matter omits `allowed-tools`
- [ ] 3.6 Implement the Codex root `AGENTS.md` managed section with begin/end markers
- [ ] 3.7 Ensure the default GitHub Copilot target does not generate `.github/prompts/*.prompt.md`

## 4. Write Policy And Output

- [ ] 4.1 Create parent directories for planned files
- [ ] 4.2 Create missing files and replace Patina-managed files
- [ ] 4.3 Skip non-managed existing `SKILL.md` files by default and emit `skill_file_exists_not_managed`
- [ ] 4.4 Make `--force` overwrite non-managed target files
- [ ] 4.5 Track `files_written`, `files_skipped`, normalized `targets`, warnings, and errors
- [ ] 4.6 Implement standard JSON envelope output for `install-skills --json`
- [ ] 4.7 Implement human-readable output for written and skipped files

## 5. Deprecated Install-Agent Compatibility

- [ ] 5.1 Map `install-agent` with no target to shared `install-skills` behavior and emit a deprecation warning
- [ ] 5.2 Map compatible old `--agent claude-code` usage to `--for claude-code`
- [ ] 5.3 Return an error for unknown old `--agent` values listing supported install-skills targets
- [ ] 5.4 Include the deprecation warning in `install-agent --json` output

## 6. Tests And Verification

- [ ] 6.1 Add test: `install_skills_no_targets_writes_shared_agents_only`
- [ ] 6.2 Add test: `install_skills_github_copilot_writes_github_skills`
- [ ] 6.3 Add test: `install_skills_codex_writes_agents_skills_and_root_agents`
- [ ] 6.4 Add test: `install_skills_claude_code_writes_claude_skills`
- [ ] 6.5 Add test: `install_skills_all_writes_all_targets`
- [ ] 6.6 Add test: `install_skills_repeated_for_flags`
- [ ] 6.7 Add test: `install_skills_substitutes_custom_knowledge_dir`
- [ ] 6.8 Add test: `install_skills_json_reports_written_files`
- [ ] 6.9 Add test: `install_agent_alias_emits_deprecation_warning`
- [ ] 6.10 Add test: `install_skills_skips_non_managed_existing_file`
- [ ] 6.11 Add test: `install_skills_force_overwrites_non_managed_existing_file`
- [ ] 6.12 Run `cargo fmt`
- [ ] 6.13 Run `cargo check`
- [ ] 6.14 Run `cargo test`
