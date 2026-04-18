---
title: Quick Start
description: Add your first skill in under a minute.
order: 3
---

# Quick Start

This guide walks through the basics of using Ion to manage skills in a project.

## Initialize a project

Start by creating an `Ion.toml` manifest in your project:

```bash
ion init
```

This prompts you to select which AI agent targets to configure (Claude, Cursor, Windsurf, etc.) and — if Ion detects your project language — offers to set up an `AGENTS.md` context file inline.

> [!TIP]
> If you're already in a project with an existing `Ion.toml`, you can skip this step and go straight to adding skills.

## Add a skill

Add a skill from GitHub:

```bash
ion add owner/repo/skill-name
```

Ion will:

1. Resolve the source and fetch the skill
2. Validate its structure and security
3. Install it to the configured target directory
4. Update `Ion.toml` and `Ion.lock`

## Install all skills

If you already have an `Ion.toml` with skills listed, install everything:

```bash
ion add
```

This reads the manifest and installs all declared skills, using the lockfile for exact versions. Run this after cloning a project.

## Create a local skill

Create a skill that lives in your project:

```bash
ion new
```

Ion prompts for a name and creates a `SKILL.md` in your skills directory (`.agents/skills/<name>/` by default) with the proper frontmatter template. Validation runs automatically after creation.

To create at a specific path without a name prompt:

```bash
ion new --path .agents/skills/my-skill
```

## Check skill quality

Validate a skill against Ion's built-in checks:

```bash
ion validate path/to/skill
```

The validator checks for security issues, structural problems, and adherence to the skill format.

> [!NOTE]
> Validation runs automatically when adding remote skills and when creating skills with `ion new`. Use `ion validate` to re-check local skills after editing.

## Search for skills

Find skills across configured sources:

```bash
ion search "code review"
```

An interactive TUI picker lets you browse results and install directly.
