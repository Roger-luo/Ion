# Symlink-based Skill Installation

## Problem

Ion currently copies skill files to multiple directories (`.agents/skills/` and optionally `.claude/skills/`). This creates multiple sources of truth, wastes disk space, and makes maintenance harder.

## Solution

Replace secondary copies with directory symlinks. Keep `.agents/skills/<name>/` as the canonical location with real files. All other agent directories (`.claude/`, `.cursor/`, etc.) get symlinks pointing to the canonical copy.

## Architecture

### Installation flow

1. Fetch/update skill into cache (`~/.cache/ion/repos/...`)
2. Copy skill from cache into `.agents/skills/<name>/` (canonical)
3. For each entry in `[options.targets]`: create a relative directory symlink
4. Check `.gitignore` for managed directories, prompt user to add missing ones

### Symlink strategy

Directory-level symlinks using relative paths for portability:

```
.claude/skills/brainstorming -> ../../.agents/skills/brainstorming
.cursor/skills/brainstorming -> ../../.agents/skills/brainstorming
```

Relative path is computed at install time from the target directory to the canonical directory.

### Removal flow

1. Remove `.agents/skills/<name>/` (canonical copy)
2. Remove symlinks in all configured target directories

### Idempotency

- If symlink exists and points correctly: skip
- If symlink exists but points wrong: replace
- If real directory exists (from older install): remove and create symlink

## Configuration

### New format

Replace `install-to-claude` with a named targets map:

```toml
[skills]
brainstorming = "anthropics/skills/brainstorming"

[options.targets]
claude = ".claude/skills"
cursor = ".cursor/skills"
```

### Data structure

```rust
pub struct ManifestOptions {
    pub targets: BTreeMap<String, String>,  // name -> relative path
}
```

### Breaking change

The old `install-to-claude` option is removed. If present in `ion.toml`, ion errors with a message explaining the new `[options.targets]` format.

## Gitignore handling

After install, ion checks whether managed directories are in `.gitignore`:

- `.agents/` (canonical)
- All paths from `[options.targets]` (e.g. `.claude/`, `.cursor/`)

If any are missing, ion prints which directories are not ignored and prompts:

```
These directories are not in .gitignore:
  .agents/
  .claude/

Add them? [y/n]
```

If the user confirms, ion appends the entries to `.gitignore` (creating it if needed).

## Key decisions

- **Canonical location:** `.agents/skills/<name>/` (real files, copied from cache)
- **Symlink type:** Directory symlinks with relative paths
- **Config format:** Named entries in `[options.targets]` for readable output
- **Backwards compat:** None. Old `install-to-claude` is a hard error with migration guidance
- **Gitignore:** Interactive prompt for all managed directories including `.agents/`
