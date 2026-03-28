# Track 2: CLI Wrappers Crate — Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Create `ion-cli` crate with typed, reusable wrappers for git, cargo, gh, and shell CLIs. Migrate all existing call sites. Replace `diff` CLI with `similar` crate.

**Architecture:** New `crates/ion-cli/` workspace crate. Both `ion-skill` and `ion` depend on it. Central `run_command()`/`run_status()` helpers with consistent `CliError` type. Existing `ion-skill/src/git.rs` becomes thin re-exports. `checksum_dir()` (pure Rust, not a CLI) stays in `ion-skill`.

**Tech Stack:** Rust, `thiserror`, `serde_json` (for cargo metadata), `similar` (for diff), `log`.

**Spec:** `docs/superpowers/specs/2026-03-27-codebase-refactor-design.md` — Track 2.

---

## File Map

### New files
| File | Responsibility |
|------|---------------|
| `crates/ion-cli/Cargo.toml` | Crate manifest — minimal deps (thiserror, serde_json, log) |
| `crates/ion-cli/src/lib.rs` | `CliError` type, `run_command()`, `run_status()`, `is_available()`, module re-exports |
| `crates/ion-cli/src/git.rs` | Git CLI wrappers (clone_or_fetch, checkout, head_commit, etc.) |
| `crates/ion-cli/src/cargo.rs` | Cargo CLI wrappers (metadata, build_release, run, init) |
| `crates/ion-cli/src/gh.rs` | GitHub CLI wrappers (run, available, star_repo) |
| `crates/ion-cli/src/shell.rs` | Shell command wrapper (run_sh) |

### Modified files
| File | Change |
|------|--------|
| `Cargo.toml` (root) | Add `ion-cli` to workspace members + dependencies |
| `crates/ion-skill/Cargo.toml` | Add `ion-cli` dependency |
| `crates/ion-skill/src/git.rs` | Replace `Command::new("git")` with `ion_cli::git::*`, keep `checksum_dir()` |
| `crates/ion-skill/src/error.rs` | Add `From<ion_cli::CliError>` |
| `crates/ion-skill/src/binary.rs` | Replace cargo calls with `ion_cli::cargo::*` |
| `crates/ion-skill/src/search/github.rs` | Replace `run_gh`/`gh_available` with `ion_cli::gh::*` |
| `crates/ion-skill/src/search/agent.rs` | Replace `Command::new("sh")` with `ion_cli::shell::run_sh()` |
| `src/commands/migrate.rs` | Replace raw git `Command::new` calls with `ion_cli::git::*` |
| `src/commands/add.rs` | Replace `Command::new("gh")` with `ion_cli::gh::star_repo()` |
| `src/commands/run.rs` | Replace `Command::new("cargo")` with `ion_cli::cargo::*` |
| `src/commands/new.rs` | Replace `Command::new("cargo")` with `ion_cli::cargo::init()` |
| `src/commands/agents.rs` | Replace `Command::new("diff")` with `similar` crate |

---

### Task 1: Create `ion-cli` crate scaffold with `CliError` and core helpers

Create the crate with Cargo.toml, lib.rs (CliError, run_command, run_status, is_available), and empty module files.

### Task 2: Implement `git` module

Move git operations from `ion-skill/src/git.rs` into `ion-cli/src/git.rs`. Add new operations (stage_files, has_staged_changes, create_commit, init). Port existing tests.

### Task 3: Migrate `ion-skill/src/git.rs` to re-export from `ion-cli`

Replace `Command::new("git")` calls with `ion_cli::git::*`. Keep `checksum_dir()` in `ion-skill`. Add `From<CliError>` to `ion-skill::Error`.

### Task 4: Implement `cargo` module

Add cargo metadata, build_release, run, run_interactive, init wrappers.

### Task 5: Migrate cargo call sites

Replace cargo calls in `binary.rs`, `run.rs`, `new.rs`.

### Task 6: Implement `gh` and `shell` modules

Add gh run/available/star_repo and shell run_sh wrappers.

### Task 7: Migrate gh and shell call sites

Replace gh calls in `search/github.rs`, `add.rs`. Replace sh call in `search/agent.rs`.

### Task 8: Migrate `migrate.rs` git operations

Replace raw `Command::new("git")` calls in `migrate.rs` with `ion_cli::git::*`.

### Task 9: Replace `diff` CLI with `similar` crate

Replace `Command::new("diff")` in `agents.rs` with pure-Rust diff using `similar`.

### Task 10: Final verification
