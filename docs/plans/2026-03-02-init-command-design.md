# Design: `ion init` Command

**Date:** 2026-03-02
**Status:** Approved

## Summary

Add an `ion init` command that scaffolds a new skill project by creating a SKILL.md template in the target directory. Optionally, `--bin` delegates to `cargo init --bin` to also scaffold a Rust CLI project alongside the skill.

## CLI Interface

```
ion init [--path <dir>] [--bin] [--force]
```

| Flag | Description |
|------|-------------|
| `--path <dir>` | Target directory (default: current directory) |
| `--bin` | Run `cargo init --bin` in the target directory before creating SKILL.md |
| `--force` | Overwrite existing SKILL.md if one exists |

## Behavior

1. **Resolve target directory** — use `--path` if provided, otherwise `std::env::current_dir()`
2. **Derive skill name** — from the target directory name, sanitized to a lowercase-hyphenated slug (e.g., `My Cool Skill` → `my-cool-skill`)
3. **Check for existing SKILL.md** — if exists and `--force` not set, exit with error
4. **If `--bin`** — run `cargo init --bin` in the target directory. Fail if cargo is not found or returns non-zero exit code
5. **Write SKILL.md** — guided template with the derived name substituted into frontmatter and heading
6. **Print success message** — `Created SKILL.md in <path>`

## Approach: Embedded Templates

Templates are hardcoded as Rust string constants. No external template files, no template engine, no runtime resolution. This is the simplest approach for a single-file scaffold.

### Default Template

```markdown
---
name: {name}
description: A brief description of what this skill does
# license: MIT
# compatibility: claude-code
# allowed-tools: Bash, Read, Write
# metadata:
#   author: your-name
#   version: 0.1.0
---

# {title}

## Overview

Describe what this skill does and when to use it.

## Process

1. Step one
2. Step two

## Examples

```bash
# Example usage
```
```

Where `{name}` is the derived slug and `{title}` is the title-cased directory name.

### Bin Template

Same as the default template, but with an additional section noting the associated Cargo project.

## File Changes

| File | Change |
|------|--------|
| `src/commands/init.rs` | New — command handler |
| `src/main.rs` | Add `Init` variant to `Commands` enum |
| `src/commands/mod.rs` | Export `init` module |

No changes to the `ion-skill` crate. This is a pure scaffolding command with no library logic.

## Error Cases

- SKILL.md already exists (without `--force`): error with message suggesting `--force`
- `--bin` but `cargo` not found: error with message to install Rust toolchain
- `--bin` but `cargo init` fails (e.g., Cargo.toml already exists): propagate cargo's error
- `--path` directory doesn't exist: create it (like `cargo init`)

## Testing

- Unit test: name derivation/sanitization
- Integration tests:
  - `ion init` in empty temp dir creates SKILL.md with correct name
  - `ion init --path <dir>` creates SKILL.md in specified directory
  - `ion init` with existing SKILL.md errors without `--force`
  - `ion init --force` overwrites existing SKILL.md
  - `ion init --bin` runs cargo init and creates SKILL.md
