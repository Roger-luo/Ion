# Design: Symlink-Based Deployment + Per-Skill Gitignore

**Date:** 2026-03-02
**Status:** Approved

## Summary

Replace copy-based skill deployment with symlinks to a persistent global data directory. Switch from blanket `.agents/` gitignore to per-skill entries so that locally authored skills remain tracked in git while remotely installed skills are ignored. Add `ion link` for local skills and `ion gc` for cleaning stale repos.

## Core Model

- **Remote skills** (`ion add`): `.agents/skills/<name>` → symlink to `~/.local/share/ion/repos/<hash>/...` → per-skill gitignore entry
- **Local skills** (`ion link`): `.agents/skills/<name>` → symlink to local path (e.g. `skills/my-skill`) → no gitignore entry (tracked)
- **Target symlinks** (e.g. `.claude/skills/<name>`): unchanged, still point to `.agents/skills/<name>`

## Change 1: Storage Location

**Current:** `~/.cache/ion/repos/<hash>/` (via `dirs::cache_dir()`)
**New:** `~/.local/share/ion/repos/<hash>/` (via `dirs::data_dir()`)

Platform paths:
- Linux: `~/.local/share/ion/repos/`
- macOS: `~/Library/Application Support/ion/repos/`
- Windows: `%APPDATA%/ion/repos/`

One-time migration: if old cache dir has repos and new data dir doesn't, move them.

## Change 2: `ion add` — Symlink + Per-Skill Gitignore

**Current:** Copies skill files from cache into `.agents/skills/<name>/`.
**New:** Creates symlink from `.agents/skills/<name>` → skill directory in global data store.

After installing, automatically adds per-skill gitignore entries (no prompt):

```gitignore
# Managed by ion
.agents/skills/brainstorming
.claude/skills/brainstorming
```

Each skill gets an entry for `.agents/skills/<name>` plus one entry per target.

Also registers the project in the global registry (see Change 5).

## Change 3: `ion link` — New Command for Local Skills

```
ion link <path>
```

Where `<path>` points to a local directory containing a `SKILL.md`.

### Behavior

1. Validate the skill at `<path>` (same validation as `ion add`)
2. Create symlink: `.agents/skills/<name>` → `<path>` (relative)
3. Create symlinks for each target (e.g. `.claude/skills/<name>` → `.agents/skills/<name>`)
4. Register in `ion.toml` as a local path source: `my-skill = { type = "path", source = "skills/my-skill" }`
5. Update `ion.lock` (checksum only, no commit SHA)
6. **No gitignore entries** — local skills are tracked in git

### Difference from `ion add ./path`

`ion add` with a local path should behave the same as `ion link` — symlink instead of copy. Both paths converge to the same behavior for local sources.

## Change 4: `ion install` — Remove Blanket Gitignore

**Current:** Prompts to add `.agents/` and target dirs as blanket gitignore entries.
**New:** Remove the blanket gitignore prompt. Instead, add per-skill gitignore entries for each remote skill installed. Local path skills get no gitignore entries.

## Change 5: `ion gc` — Garbage Collect Stale Repos

```
ion gc [--dry-run]
```

### Global Registry

**File:** `~/.local/share/ion/registry.toml`

```toml
[repos.f8a3f25821e2a56d]
url = "https://github.com/obra/superpowers.git"
projects = [
    "/Users/roger/Code/my-project",
    "/Users/roger/Code/another-project",
]
```

**Maintained by:**
- `ion add` / `ion install` — registers project dir + repo hash
- `ion remove` — unregisters the project for that repo
- `ion gc` — validates and cleans stale entries

### `ion gc` Flow

1. Load `registry.toml`
2. For each repo entry:
   - Remove projects whose directories no longer exist
   - Remove projects whose `ion.lock` no longer references this repo
   - If no projects remain, delete the repo from data dir
3. Write updated `registry.toml`
4. `--dry-run` lists what would be cleaned without acting

## Change 6: `ion remove` — Clean Up Gitignore

When removing a skill, also remove its per-skill gitignore entries (`.agents/skills/<name>` and target entries).

## Error Handling

- **Broken symlinks** (data dir deleted): `ion install` re-fetches repos and recreates symlinks. Document this as the recovery path.
- **`ion link` with missing SKILL.md**: error with clear message
- **`ion gc` with missing registry**: create empty registry, scan data dir for orphaned repos

## File Changes

| File | Change |
|------|--------|
| `crates/ion-skill/src/installer.rs` | Symlink to data dir instead of copy for remote skills |
| `crates/ion-skill/src/installer.rs` | Change `cache_dir()` → `data_dir()` |
| `crates/ion-skill/src/gitignore.rs` | Add `add_per_skill_entries()`, `remove_per_skill_entries()` |
| `crates/ion-skill/src/registry.rs` | New — global registry read/write/update |
| `src/commands/add.rs` | Call per-skill gitignore after install, register in registry |
| `src/commands/install.rs` | Remove blanket gitignore prompt, add per-skill entries, register |
| `src/commands/link.rs` | New — `ion link` command |
| `src/commands/remove.rs` | Clean up per-skill gitignore entries, unregister from registry |
| `src/commands/gc.rs` | New — `ion gc` command |
| `src/main.rs` | Add `Link` and `Gc` variants |
| `src/commands/mod.rs` | Export `link` and `gc` modules |

## Testing

### Unit Tests
- `gitignore.rs`: add/remove per-skill entries, idempotency
- `registry.rs`: register/unregister projects, load/save, stale detection
- `installer.rs`: symlink creation instead of copy, data dir path

### Integration Tests
- `ion add` remote skill creates symlinks + gitignore entries
- `ion link` local skill creates symlinks, no gitignore entries
- `ion remove` cleans up symlinks + gitignore entries
- `ion install` re-creates broken symlinks
- `ion gc --dry-run` reports stale repos without deleting
- `ion gc` removes repos with no remaining projects
