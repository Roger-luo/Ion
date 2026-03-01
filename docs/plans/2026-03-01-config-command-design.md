# Config Command Design

## Overview

Add an `ion config` command with two modes:
1. **Interactive TUI** (default) — full-screen ratatui interface for browsing and editing config
2. **Non-interactive CLI** — `get`, `set`, `list` subcommands for scripting and direct edits

## CLI Interface

```
ion config                              # Launch interactive TUI
ion config get <key> [--project]        # Get a config value
ion config set <key> <value> [--project] # Set a config value
ion config list [--project]             # List all config values
```

### Dot notation keys

Keys use dot notation to address config sections:
- `targets.claude` → `[targets]` section, key `claude`
- `sources.superpowers` → `[sources]` section, key `superpowers`
- `cache.max-age-days` → `[cache]` section, key `max-age-days`
- `ui.color` → `[ui]` section, key `color`

### Scope

- **Default:** Global config at `~/.config/ion/config.toml`
- **`--project` flag:** Project-level `[options]` section in `ion.toml`

### Examples

```bash
$ ion config get targets.claude
.claude/skills

$ ion config set targets.cursor .cursor/skills
Set targets.cursor = ".cursor/skills" in global config

$ ion config list
[targets]
claude = ".claude/skills"
cursor = ".cursor/skills"

[sources]
superpowers = "obra/superpowers"

[cache]
max-age-days = 30

[ui]
color = true

$ ion config set targets.windsurf .windsurf/skills --project
Set targets.windsurf = ".windsurf/skills" in project config
```

## Interactive TUI

Uses **ratatui** with **crossterm** backend.

### Layout

```
┌─ Ion Config ─────────────────────────────────────┐
│  ◄ Global ►   Project                            │
│──────────────────────────────────────────────────│
│                                                   │
│  [Targets]                                        │
│  > claude ··············· .claude/skills          │
│    cursor ··············· .cursor/skills          │
│                                                   │
│  [Sources]                                        │
│    superpowers ·········· obra/superpowers         │
│                                                   │
│  [Cache]                                          │
│    max-age-days ········· 30                      │
│                                                   │
│  [UI]                                             │
│    color ················ true                     │
│                                                   │
│──────────────────────────────────────────────────│
│  ↑↓ Navigate  ←→ Tab  Enter Edit  a Add  d Del  │
│  q Quit  s Save                                   │
└──────────────────────────────────────────────────┘
```

### Keyboard shortcuts

| Key | Action |
|-----|--------|
| ←/→ | Switch between Global and Project tabs |
| ↑/↓ | Navigate between config entries |
| Enter | Edit selected value inline |
| a | Add new key-value pair to current section |
| d | Delete selected entry (with confirmation) |
| s | Save changes to disk |
| q / Esc | Quit (prompt if unsaved changes) |

### Project tab behavior

Shows only project-level settings (`[options.targets]` from `ion.toml`). If no `ion.toml` exists, displays "No ion.toml found in current directory."

## Architecture

### New dependencies

- `ratatui` — TUI framework
- `crossterm` — Terminal backend

### New files

| File | Purpose |
|------|---------|
| `src/commands/config.rs` | CLI dispatch: routes to get/set/list or launches TUI |
| `src/tui/mod.rs` | TUI module exports |
| `src/tui/app.rs` | App state: current tab, selected item, edit mode, dirty flag |
| `src/tui/ui.rs` | Rendering: layout, tabs, sections, help bar |
| `src/tui/event.rs` | Event handling: keyboard input → state transitions |

### Modified files

| File | Change |
|------|--------|
| `src/main.rs` | Add `Config` variant to `Commands` enum |
| `src/commands/mod.rs` | Export config module |
| `crates/ion-skill/src/config.rs` | Add get/set/delete by dot-notation key |
| `crates/ion-skill/src/manifest.rs` | Add get/set for project-level options |

### Data flow — TUI

```
Load GlobalConfig + Manifest → Build AppState → Render loop
  ↓ on keypress
Handle event → Mutate AppState → Re-render
  ↓ on save
Serialize AppState → Write to config.toml / ion.toml via toml_edit
```

### Data flow — Non-interactive

```
get: Parse dot key → Load config → Lookup → Print value
set: Parse dot key → Load file as toml_edit::Document → Set value → Write file
list: Load config → Print all sections and values
```

### Key decisions

1. **Config mutation uses `toml_edit`** to preserve comments and formatting (already used in `manifest_writer.rs`).
2. **App state is a simple struct**, not a full ELM/Redux pattern. Fields: tab, cursor position, edit buffer, dirty flag.
3. **Save writes both files independently** — global via config writer, project via manifest writer.
