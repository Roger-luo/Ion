---
title: Managing Skills
description: How to add, remove, update, and organize skills with Ion.
---

## Adding Skills

Ion supports multiple skill sources:

### From GitHub

```bash
# Standalone repo
ion add owner/skill-repo

# Skill inside a monorepo
ion add owner/repo/path/to/skill

# Specific branch or revision
ion add owner/repo --rev main
```

### From a Git URL

```bash
ion add https://github.com/owner/repo.git
```

### From a Local Path

```bash
ion add --path ./my-skills/custom-skill
```

## Removing Skills

```bash
ion remove skill-name
```

This removes the skill from your `Ion.toml`, `Ion.lock`, and the target installation directory.

## Updating Skills

```bash
# Update all skills to their latest versions
ion update

# Update a specific skill
ion update skill-name
```

Pinned skills (those with a specific `rev` set) are skipped during updates.

## Skill Information

```bash
# Show details about an installed skill
ion skill info skill-name

# Validate all skills
ion skill validate
```

## The Manifest and Lockfile

### Ion.toml

The manifest declares your skill dependencies and targets:

```toml
[options]
targets = ["claude"]

[skills.brainstorming]
source = "obra/superpowers/brainstorming"
```

### Ion.lock

The lockfile pins exact versions with checksums. Commit this file to ensure everyone on your team gets identical skill versions:

```toml
[[skill]]
name = "brainstorming"
source = "obra/superpowers/brainstorming"
rev = "abc123..."
checksum = "sha256:..."
```
