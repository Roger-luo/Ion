# Ion Workspace Support Design

## Overview

Workspace-first redesign of Ion's project model. Every project becomes a workspace-of-one. Multi-project workspaces are opt-in via an explicit `[workspace]` section in the root `Ion.toml`.

## Motivation

Ion currently operates on a single `Ion.toml` per invocation, discovered strictly from the current directory. Users with multi-project setups (e.g., a Rust CLI with a docs site) cannot manage skills for sub-projects from the root, and `ion update` cannot maintain the entire workspace's skills and AGENTS.md templates in one pass.

## Core Data Model

`WorkspaceContext` replaces `ProjectContext` as the single entry point for all commands.

```
WorkspaceContext
├── root_dir: PathBuf              // workspace root (where root Ion.toml lives)
├── root_project: Project          // the root project itself
├── members: Vec<Project>          // sub-projects declared in [workspace]
├── active_project: Option<&Project> // resolved from CWD or --project flag
└── config: GlobalConfig           // user-wide config (unchanged)

Project
├── dir: PathBuf                   // project directory (root or sub-project)
├── manifest: Manifest             // this project's Ion.toml
├── lockfile: Lockfile             // this project's Ion.lock
└── effective_options: Options     // merged: root options + local overrides
```

A solo project (no `[workspace]` section) is a workspace-of-one: `members` is empty, `root_project` is the only project, `active_project` defaults to it. Every command goes through the same `WorkspaceContext` code path.

### Discovery Logic

Replaces the current `ProjectContext::load()`:

1. Walk up from CWD looking for `Ion.toml` files.
2. If an `Ion.toml` has `[workspace]`, that's the root — load all members.
3. If CWD is inside a member, set it as `active_project`.
4. If no `[workspace]` found, the nearest `Ion.toml` is a standalone workspace-of-one.
5. If no `Ion.toml` found at all, error (same as today).

## Manifest Format

### Root `Ion.toml`

```toml
[workspace]
members = ["docs", "packages/frontend"]

[options.targets]
claude = ".claude/skills"

[skills]
systematic-debugging = { source = "obra/superpowers/systematic-debugging" }
```

### Sub-project `docs/Ion.toml`

```toml
# No [workspace] section — its absence marks this as a member

[options.targets]
# Omitted — inherited from root

[options]
skills-dir = ".agents/skills"  # Can override if needed

[skills]
frontend-design = { source = "pbakaus/impeccable/source/skills/frontend-design" }
web-design-guidelines = { source = "vercel-labs/agent-skills/web-design-guidelines" }
```

### Option Inheritance

- `[options.targets]` — inherited from root if not declared locally.
- `[options.skills-dir]` — inherited from root if not declared locally.
- `[skills]` — never inherited, always per-project.
- `[workspace]` — only valid in root; error if found in a member.

### Validation

- Members listed in `[workspace]` must exist and contain an `Ion.toml`.
- Circular membership is impossible (only root declares members).
- A project cannot be both a workspace root and a member of another workspace (walk-up stops at the first `[workspace]`).

## Lockfiles

Each project has its own `Ion.lock` — root gets `Ion.lock`, `docs/` gets `docs/Ion.lock`. Each is self-contained. No shared or aggregated lockfile.

## Command Behavior

### Scoping Rules

| Context | Behavior |
|---|---|
| CWD is root, no `--project` | Operates on all projects (root + members) |
| CWD is root, `--project docs` | Operates on `docs` only |
| CWD is root, `--project .` | Operates on root only |
| CWD is inside `docs/` | Operates on `docs` only |
| CWD is inside `docs/`, `--project .` | Operates on root only (flag overrides CWD) |

### Per-Command Details

- **`ion add <source>`** — Adds to active project. From root without `--project`, this is an error (ambiguous). Must specify `--project`.
- **`ion add`** (no args, install-all) — Installs all skills for active project(s). From root = all projects.
- **`ion remove <skill>`** — Removes from active project. Error from root without `--project` if skill exists in multiple projects.
- **`ion update`** — Updates all skills in scope. From root = all projects.
- **`ion list`** — Lists skills grouped by project. From root = all projects. From `docs/` = docs skills only.
- **`ion search`** — Unchanged (not project-scoped).
- **`ion project init`** — If inside a workspace, auto-registers as member in root `Ion.toml`. Otherwise creates a standalone workspace-of-one.

### The `--project` Flag

- Accepts a path relative to workspace root, or `.` for root project.
- Added to all project-scoped commands via a shared clap argument.
- Multiple values allowed: `--project docs --project packages/frontend`.

## New Command Group: `ion workspace`

```
ion workspace add <path>     # Create sub-project Ion.toml + register in [workspace]
ion workspace remove <path>  # Unregister from [workspace] (doesn't delete files)
ion workspace list           # Show all members with skill counts
ion workspace status         # Show update availability across all projects
```

### `ion workspace add docs`

1. Adds `"docs"` to `[workspace].members` in root `Ion.toml`.
2. Creates `docs/Ion.toml` if it doesn't exist (with inherited targets).
3. Creates `docs/Ion.lock` (empty).
4. Prints confirmation with inherited options.

### `ion workspace list` Output

```
Workspace: /Users/roger/Code/rust/ion

  . (root)          15 skills
  docs               3 skills
```

### `ion workspace status` Output

```
Workspace: /Users/roger/Code/rust/ion

  . (root)          2 updates available
  docs               0 updates available
```

## AGENTS.md in Workspaces

No new AGENTS.md features. The existing `ion agents` commands gain workspace awareness like all other commands:

- **`ion agents init <source>`** — From root without `--project`, initializes template for root only (setup command, not ambiguous). With `--project docs`, initializes for `docs/`.
- **`ion agents update`** — From root, updates templates for all projects that have `[agents]` configured. With `--project`, scopes to one.
- **`ion agents diff`** — From root, shows diffs for all projects. With `--project`, scopes to one.
- **Sub-project symlinks** — `ensure_agent_symlinks` runs in each sub-project's directory using that project's inherited targets.
- **Option inheritance** — Sub-projects inherit `[options.targets]` from root, so symlink targets (claude, codex, etc.) are consistent unless overridden.

## Migration

Existing single-project `Ion.toml` files work without changes. The absence of `[workspace]` means workspace-of-one. No migration step required.
