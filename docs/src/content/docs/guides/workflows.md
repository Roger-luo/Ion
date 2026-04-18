---
title: Typical Workflows
description: Step-by-step walkthroughs of the most common Ion workflows.
order: 9
---

# Typical Workflows

This page walks through the workflows you'll use most often with Ion, from setting up a new project to collaborating with a team.

## New project setup

You have a codebase and want to make it agent-ready.

```bash
$ ion init
? Select agent targets: [claude] cursor windsurf
? Set up AGENTS.md? [Y/n] (template: rust)
✓ Created Ion.toml with 1 target: claude → .claude/skills
✓ Created AGENTS.md from template
✓ Updated Ion.toml with template source
```

`ion init` detects your project language (Cargo.toml → rust, pyproject.toml → python, both → rust+python, Project.toml → julia) and offers to set up an `AGENTS.md` context file inline — no separate command needed.

Commit the results:

```bash
$ git add Ion.toml AGENTS.md .gitignore
$ git commit -m "chore: set up ion"
```

## Finding and adding a skill

Search across registries and GitHub, then install from the interactive picker:

```bash
$ ion search brainstorm
  1  brainstorming        obra/superpowers       ★ 204
     Ideates before any creative or feature work
  2  feature-brainstorm   acme/skills            ★ 12

[↑↓ select  enter install  q quit]
```

Selecting a result installs immediately — no separate `ion add` step needed.

To add directly without searching:

```bash
$ ion add obra/superpowers/brainstorming
✓ Installed brainstorming → .agents/skills/brainstorming/
✓ Updated Ion.toml and Ion.lock
```

## Cloning a project that already uses Ion

Skills are gitignored and tracked in `Ion.toml`. After cloning, one command restores everything:

```bash
$ git clone git@github.com:acme/my-project && cd my-project
$ ion add
Installing 4 skill(s)...
  ✓ brainstorming
  ✓ feature-dev
  ✓ writing-plans
  ✓ commit-commands
Done!
```

`ion add` with no arguments reads `Ion.toml` and installs all declared skills at the pinned versions from `Ion.lock`.

## Writing a local skill

Local skills live in your project and are tracked by git directly.

```bash
$ ion new
Skill name: deploy-staging
✓ Created .agents/skills/deploy-staging/SKILL.md
✓ Registered 'deploy-staging' in Ion.toml as local skill
```

Then edit the generated `SKILL.md` with your instructions. Validation runs automatically after creation — warnings surface immediately. Run it again any time:

```bash
$ ion validate .agents/skills/deploy-staging/
✓ deploy-staging  no issues
```

## Customizing a remote skill

Eject a remote skill into an editable local copy:

```bash
$ ion list
  brainstorming    obra/superpowers  v1.2.0   ✓
  feature-dev      obra/superpowers  v1.2.0   ✓

$ ion skill eject brainstorming
✓ Copied to .agents/skills/brainstorming/
✓ Converted to local in Ion.toml  (forked-from: obra/superpowers)
✓ Removed from .gitignore
```

The skill is now tracked in git and fully yours to edit. The `forked-from` metadata records where it came from.

## Keeping skills up to date

```bash
$ ion update
  ✓  brainstorming   abc1234 → def5678
  ✓  feature-dev     up to date
  ✓  writing-plans   abc1234 → 9f02b1a
  —  deploy-staging  local, skipped
Summary: 2 updated, 1 up to date, 1 skipped
```

Local and pinned skills are skipped automatically. To update a single skill:

```bash
$ ion update brainstorming
```

## Merging upstream AGENTS.md changes

When your template source publishes updates, `ion update` flags it and stages the new content:

```bash
$ ion update
  ✓  brainstorming   updated
  !  agents template: a3f91c → 7b20de
     upstream saved to .agents/templates/AGENTS.md.upstream
```

Review the diff and merge with your agent:

```bash
$ ion agents diff
--- local/AGENTS.md
+++ upstream/AGENTS.md
@@ -12,3 +12,8 @@
 ## Testing
+### Property-based tests
+Use proptest for ...
```

Ask your agent: *"Merge the upstream AGENTS.md changes into my local version."*

## Building a binary skill

Binary skills are compiled CLI tools invoked via `ion run`. Scaffold a new one:

```bash
$ ion init --bin my-linter
✓ Cargo project scaffolded
✓ SKILL.md created
Set up GitHub Actions CI/CD? [Y/n]
✓ .github/workflows/ci.yml
✓ .github/workflows/release.yml
✓ .github/workflows/release-plz.yml
```

Test it locally while developing:

```bash
$ cd my-project
$ ion add --dev ../my-linter
✓ Registered 'my-linter' (dev mode) — ion run forwards to cargo run

$ ion run my-linter check --fix
```

Once ready to publish, push to GitHub. The release workflow builds binaries for macOS and Linux automatically. Users install with:

```bash
$ ion add your-org/my-linter
```

## Workspace — monorepo with per-project skills

```bash
$ ion workspace list
  . (root)      3 skills
  apps/api      2 skills
  apps/web      1 skill

# Add a skill to one sub-project
$ ion add obra/superpowers/writing-plans --project apps/api

# Install all skills across all projects
$ ion add
  . (root)   Installing 3 skill(s)...  Done!
  apps/api   Installing 3 skill(s)...  Done!
  apps/web   Installing 1 skill(s)...  Done!
```

## Quick reference

```bash
ion init          # set up project (targets + AGENTS.md)
ion add <src>     # install a remote skill
ion add           # restore all skills (after git clone)
ion new           # create a local skill
ion update        # pull latest versions
ion list          # what's installed
ion search <q>    # find skills
ion remove <n>    # uninstall a skill
```
