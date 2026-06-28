# Exercise: Let Patina Teach Itself Something Useful

This exercise demonstrates the core Patina workflow: using an AI coding agent to add useful project knowledge, indexing that knowledge locally, and then querying it back through Patina.

The goal is not to test whether Patina can store text. That is trivial. The goal is to test whether Patina can help a repository accumulate useful reasoning that would otherwise remain buried in chat history, issue comments, or release notes.

By the end of the exercise, your repository should contain new knowledge pages created by an AI agent, validated by Patina, indexed locally, and retrievable through deterministic CLI queries.

## What You Will Add

You will ask an AI coding agent to add knowledge about Patina’s own release and agent-integration strategy.

This is a useful topic because it combines several real project decisions:

* why Patina uses the CLI as its stable integration boundary
* why agent skills should be thin wrappers over CLI commands
* why the local index is disposable
* why release artifacts are built per platform
* why macOS Apple Silicon is supported, while macOS Intel may be intentionally omitted
* why tag-triggered GitHub Actions releases depend on the workflow file at the tagged commit

This knowledge is specific enough to be valuable, but general enough to demonstrate how Patina handles concepts, systems, decisions, and operational lessons.

## Prerequisites

You need:

* Patina installed and available on your `PATH`
* a Git repository containing a Patina knowledge base
* an AI coding agent with access to the repository
* Patina skills or agent instructions installed

Run:

```bash
patina doctor
```

If the repository is not initialized yet, run:

```bash
patina init
patina index --reset
patina install-skills
```

If your agent supports explicit Patina skills, install the relevant target:

```bash
patina install-skills --for github-copilot
patina install-skills --for codex
patina install-skills --for claude-code
```

Use the target that matches your AI coding tool.

## Step 1: Check the Current Knowledge Base

Before adding anything, query the current knowledge base.

Run:

```bash
patina query "CLI contract agent skills release workflow" --json --limit 5
```

Then try:

```bash
patina query "GitHub Actions release tag macOS runner" --json --limit 5
```

Expected result: Patina may return general project pages, but it should not yet contain a complete explanation of the release and agent-integration strategy.

This absence is intentional. The exercise creates that knowledge.

## Step 2: Ask the Agent to Add the Knowledge

Give the following instruction to your AI coding agent.

```text
Use the Patina knowledge workflow.

I want to add new project knowledge about Patina’s release and agent-integration strategy.

First, query the existing knowledge base for:

- Patina CLI
- agent skills
- GitHub Copilot
- Codex
- Claude Code
- release workflow
- GitHub Actions
- macOS runner
- tag-triggered release

Use `patina query` and `patina read` before editing.

Then add or update knowledge pages that explain the following:

1. Patina’s stable integration boundary is the CLI.
   Agent skills for GitHub Copilot, Codex, and Claude Code should remain thin wrappers over CLI commands such as `patina query`, `patina read`, `patina lint`, `patina stale`, and `patina index`.

2. The local `.patina/` directory is generated state.
   It is useful for retrieval and indexing, but it is not source of truth and must not be committed or included as a release artifact.

3. Patina release artifacts are built through a GitHub Actions matrix.
   The intended release targets are Linux x64, Windows x64, and macOS ARM64 / Apple Silicon.

4. macOS Intel builds are intentionally not required for the current release strategy.
   The macOS ARM64 build should use an Apple Silicon runner label such as `macos-26`.

5. Tag-triggered GitHub Actions workflows use the workflow file from the tagged commit.
   Changing the workflow YAML on `main` does not fix an existing tag-triggered release unless the tag points to the corrected commit or a new tag is created.

Create or update appropriate Markdown pages under the Patina knowledge directory.

Prefer these pages if they do not already exist:

- `knowledge/wiki/decisions/patina-cli-as-stable-agent-contract.md`
- `knowledge/wiki/systems/patina-release-workflow.md`
- `knowledge/wiki/concepts/agent-skills-as-thin-adapters.md`

Use valid front matter with `title`, `type`, and `status`.

After editing, run:

- `patina lint --json`
- `patina index`
- `patina query "why is the CLI the stable integration boundary for Patina?" --json --explain`
- `patina query "what happens when a GitHub Actions release tag points to an old workflow?" --json --explain`

Report:

- files created or changed
- lint result
- the most relevant query result for each query
- any gaps or uncertainties
```

## Step 3: Review the Agent’s Changes

Inspect the Git diff.

```bash
git diff
```

Check that the agent created or updated knowledge pages, not generated cache files.

The diff should include files under `knowledge/`.

It should not include `.patina/`.

If `.patina/` appears in the diff, stop and fix `.gitignore`.

## Step 4: Validate the Knowledge Base

Run:

```bash
patina lint
```

Then:

```bash
patina stale
```

If lint reports errors, ask the agent to fix them.

Do not accept pages with broken links, missing required front matter, duplicate aliases, or invalid source references.

## Step 5: Rebuild or Update the Index

Run:

```bash
patina index
```

If you want to test full regeneration, delete the local generated state and rebuild:

```bash
rm -rf .patina
patina index --full
```

On Windows, remove `.patina` through PowerShell or Explorer and run the same indexing command afterwards.

The knowledge base should recover completely from Git-tracked Markdown files.

## Step 6: Query for the Nugget

Now query the new knowledge.

```bash
patina query "why is the CLI the stable integration boundary for Patina?" --json --explain
```

A good result should retrieve the decision page about the CLI contract.

Then run:

```bash
patina query "what happens when a GitHub Actions release tag points to an old workflow?" --json --explain
```

A good result should retrieve the release workflow page and explain that tag-triggered workflows use the workflow file at the tagged commit.

Finally, try:

```bash
patina query "why are Patina agent skills thin wrappers?" --json --explain
```

A good result should retrieve the concept page about agent skills as adapters over the CLI.

## What This Exercise Demonstrates

This exercise demonstrates several important properties of Patina.

First, Patina can preserve project reasoning, not only documentation. The release-tag lesson is a concrete operational fact that is easy to lose in chat history but valuable to keep.

Second, Patina keeps the agent honest. The agent should query existing knowledge before writing, update Markdown files, run validation, and rebuild the index.

Third, Patina separates durable and disposable state. The useful knowledge lives in Git-tracked Markdown. The local index can be rebuilt.

Fourth, Patina makes knowledge queryable after it is added. The test is not complete until the new pages can be found through `patina query`.

Fifth, Patina creates a shared operating model for humans and agents. A human can review the Markdown. An agent can consume the JSON output. Both use the same repository.

## Expected Outcome

After completing the exercise, your repository should contain new or updated knowledge pages explaining:

* Patina’s CLI-first integration model
* agent skills as thin adapters
* release target choices
* GitHub Actions tag-trigger behaviour
* the disposable nature of `.patina/`

The most interesting result should be a queryable operational lesson:

> In a tag-triggered GitHub Actions release, the workflow is evaluated from the tagged commit. Fixing the workflow on `main` is not enough unless the tag is moved to the corrected commit or a new tag is created.

That is the kind of knowledge Patina is meant to preserve.
