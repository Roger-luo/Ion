---
title: Configuration
description: Configuring Ion for your project and environment.
order: 11
---

# Configuration

Ion uses two levels of configuration: project-level (`Ion.toml`) and user-level (`~/.config/ion/config.toml`).

## Project configuration

The `Ion.toml` manifest in your project root defines which skills to install and where:

```toml
[options]
skills-dir = ".agents/skills"     # Where local skills live

[[targets]]
type = "claude"                    # Target AI tool
path = ".claude/skills"            # Installation directory

[[skills]]
name = "code-review"
source = "github:owner/code-review"

[[skills]]
name = "my-local-skill"
source = { type = "local" }
```

### Targets

Targets define where skills are installed. Each target has a `type` and `path`:

| Type | Default path | Tool |
|------|-------------|------|
| `claude` | `.claude/skills` | Claude Code |
| `cursor` | `.cursor/skills` | Cursor |
| `windsurf` | `.windsurf/skills` | Windsurf |

You can define multiple targets to install skills for several tools simultaneously.

### Options

| Option | Default | Description |
|--------|---------|-------------|
| `skills-dir` | `.agents/skills` | Directory for local skills |

## Global configuration

User-wide settings live at `~/.config/ion/config.toml`:

```toml
[cache]
max-age-days = 1          # Search cache TTL

[[sources]]
name = "my-registry"
type = "registry"
url = "https://skills.example.com"
```

### Sources

Global sources are available for `ion search` across all projects. Each source has a `name`, `type`, and type-specific fields.

### Cache

The search cache stores results locally to avoid repeated network requests. Configure the TTL with `cache.max-age-days`. Clear the cache manually:

```bash
ion cache gc
```
