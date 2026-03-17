---
title: Configuration
description: Ion configuration reference.
---

Ion uses a global configuration file at `~/.config/ion/config.toml` for user-wide settings. These are merged with per-project settings in `Ion.toml`.

## Location

| Platform | Path |
|----------|------|
| macOS | `~/.config/ion/config.toml` |
| Linux | `~/.config/ion/config.toml` |

## Viewing and Setting Configuration

```bash
# List all global config values
ion config

# Get a specific value
ion config get targets.claude

# Set a value
ion config set targets.claude ".claude/skills"

# Delete a value
ion config delete targets.cursor
```

All keys use dot-notation: `section.key`.

## `[targets]`

Default agent targets applied to all projects (unless overridden by `[options.targets]` in `Ion.toml`). Maps target names to skill directories:

```toml
[targets]
claude = ".claude/skills"
cursor = ".cursor/skills"
```

Project-level targets in `Ion.toml` take precedence over global targets on key collision.

## `[sources]`

Source aliases for shorthand skill references. If the first path segment matches an alias, it's expanded:

```toml
[sources]
superpowers = "obra/superpowers"
```

With this, `ion add superpowers/brainstorming` expands to `ion add obra/superpowers/brainstorming`.

URLs and local paths (starting with `https://`, `./`, `../`, or `/`) are never expanded.

## `[registries]`

Configure skill registries for `ion search`:

```toml
[registries.skills-sh]
url = "https://skills.sh/api"
default = true

[registries.my-company]
url = "https://skills.internal.co/api"
```

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `url` | `string` | Yes | Registry API endpoint |
| `default` | `bool` | No | If `true`, results appear before GitHub results |

The built-in `skills.sh` registry is always included and doesn't need configuration.

## `[cache]`

Control search cache behavior:

```toml
[cache]
max-age-days = 1
```

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `max-age-days` | `integer` | `1` | How long cached search results are considered fresh |

The search cache is stored in `~/Library/Application Support/ion/search_cache/` on macOS. Use `ion cache gc` to clean up stale entries. Agent source results are never cached.

## `[ui]`

```toml
[ui]
color = true
```

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `color` | `bool` | auto-detected | Force colored output on or off |

## `[search]`

```toml
[search]
agent-command = "claude -p 'search: {query}'"
```

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `agent-command` | `string` | None | Command template for agent-based search. `{query}` is replaced with the search query. |

Use with `ion search --agent` to include results from an AI agent alongside registry and GitHub results.
