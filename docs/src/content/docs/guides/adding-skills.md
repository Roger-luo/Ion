---
title: Adding Skills
description: How to add skills from different sources.
order: 10
---

# Adding Skills

Ion supports multiple skill sources. This guide covers each one and when to use them.

## From GitHub

The most common way to add a skill:

```bash
ion add github:owner/repo-name
```

Ion clones the repository, validates the skill, and installs it. The GitHub source supports pinning to a specific revision:

```bash
ion add github:owner/repo-name --rev abc1234
```

Pinned skills won't be updated by `ion update` — they stay at the exact commit you specified.

## From Git URLs

For skills hosted outside GitHub:

```bash
ion add git:https://gitlab.com/owner/repo.git
```

This works with any Git hosting provider.

## From HTTP

For standalone skill archives:

```bash
ion add https://example.com/my-skill.tar.gz
```

## Local skills

Create skills that live in your project's repository:

```bash
ion new
```

Ion prompts for a name and creates a `SKILL.md` in your skills directory with the proper frontmatter template. To create at a specific path:

```bash
ion new --path .agents/skills/my-custom-skill
```

Local skills are tracked in `Ion.toml` as `{ type = "local" }` and managed by your project's version control directly — they skip the fetch, validate, and gitignore steps that remote skills go through.

### Ejecting a remote skill

You can convert a remote skill to a local copy for customization:

```bash
ion skill eject skill-name
```

This copies the skill files into your local skills directory and updates the manifest. The original source is recorded as `forked-from` metadata.

## Managing skills

List installed skills:

```bash
ion list
```

Remove a skill:

```bash
ion remove skill-name
```

Update all skills to their latest versions:

```bash
ion update
```
