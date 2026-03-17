---
title: Quick Start
description: Get up and running with Ion in minutes.
---

## Initialize a Project

Start by initializing Ion in your project directory. Choose a target agent:

```bash
ion project init --target claude
```

This creates an `Ion.toml` manifest file in your project root.

## Add Skills

Add skills from GitHub repositories:

```bash
# Add a skill from a GitHub monorepo
ion add obra/superpowers/brainstorming

# Add from a standalone repo
ion add owner/repo
```

Ion will fetch the skill, validate it, and install it to the appropriate target directory (e.g., `.claude/commands/` for Claude).

## Install All Skills

If you're joining a project that already has an `Ion.toml`, install everything at once:

```bash
ion add
```

This reads the manifest and installs all declared skills, using the lockfile (`Ion.lock`) to ensure reproducible versions.

## Search for Skills

Find skills across registries and GitHub:

```bash
ion search "code review"
```

This opens an interactive TUI where you can browse results and add skills directly.

## List Installed Skills

```bash
ion skill list
```

## Update Skills

```bash
# Update all skills
ion update

# Update a specific skill
ion update my-skill
```

## What's Next?

- [Managing Skills](/guides/managing-skills) — deep dive into add, remove, and update workflows
- [Local Skills](/guides/local-skills) — create and customize project-specific skills
- [Commands Reference](/reference/commands) — full command reference
