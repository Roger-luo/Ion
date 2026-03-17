---
title: Configuration
description: Ion configuration reference.
---

Ion uses a global configuration file at `~/.config/ion/config.toml` for user-wide settings.

## Location

| Platform | Path |
|----------|------|
| macOS | `~/.config/ion/config.toml` |
| Linux | `~/.config/ion/config.toml` |

## Viewing Configuration

```bash
ion config
```

## Settings

### Targets

Configure default agent targets:

```toml
[targets]
default = ["claude"]
```

### Registries

Configure skill registries:

```toml
[[registries]]
name = "default"
url = "https://skills.sh"
```

### Cache

Control search cache behavior:

```toml
[cache]
max-age-days = 1
```

The search cache is stored in `~/Library/Application Support/ion/search_cache/` on macOS. Use `ion cache gc` to clean up stale entries.

### Sources

Configure additional skill sources:

```toml
[[sources]]
name = "my-org"
type = "github"
owner = "my-org"
```
