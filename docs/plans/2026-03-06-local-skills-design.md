# Local Skills Design

## Problem

Ion currently manages remote skills (GitHub, git, binary) and can link local paths. But there's no first-class workflow for:
- Creating project-specific skills that live in the project repo
- Forking ("ejecting") a remote skill into an editable local copy

## Solution

Add a `local` source type and two workflows: `ion skill new` creates local skills, `ion skill eject` converts remote skills to local.

## `skills-dir` Config

New optional field under `[options]` in Ion.toml:

```toml
[options]
skills-dir = ".agents"
```

Default: `.agents` (the existing canonical skill location). Can be overridden via Ion.toml or `ion skill new --dir X`.

When set via `--dir`, the value is persisted to Ion.toml for future use.

## `local` Source Type

New `SourceType::Local` variant. In Ion.toml:

```toml
[skills]
# Existing remote skill
brainstorming = "anthropics/skills/brainstorming"

# Project-local skill (created with ion skill new)
my-deploy = { type = "local" }

# Ejected skill (forked from remote)
brainstorming = { type = "local", forked-from = "anthropics/skills/brainstorming" }
```

### Behavior

| Operation | Local skill behavior |
|-----------|---------------------|
| `ion add` (install all) | Ensure target symlinks exist, skip fetch/validation |
| `ion update` | Skip (no remote) |
| `ion remove` | Remove Ion.toml entry + target symlinks. Do NOT delete skill directory. |
| `.gitignore` | Never gitignored (tracked by git) |
| Lockfile | Checksum only (no commit, no remote URL) |

## `ion skill new` Enhancement

Current: creates a SKILL.md in the current directory or `--path`.

New behavior:
- If `skills-dir` is configured (or using default `.agents`), create under `{skills-dir}/skills/{name}/`
- Set up target symlinks (`.claude/skills/{name}` → `.agents/skills/{name}`)
- Add `{ type = "local" }` entry to Ion.toml
- `--path` still works as a one-off override (no Ion.toml tracking)
- `--dir X` sets `skills-dir = X` in Ion.toml, then creates under `{X}/skills/{name}/`

```bash
# Uses default .agents, creates .agents/skills/my-tool/SKILL.md
ion skill new

# Sets skills-dir and creates under custom location
ion skill new --dir my-skills

# One-off, no Ion.toml tracking
ion skill new --path /tmp/scratch-skill
```

## `ion skill eject <name>` — New Command

Converts a remote skill into an editable local copy.

### Flow

1. Verify skill exists in Ion.toml as remote type (github/git)
2. Resolve `skills-dir` (default: `.agents`)
3. Copy skill content from cached repo to `{skills-dir}/skills/{name}/`
4. Replace `.agents/skills/{name}` symlink: point at new local copy (or if skills-dir is `.agents`, it's already a real directory in place)
5. Target symlinks (`.claude/skills/{name}` etc.) unchanged — they already point at `.agents/skills/{name}`
6. Update Ion.toml: `{ type = "local", forked-from = "anthropics/skills/brainstorming" }`
7. Remove `.gitignore` entries for this skill
8. Update Ion.lock: drop commit hash, keep checksum
9. Print confirmation message

### Symlink structure after eject

When `skills-dir = ".agents"` (default):
```
.agents/skills/brainstorming/     ← real directory (was symlink to cache)
.claude/skills/brainstorming      ← symlink → ../../.agents/skills/brainstorming
```

When `skills-dir = "skills"` (custom):
```
skills/brainstorming/             ← real directory (ejected content)
.agents/skills/brainstorming      ← symlink → ../../skills/brainstorming
.claude/skills/brainstorming      ← symlink → ../../.agents/skills/brainstorming
```

## Deferred

- `ion skill publish` / extract-to-repo: convert a local skill into a standalone git repository and re-link it as a remote skill
