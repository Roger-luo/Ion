---
title: Commands
description: Complete reference for all Ion CLI commands.
---

## Top-Level Commands

### `ion add [source]`

Add a skill or install all skills from `Ion.toml`.

```bash
# Install all from manifest
ion add

# Add a single skill
ion add owner/repo
ion add owner/repo/path/to/skill
ion add --path ./local/skill
```

**Flags:**
- `--rev <rev>` — Pin to a specific git revision
- `--yes` — Skip confirmation prompts

### `ion remove <name>`

Remove a skill from the project.

```bash
ion remove brainstorming
```

### `ion search <query>`

Search for skills across registries and GitHub.

```bash
ion search "code review"
```

Opens an interactive TUI for browsing results.

### `ion update [name]`

Update skills to their latest versions.

```bash
# Update all
ion update

# Update specific skill
ion update brainstorming
```

### `ion run <name> [args...]`

Run a binary skill.

```bash
ion run my-tool --flag value
```

---

## `ion skill` Subcommands

### `ion skill new`

Create a new local skill.

**Flags:**
- `--dir <path>` — Custom skills directory

### `ion skill validate`

Validate all skill definitions in the project.

### `ion skill list`

List all installed skills.

### `ion skill info <name>`

Show detailed information about a skill.

### `ion skill link <path>`

Link an external skill directory.

### `ion skill eject <name>`

Eject a remote skill into an editable local copy.

---

## `ion project` Subcommands

### `ion project init`

Initialize `Ion.toml` in the current directory.

**Flags:**
- `--target <target>` — Set the agent target (e.g., `claude`, `cursor`, `windsurf`)

### `ion project migrate`

Migrate from legacy configuration formats.

---

## `ion cache` Subcommands

### `ion cache gc`

Clean up stale cached repositories.

---

## `ion config`

View and set configuration values.

---

## `ion self` Subcommands

### `ion self update`

Update Ion to the latest version.

### `ion self check`

Check if a newer version is available.

### `ion self info`

Show version, build target, and executable path.

---

## Global Flags

- `--json` — Structured JSON output (place before subcommand)
- `--help` — Show help for any command
- `--version` — Show version
