# Design: `ion new` Command (renamed from `ion init`)

**Date:** 2026-03-02
**Status:** Approved

## Summary

Rename `ion init` to `ion new` and add a `--collection` flag that scaffolds a multi-skill collection project (like obra/superpowers). The existing single-skill scaffolding behavior is unchanged.

## CLI Interface

```
ion new [--path <dir>] [--bin] [--collection] [--force]
```

| Flag | Description |
|------|-------------|
| `--path <dir>` | Target directory (default: current directory) |
| `--bin` | Run `cargo init --bin` to scaffold a Rust CLI project alongside the skill |
| `--collection` | Create a multi-skill collection with a `skills/` directory |
| `--force` | Overwrite existing files if they exist |

`--collection` and `--bin` are mutually exclusive.

## Behavior: Default Mode (single skill)

Unchanged from `ion init`. Creates a `SKILL.md` in the target directory.

## Behavior: Collection Mode (`--collection`)

1. **Resolve target directory** — use `--path` if provided, otherwise current directory
2. **Derive collection name** — from the target directory name, using the existing slugify logic
3. **Check for existing README.md** — if exists and `--force` not set, exit with error
4. **Create `skills/` directory** — empty, ready for skills to be added via `ion new --path skills/<name>`
5. **Write README.md** — minimal template with collection name
6. **Print success message** — `Created skill collection in <path>`

### Scaffolded Structure

```
<target>/
  skills/        # empty directory
  README.md      # minimal readme
```

### README Template

```markdown
# {title}

A collection of skills for AI agents.

## Skills

Add skills with:

\```bash
ion new --path skills/<skill-name>
\```
```

## Rename: `init` → `new`

`new` reads better for a scaffolding command — it creates something fresh rather than configuring an existing project.

This is a clean rename with no backwards-compatibility alias. The project is new enough that this is not a concern.

## Error Cases

- `--collection` combined with `--bin`: error — "cannot combine --collection with --bin"
- README.md already exists without `--force` (collection mode): error suggesting `--force`
- SKILL.md already exists without `--force` (default mode): error suggesting `--force` (existing behavior)
- `--bin` but `cargo` not found: error with message to install Rust toolchain (existing behavior)
- `--path` directory doesn't exist: create it (existing behavior)

## File Changes

| File | Change |
|------|--------|
| `src/commands/init.rs` → `src/commands/new.rs` | Rename, add collection branch |
| `src/commands/mod.rs` | Rename `init` → `new` |
| `src/main.rs` | `Commands::Init` → `Commands::New`, add `--collection` flag |
| `tests/init_integration.rs` → `tests/new_integration.rs` | Rename, add collection tests |

## Testing

- Unit tests: existing slugify tests (unchanged)
- Integration tests:
  - `ion new` in empty temp dir creates SKILL.md (existing, renamed)
  - `ion new --path <dir>` creates SKILL.md in specified directory (existing, renamed)
  - `ion new` with existing SKILL.md errors without `--force` (existing, renamed)
  - `ion new --force` overwrites existing SKILL.md (existing, renamed)
  - `ion new --bin` runs cargo init and creates SKILL.md (existing, renamed)
  - `ion new --collection` creates `skills/` dir and `README.md`
  - `ion new --collection --path <dir>` creates collection in specified directory
  - `ion new --collection` with existing README.md errors without `--force`
  - `ion new --collection --force` overwrites existing README.md
  - `ion new --collection --bin` errors
