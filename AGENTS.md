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
- `SkillSource` / `SourceType` — where a skill comes from (GitHub, Git, HTTP, Path, Binary, Local)
- `SkillMetadata` — parsed from SKILL.md YAML frontmatter (name, description, compatibility, etc.)
- `Manifest` — Ion.toml: project skill configuration with targets and skill entries
- `Lockfile` / `LockedSkill` — Ion.lock: pinned versions with checksums
- `GlobalConfig` — ~/.config/ion/config.toml: user-wide settings (targets, registries, sources)
- `ProjectContext` — per-command aggregation of manifest, lockfile, config, and project paths

**Installation pipeline:** Resolve source → Fetch (git clone or download) → Validate → Install to target dirs → Write manifest + lockfile. Binary skills use a specialized path: download from GitHub Releases → extract → verify checksum.

**Update system** (`ion-skill/src/update/`): `Updater` trait with `check()` / `apply()` methods, dispatched per source type. `GitUpdater` fetches latest from default branch and re-validates; `BinaryUpdater` checks GitHub Releases for newer versions. Pinned skills (with `rev` set) and path/HTTP skills are skipped.

**Validation system** (`ion-skill/src/validate/`): Multiple checkers (security, structure, markdown, codeblock) producing `Finding` items with `Severity` levels, aggregated into `ValidationReport`.

**Search system** (`ion-skill/src/search/`): Multiple backends (GitHub API, skills.sh registry, configured agent command). Interactive search uses a TUI built with ratatui/crossterm (`src/tui/`).

**Local skills:** `SourceType::Local` for project-specific skills. Created via `ion skill new` (with optional `--dir` flag), or ejected from remote via `ion skill eject`. Tracked in Ion.toml as `{ type = "local" }` with optional `forked-from`. Local skills skip fetch/validation/gitignore — they're managed by git directly. Config: `skills-dir` in `[options]` (default `.agents/skills`). Skills live at `{skills-dir}/{name}/`.

**Self-update system** (`src/commands/self_cmd.rs`): `ion self update` downloads pre-built binaries from GitHub Releases for the `Roger-luo/Ion` repo, using the same `binary.rs` infrastructure as skill binary installs. `ion self check` compares versions. `ion self info` shows version, build target, and exe path. Build target triple embedded via `build.rs`.

**Search cache** (`ion-skill/src/search/cache.rs`): File-based JSON cache in `~/Library/Application Support/ion/search_cache/`, keyed by `(source_name, query)` hash. TTL from `cache.max-age-days` config (default 1 day). Agent source is never cached.

## Command Structure

Top-level: `add`, `remove`, `search`, `update`, `run`
Subcommand groups: `skill` (new, validate, info, list, link, eject), `project` (init, migrate), `cache` (gc), `config`, `self` (update, check, info)

`ion add` with no args runs install-all from Ion.toml. With a source arg, adds a single skill.

## Key Conventions

- **Manifest files:** `Ion.toml` and `Ion.lock` (capitalized)
- **Error handling:** `anyhow::Result` in CLI code, custom `ion_skill::Error` (thiserror) in library
- **Skill names:** lowercase with hyphens, 1-64 chars, no leading/trailing/consecutive hyphens
- **Rust edition:** 2024
- **Tests are primarily integration tests** in `/tests/`, using `tempfile` for isolated temp directories and invoking the compiled binary via `env!("CARGO_BIN_EXE_ion")`
- **Colored output:** `style::Paint` helper respects config and terminal capabilities

## Git & Release Conventions

- **Conventional commits:** `feat:`, `fix:`, `docs:`, `test:`, `ci:`, `refactor:`, `perf:`, `build:`, `chore:`
- **Breaking changes:** Use `feat!:` or `fix!:` (note the `!`) or add a `BREAKING CHANGE:` footer in the commit body. This triggers a minor version bump (pre-1.0) or major bump (post-1.0). Examples: changing CLI flags, renaming config keys, altering default behavior. `refactor:` alone is NOT breaking — it means internal restructuring with no behavior change.
- **Version bumps (pre-1.0):** `fix:`/`refactor:`/`docs:` → patch, `feat:` → patch, `feat!:`/`BREAKING CHANGE` → minor
- **Linear history:** main branch must not contain merge commits — use rebase or squash merges
- **Automated releases:** release-plz opens version bump PRs based on conventional commits. Merging creates a tag, which triggers GitHub Actions to build binaries for 4 targets (aarch64/x86_64 × macOS/Linux)
- **Asset naming:** `ion-{version}-{target}.tar.gz` (e.g. `ion-0.2.0-aarch64-apple-darwin.tar.gz`)
