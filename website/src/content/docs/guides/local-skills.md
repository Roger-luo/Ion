---
title: Local Skills
description: Create and manage project-specific skills.
---

Local skills are project-specific skills that live in your repository and are tracked by git directly. They skip the fetch/validation pipeline since you own and maintain them.

## Creating a Local Skill

```bash
ion skill new
```

This creates a new skill in your project's skills directory (default: `.agents/skills/`). You'll be prompted for a name and description.

### Custom Skills Directory

```bash
ion skill new --dir ./my-skills
```

Or configure the default in `Ion.toml`:

```toml
[options]
skills-dir = "my-skills"
```

## Ejecting a Remote Skill

Want to customize an existing remote skill? Eject it into a local copy:

```bash
ion skill eject brainstorming
```

This creates a local copy in your skills directory and updates `Ion.toml` to track it as a local skill with a `forked-from` reference to the original source.

## How Local Skills Are Tracked

In `Ion.toml`, local skills appear as:

```toml
[skills.my-custom-skill]
type = "local"

[skills.ejected-skill]
type = "local"
forked-from = "obra/superpowers/brainstorming"
```

Local skills are installed to target directories just like remote skills, but they're managed by git instead of Ion's fetch pipeline.

## Linking External Directories

If you have skills in a directory outside your project:

```bash
ion skill link /path/to/external/skill
```
