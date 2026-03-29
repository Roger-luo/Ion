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
ion project init
```

This creates an empty `Ion.toml` with a default target configuration for your detected AI tool.

> [!TIP]
> If you're already in a project with an existing `Ion.toml`, you can skip this step and go straight to adding skills.

## Add a skill

Add a skill from GitHub:

```bash
ion add github:owner/skill-name
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

This reads the manifest and installs all declared skills, using the lockfile for exact versions.

## Create a local skill

Create a skill that lives in your project:

```bash
ion skill new my-skill
```

This creates a `SKILL.md` in your skills directory (`.agents/skills/my-skill/` by default) with the proper frontmatter template.

## Check skill quality

Validate a skill against Ion's built-in checks:

```bash
ion skill validate path/to/skill
```

The validator checks for security issues, structural problems, and adherence to the skill format specification.

> [!NOTE]
> Validation runs automatically when adding remote skills. Use `ion skill validate` for manual checks on local skills.

## Search for skills

Find skills across configured sources:

```bash
ion search "code review"
```

Add `--interactive` for a TUI-based search experience:

```bash
ion search -i
```
