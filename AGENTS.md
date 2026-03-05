# AGENTS.md

This file provides guidance when working with code in this repository.

## Project

Ion is a Rust CLI skill manager for AI agent tools (Claude, Cursor, Windsurf, etc.). It manages installation, validation, and organization of "skills" — reusable prompts/instructions that enhance AI agent capabilities.

## Build & Test Commands

```bash
cargo build                              # Dev build
cargo test                               # All tests (unit + integration)
cargo test <test_name>                   # Single test by name
cargo test --test integration            # Tests from one file
cargo clippy                             # Lint
cargo fmt                                # Format
```

## Architecture

**Workspace layout** — two crates:
- `ion` (root) — CLI binary using clap with command dispatch in `src/main.rs`
- `ion-skill` (`crates/ion-skill/`) — core library with all business logic

**Command pattern:** Each CLI command lives in `src/commands/<name>.rs` with a `pub fn run(...)` entry point. `main.rs` parses args via clap derive and dispatches to the appropriate command module.

**Core data types** (in `ion-skill`):
- `SkillSource` / `SourceType` — where a skill comes from (GitHub, Git, HTTP, Path, Binary)
- `SkillMetadata` — parsed from SKILL.md YAML frontmatter (name, description, compatibility, etc.)
- `Manifest` — Ion.toml: project skill configuration with targets and skill entries
- `Lockfile` / `LockedSkill` — Ion.lock: pinned versions with checksums
- `GlobalConfig` — ~/.config/ion/config.toml: user-wide settings (targets, registries, sources)
- `ProjectContext` — per-command aggregation of manifest, lockfile, config, and project paths

**Installation pipeline:** Resolve source → Fetch (git clone or download) → Validate → Install to target dirs → Write manifest + lockfile. Binary skills use a specialized path: download from GitHub Releases → extract → verify checksum.

**Update system** (`ion-skill/src/update/`): `Updater` trait with `check()` / `apply()` methods, dispatched per source type. `GitUpdater` fetches latest from default branch and re-validates; `BinaryUpdater` checks GitHub Releases for newer versions. Pinned skills (with `rev` set) and path/HTTP skills are skipped.

**Validation system** (`ion-skill/src/validate/`): Multiple checkers (security, structure, markdown, codeblock) producing `Finding` items with `Severity` levels, aggregated into `ValidationReport`.

**Search system** (`ion-skill/src/search/`): Multiple backends (GitHub API, skills.sh registry, configured agent command). Interactive search uses a TUI built with ratatui/crossterm (`src/tui/`).

## Key Conventions

- **Manifest files:** `Ion.toml` and `Ion.lock` (capitalized)
- **Error handling:** `anyhow::Result` in CLI code, custom `ion_skill::Error` (thiserror) in library
- **Skill names:** lowercase with hyphens, 1-64 chars, no leading/trailing/consecutive hyphens
- **Rust edition:** 2024
- **Tests are primarily integration tests** in `/tests/`, using `tempfile` for isolated temp directories and invoking the compiled binary via `env!("CARGO_BIN_EXE_ion")`
- **Colored output:** `style::Paint` helper respects config and terminal capabilities
