---
title: Ion.toml
description: Project manifest file reference.
---

`Ion.toml` is the project manifest file that declares your skill dependencies and configuration. It lives in your project root.

## Example

```toml
[options]
targets = ["claude"]
skills-dir = ".agents/skills"

[skills.brainstorming]
source = "obra/superpowers/brainstorming"

[skills.code-review]
source = "owner/repo/code-review"
rev = "abc123"

[skills.my-custom-skill]
type = "local"

[skills.ejected-skill]
type = "local"
forked-from = "obra/superpowers/brainstorming"
```

## `[options]`

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `targets` | `string[]` | `[]` | Agent targets (e.g., `"claude"`, `"cursor"`, `"windsurf"`) |
| `skills-dir` | `string` | `".agents/skills"` | Directory for local skills |

## `[skills.<name>]`

Each skill entry is keyed by its name (lowercase with hyphens, 1-64 characters).

### Remote Skills

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `source` | `string` | Yes | Skill source (e.g., `"owner/repo"` or `"owner/repo/path"`) |
| `rev` | `string` | No | Pin to a specific git revision |

### Local Skills

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `type` | `string` | Yes | Must be `"local"` |
| `forked-from` | `string` | No | Original source if ejected from a remote skill |

## Ion.lock

The lockfile (`Ion.lock`) is automatically generated and should be committed to version control. It pins exact revisions and checksums for reproducible installs.
