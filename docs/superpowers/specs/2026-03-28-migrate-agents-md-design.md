# Design: AGENTS.md/CLAUDE.md Migration & Management

**Date:** 2026-03-28
**Scope:** CLAUDE.md gitignore + symlink management, migrate manual CLAUDE.md to ion-managed AGENTS.md

## Problem

Ion creates a `CLAUDE.md -> AGENTS.md` symlink for projects with `claude` as a configured target. However:

1. CLAUDE.md is not gitignored — both files can end up tracked in git, causing confusion.
2. Projects with a manual CLAUDE.md (real content or just an `@AGENTS.md` pointer) have no migration path to the symlink model.
3. `ion migrate` doesn't handle AGENTS.md/CLAUDE.md conversion at all.

## Design

Two-part change: steady-state behavior in `ensure_agent_symlinks`, one-time conversion in `ion migrate`.

### Part 1: `ensure_agent_symlinks` changes (agents.rs)

**Gate:** All CLAUDE.md handling only applies when `"claude"` is in `[options.targets]`. If claude is not a configured target, CLAUDE.md is never touched.

**Current behavior:** Creates `CLAUDE.md -> AGENTS.md` symlink if AGENTS.md exists and CLAUDE.md doesn't. Warns and skips if CLAUDE.md exists as a regular file.

**New behavior:**

1. **Gitignore on symlink creation:** After creating or verifying the CLAUDE.md symlink, add `CLAUDE.md` to `.gitignore` under the managed section. Idempotent — existing projects get the entry added on next `ion add` or `ion init`.

2. **Pointer file detection:** When CLAUDE.md exists as a regular file, check if it's a pointer file using `is_agents_pointer()`. If it is: delete it, create the symlink, gitignore it. No prompt needed.

3. **Real content:** When CLAUDE.md has real content, keep the existing behavior (warn and skip). `ion migrate` handles this case.

#### Pointer detection: `is_agents_pointer(content: &str) -> bool`

A CLAUDE.md is a "pointer file" if its only meaningful purpose is to reference AGENTS.md. Detection heuristic:

- Strip blank lines and whitespace-only lines.
- The file contains `@AGENTS.md`.
- All non-blank lines either contain `@AGENTS.md` or are surrounding prose/boilerplate (e.g., "treat @AGENTS.md the same as this file").

Examples that are pointers:
- `@AGENTS.md`
- `treat @AGENTS.md the same as this file`
- Whitespace/blank lines around an `@AGENTS.md` reference

Examples that are NOT pointers:
- A file with `@AGENTS.md` plus additional instructions, rules, or content
- A file with no `@AGENTS.md` reference at all

### Part 2: Gitignore support (gitignore.rs)

New function: `ensure_agent_file_ignored(project_dir: &Path, filename: &str) -> Result<()>`

Adds a single file entry (e.g., `CLAUDE.md`) to `.gitignore` under the `# Managed by ion` section. Reuses the same idempotent pattern as `add_skill_entries`: check if line already exists, append if not.

Called from `ensure_agent_symlinks` whenever a symlink is created or already exists correctly.

### Part 3: `ion migrate` — AGENTS.md/CLAUDE.md conversion phase

New phase inserted **before** skill discovery (Phase 1), since it's about project-level files, not skills.

Only runs when `"claude"` is in the project's configured targets.

#### State detection table

| AGENTS.md | CLAUDE.md | Action |
|-----------|-----------|--------|
| Exists | Missing | Create symlink + gitignore (handled by `ensure_agent_symlinks`) |
| Exists | Pointer file | Delete CLAUDE.md, create symlink + gitignore. Auto with `--yes`. |
| Exists | Real content | Prompt: keep (1) AGENTS.md, (2) CLAUDE.md, or (3) abort. **Always confirm** — `--yes` prints a message and skips. Backup loser to `.bak`. |
| Missing | Pointer file | Warn: "CLAUDE.md references AGENTS.md but it doesn't exist." Skip. |
| Missing | Real content | Rename CLAUDE.md → AGENTS.md, create symlink + gitignore. **Always confirm** — `--yes` prints a message and skips. |
| Missing | Missing | No-op. |

#### `--yes` behavior

- **Pointer files:** Auto-proceed (safe, no content loss).
- **Real content files:** Always require confirmation. With `--yes`, print "Both AGENTS.md and CLAUDE.md have content — run without --yes to choose which to keep" and skip the conversion phase. No data loss.

#### JSON mode

Reports the action taken under a new `"agents_md"` key in the output JSON:
- `{"action": "symlinked"}` — pointer replaced or fresh symlink created
- `{"action": "renamed", "from": "CLAUDE.md", "backup": "CLAUDE.md.bak"}` — content file renamed
- `{"action": "skipped", "reason": "conflict"}` — both have content, user must resolve
- `null` — no CLAUDE.md handling needed (claude not a target, or nothing to do)

### Part 4: Files modified

1. **`crates/ion-skill/src/agents.rs`**
   - Add `is_agents_pointer(content: &str) -> bool`
   - Modify `ensure_agent_symlinks`: pointer detection + gitignore call
   - Unit tests for pointer detection patterns

2. **`crates/ion-skill/src/gitignore.rs`**
   - Add `ensure_agent_file_ignored(project_dir, filename)`
   - Unit tests for idempotency

3. **`src/commands/migrate.rs`**
   - Add `migrate_agents_md(project_dir, targets, json, yes) -> Result<Option<AgentsMdAction>>`
   - Insert before existing Phase 1
   - Include in JSON output

4. **`tests/migrate_integration.rs`**
   - Pointer CLAUDE.md replaced with symlink + gitignored
   - Real CLAUDE.md without AGENTS.md prompts for rename
   - Both exist prompts for choice
   - Skipped when claude not in targets
   - `--yes` auto-handles pointer, skips real content conflict

No new files created.
