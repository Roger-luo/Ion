# Global Configuration

## Problem

Ion has no global user-level configuration. All settings are per-project in `ion.toml`. Users who want the same targets (e.g., always symlink to `.claude/skills/`) across all projects must duplicate config. There's no way to define source aliases, control cache behavior, or configure CLI appearance.

## Solution

Add a global `config.toml` using platform-appropriate directories via the `dirs` crate (already a dependency).

## Directory layout

Using `dirs` crate for XDG-compliant, cross-platform paths:

| Purpose | Linux | macOS |
|---------|-------|-------|
| Config | `~/.config/ion/config.toml` | `~/Library/Preferences/ion/config.toml` |
| Cache | `~/.cache/ion/repos/` | `~/Library/Caches/ion/repos/` |

Cache already lives here today. The only addition is the config file.

## Config file format

TOML, consistent with `ion.toml`. Four sections:

```toml
[targets]
claude = ".claude/skills"

[sources]
superpowers = "obra/superpowers"
anthropic = "anthropics/skills"

[cache]
max-age-days = 30

[ui]
color = true
```

### `[targets]`

Default symlink targets applied to all projects. A project's `[options.targets]` merges on top. On key collision, the project wins.

### `[sources]`

Named aliases for repositories. Allows shorthand like `ion add superpowers/brainstorming` instead of `ion add obra/superpowers/brainstorming`. Additive only; project config doesn't override these.

### `[cache]`

Cache management settings. `max-age-days` controls auto-pruning of stale cached repos.

### `[ui]`

CLI appearance. `color = true/false` toggles colored output. Future: specific color overrides for different UI elements.

## Resolution order

Project config overrides global config:

- **Targets:** Global `[targets]` merged with project `[options.targets]`. Project wins on key collision.
- **Sources:** Global aliases available everywhere. Purely additive.
- **Cache/UI:** Global-only settings. No per-project override.

## Architecture

### New module: `config`

In the `ion-skill` crate:

```rust
pub struct GlobalConfig {
    pub targets: BTreeMap<String, String>,
    pub sources: BTreeMap<String, String>,
    pub cache: CacheConfig,
    pub ui: UiConfig,
}

pub struct CacheConfig {
    pub max_age_days: Option<u32>,
}

pub struct UiConfig {
    pub color: Option<bool>,
}
```

Key methods:
- `GlobalConfig::load()` — reads from `dirs::config_dir()/ion/config.toml`, returns `Default` if file missing
- `GlobalConfig::save()` — writes config back (for future `ion config` command)
- `GlobalConfig::resolve_targets(project: &ManifestOptions) -> BTreeMap<String, String>` — merges global + project targets
- `GlobalConfig::resolve_source(input: &str) -> String` — expands source aliases

### CLI integration

- `install_skill()` and `uninstall_skill()` receive merged targets (global + project)
- `SkillSource::infer()` checks global source aliases before parsing
- Cache directory already uses `dirs::cache_dir()`, no change needed
- UI color config read at CLI startup in `main.rs`

## Key decisions

- **Platform dirs via `dirs` crate** — already a dependency, zero cost
- **TOML format** — consistent with `ion.toml`
- **Project overrides global** — targets merge with project winning on conflict
- **Source aliases are additive** — no collision semantics needed
- **No `ion config` CLI command yet** — users edit the file directly for now
