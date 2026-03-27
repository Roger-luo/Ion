# Binary Skill CI/CD Setup Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add GitHub Actions CI/CD scaffolding for binary skill projects — via `--ci` flag on `ion init --bin` and a standalone `ion ci` command.

**Architecture:** A shared `setup_ci(project_dir, name, force)` function writes four files (`.github/workflows/ci.yml`, `release.yml`, `release-plz.yml`, and `release-plz.toml`) from const templates parameterized by the binary name. `ion init --bin --ci` calls it after scaffolding; `ion ci` calls it standalone by reading the package name from `Cargo.toml`.

**Tech Stack:** Rust, clap (derive), toml (for Cargo.toml parsing in `ion ci`), GitHub Actions YAML templates as const strings.

---

## File Structure

| Action | Path | Responsibility |
|--------|------|----------------|
| Create | `src/commands/ci.rs` | CI templates, `setup_ci()` shared function, `run()` entry point for `ion ci` |
| Modify | `src/commands/mod.rs` | Register `ci` module |
| Modify | `src/main.rs` | Add `--ci` flag to `Init`, add `Ci` command variant, wire dispatch |
| Modify | `src/commands/new.rs` | Call `ci::setup_ci()` when `--ci` is true |
| Create | `tests/ci_integration.rs` | Integration tests for both `--ci` flag and `ion ci` |

---

## Chunk 1: Core CI Module and CLI Wiring

### Task 1: Create `src/commands/ci.rs` with templates and `setup_ci()`

**Files:**
- Create: `src/commands/ci.rs`

- [ ] **Step 1: Write the failing test (unit test in ci.rs)**

Add the module file with templates and a basic unit test that verifies template substitution:

```rust
// src/commands/ci.rs

use std::path::Path;

const CI_TEMPLATE: &str = r#"name: CI

on:
  push:
    branches: [main]
  pull_request:
    branches: [main]

env:
  CARGO_TERM_COLOR: always

jobs:
  fmt:
    name: Format
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          components: rustfmt
      - run: cargo fmt --all --check

  clippy:
    name: Clippy
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          components: clippy
      - uses: Swatinem/rust-cache@v2
      - run: cargo clippy --all-targets --all-features -- -D warnings

  test:
    name: Test
    strategy:
      matrix:
        os: [ubuntu-latest, macos-latest]
    runs-on: ${{ matrix.os }}
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - uses: Swatinem/rust-cache@v2
      - run: cargo test --all-features
"#;

const RELEASE_TEMPLATE: &str = r#"name: Release

on:
  push:
    tags:
      - "v*"

permissions:
  contents: write

jobs:
  build:
    strategy:
      matrix:
        include:
          - target: aarch64-apple-darwin
            os: macos-latest
          - target: x86_64-apple-darwin
            os: macos-latest
          - target: x86_64-unknown-linux-gnu
            os: ubuntu-latest
          - target: aarch64-unknown-linux-gnu
            os: ubuntu-latest

    runs-on: ${{ matrix.os }}

    steps:
      - uses: actions/checkout@v4

      - name: Install Rust toolchain
        uses: dtolnay/rust-toolchain@stable
        with:
          targets: ${{ matrix.target }}

      - name: Install cross-compilation tools
        if: matrix.target == 'aarch64-unknown-linux-gnu'
        run: |
          sudo apt-get update
          sudo apt-get install -y gcc-aarch64-linux-gnu
          echo "CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER=aarch64-linux-gnu-gcc" >> $GITHUB_ENV

      - name: Build
        run: cargo build --release --target ${{ matrix.target }}

      - name: Package
        shell: bash
        run: |
          VERSION="${GITHUB_REF_NAME#v}"
          cd target/${{ matrix.target }}/release
          tar czf ../../../{name}-${VERSION}-${{ matrix.target }}.tar.gz {name}
          cd ../../..

      - name: Upload artifact
        uses: actions/upload-artifact@v4
        with:
          name: {name}-${{ matrix.target }}
          path: "{name}-*.tar.gz"

  upload:
    needs: build
    runs-on: ubuntu-latest
    steps:
      - name: Download all artifacts
        uses: actions/download-artifact@v4
        with:
          merge-multiple: true

      - name: Upload assets to release
        uses: softprops/action-gh-release@v2
        with:
          tag_name: ${{ github.ref_name }}
          files: "{name}-*.tar.gz"
"#;

const RELEASE_PLZ_WORKFLOW_TEMPLATE: &str = r#"name: Release-plz

on:
  push:
    branches:
      - main

permissions:
  contents: write
  pull-requests: write

jobs:
  release-plz-release:
    name: Release-plz release
    runs-on: ubuntu-latest
    steps:
      - name: Checkout repository
        uses: actions/checkout@v4
        with:
          fetch-depth: 0
      - name: Install Rust toolchain
        uses: dtolnay/rust-toolchain@stable
      - name: Run release-plz
        uses: release-plz/action@v0.5
        with:
          command: release
        env:
          GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}

  release-plz-pr:
    name: Release-plz PR
    runs-on: ubuntu-latest
    concurrency:
      group: release-plz-${{ github.ref }}
      cancel-in-progress: false
    steps:
      - name: Checkout repository
        uses: actions/checkout@v4
        with:
          fetch-depth: 0
      - name: Install Rust toolchain
        uses: dtolnay/rust-toolchain@stable
      - name: Run release-plz
        uses: release-plz/action@v0.5
        with:
          command: release-pr
        env:
          GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
"#;

const RELEASE_PLZ_TOML_TEMPLATE: &str = r#"[package]
# Binary skills are distributed via GitHub Releases, not crates.io
publish = false

[changelog]
body = """
{% for group, commits in commits | group_by(attribute="group") %}
### {{ group | upper_first }}
{% for commit in commits %}
- {{ commit.message | upper_first }} ({{ commit.id | truncate(length=7, end="") }})\
{% endfor %}
{% endfor %}
"""
trim = true
commit_parsers = [
    { message = "^feat", group = "Added" },
    { message = "^fix", group = "Fixed" },
    { message = "^doc", group = "Documentation" },
    { message = "^perf", group = "Performance" },
    { message = "^refactor", group = "Refactored" },
    { message = "^test", group = "Testing" },
    { message = "^ci", group = "CI" },
    { message = "^build", group = "Build" },
    { message = "^chore", skip = true },
]
"#;

/// Write GitHub Actions CI/CD workflow files into a project directory.
///
/// Creates `.github/workflows/ci.yml`, `release.yml`, `release-plz.yml`,
/// and a top-level `release-plz.toml`.
pub fn setup_ci(project_dir: &Path, name: &str, force: bool) -> anyhow::Result<Vec<String>> {
    let workflows_dir = project_dir.join(".github/workflows");
    let files: &[(&str, &str, bool)] = &[
        (".github/workflows/ci.yml", CI_TEMPLATE, false),
        (".github/workflows/release.yml", RELEASE_TEMPLATE, true),
        (
            ".github/workflows/release-plz.yml",
            RELEASE_PLZ_WORKFLOW_TEMPLATE,
            false,
        ),
        ("release-plz.toml", RELEASE_PLZ_TOML_TEMPLATE, false),
    ];

    // Pre-check: error if any file exists and --force not set
    if !force {
        for &(rel_path, _, _) in files {
            let full_path = project_dir.join(rel_path);
            if full_path.exists() {
                anyhow::bail!(
                    "{rel_path} already exists. Use --force to overwrite."
                );
            }
        }
    }

    std::fs::create_dir_all(&workflows_dir)?;

    let mut created = Vec::new();
    for &(rel_path, template, needs_name) in files {
        let content = if needs_name {
            template.replace("{name}", name)
        } else {
            template.to_string()
        };
        std::fs::write(project_dir.join(rel_path), content)?;
        created.push(rel_path.to_string());
    }

    Ok(created)
}

/// Read the package name from a Cargo.toml file.
fn read_cargo_package_name(project_dir: &Path) -> anyhow::Result<String> {
    let cargo_toml_path = project_dir.join("Cargo.toml");
    if !cargo_toml_path.exists() {
        anyhow::bail!(
            "No Cargo.toml found in {}. Run this command from a Rust project directory, \
             or use `ion init --bin --ci` to scaffold a new binary skill project with CI.",
            project_dir.display()
        );
    }
    let content = std::fs::read_to_string(&cargo_toml_path)?;
    let doc: toml_edit::DocumentMut = content.parse()?;
    let name = doc
        .get("package")
        .and_then(|p| p.get("name"))
        .and_then(|n| n.as_str())
        .ok_or_else(|| anyhow::anyhow!("Cargo.toml is missing [package].name"))?;
    Ok(name.to_string())
}

/// Entry point for `ion ci` — set up CI/CD in the current directory.
pub fn run(force: bool, json: bool) -> anyhow::Result<()> {
    let project_dir = std::env::current_dir()?;
    let name = read_cargo_package_name(&project_dir)?;
    let created = setup_ci(&project_dir, &name, force)?;

    if json {
        crate::json::print_success(serde_json::json!({
            "name": name,
            "files": created,
        }));
        return Ok(());
    }

    println!("Created CI/CD workflows for '{name}':");
    for f in &created {
        println!("  {f}");
    }
    println!();
    println!("Setup:");
    println!("  1. Push to GitHub");
    println!("  2. CI runs automatically on push and PRs");
    println!("  3. release-plz creates version bump PRs on push to main");
    println!("  4. Merging a release PR tags and builds binaries for 4 targets");

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn setup_ci_creates_all_files() {
        let dir = tempfile::tempdir().unwrap();
        let created = setup_ci(dir.path(), "my-tool", false).unwrap();
        assert_eq!(created.len(), 4);
        assert!(dir.path().join(".github/workflows/ci.yml").exists());
        assert!(dir.path().join(".github/workflows/release.yml").exists());
        assert!(dir.path().join(".github/workflows/release-plz.yml").exists());
        assert!(dir.path().join("release-plz.toml").exists());
    }

    #[test]
    fn setup_ci_substitutes_name_in_release() {
        let dir = tempfile::tempdir().unwrap();
        setup_ci(dir.path(), "my-tool", false).unwrap();
        let content =
            std::fs::read_to_string(dir.path().join(".github/workflows/release.yml")).unwrap();
        assert!(content.contains("my-tool-${VERSION}"));
        assert!(content.contains("my-tool-${{ matrix.target }}"));
        assert!(!content.contains("{name}"));
    }

    #[test]
    fn setup_ci_errors_without_force_if_exists() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join(".github/workflows")).unwrap();
        std::fs::write(dir.path().join(".github/workflows/ci.yml"), "existing").unwrap();
        let err = setup_ci(dir.path(), "my-tool", false).unwrap_err();
        assert!(err.to_string().contains("already exists"));
    }

    #[test]
    fn setup_ci_force_overwrites() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join(".github/workflows")).unwrap();
        std::fs::write(dir.path().join(".github/workflows/ci.yml"), "old").unwrap();
        let created = setup_ci(dir.path(), "my-tool", true).unwrap();
        assert_eq!(created.len(), 4);
        let content =
            std::fs::read_to_string(dir.path().join(".github/workflows/ci.yml")).unwrap();
        assert!(content.contains("cargo fmt"));
    }

    #[test]
    fn ci_template_has_no_name_placeholder() {
        assert!(!CI_TEMPLATE.contains("{name}"));
    }

    #[test]
    fn release_plz_workflow_has_no_name_placeholder() {
        assert!(!RELEASE_PLZ_WORKFLOW_TEMPLATE.contains("{name}"));
    }

    #[test]
    fn release_plz_toml_has_no_name_placeholder() {
        assert!(!RELEASE_PLZ_TOML_TEMPLATE.contains("{name}"));
    }
}
```

- [ ] **Step 2: Register the module in `src/commands/mod.rs`**

Add `pub mod ci;` to `src/commands/mod.rs`:

```rust
pub mod ci;  // add after the existing `pub mod completion;` line
```

- [ ] **Step 3: Run tests to verify they pass**

Run: `cargo nextest run -E 'test(ci::tests)'`
Expected: All 7 unit tests PASS.

- [ ] **Step 4: Commit**

```bash
git add src/commands/ci.rs src/commands/mod.rs
git commit -m "feat: add CI/CD setup module with workflow templates"
```

---

### Task 2: Wire `--ci` flag into `ion init --bin`

**Files:**
- Modify: `src/main.rs:24-36` (Init variant)
- Modify: `src/main.rs:232-243` (Init dispatch)
- Modify: `src/commands/new.rs:310-351` (`run_bin` function)

- [ ] **Step 1: Add `--ci` flag to Init command in `src/main.rs`**

Add a new field to the `Init` variant:

```rust
    Init {
        /// Path for binary skill project (default: current directory)
        path: Option<String>,
        /// Scaffold a binary skill CLI project with ionem
        #[arg(long)]
        bin: bool,
        /// Set up GitHub Actions CI/CD (requires --bin)
        #[arg(long)]
        ci: bool,
        /// Configure specific targets (e.g. claude, cursor, or name:path)
        #[arg(long, short = 't')]
        target: Vec<String>,
        /// Overwrite existing files
        #[arg(long)]
        force: bool,
    },
```

- [ ] **Step 2: Update Init dispatch to pass `ci` to `run_bin`**

In the `match cli.command` block, update the Init arm:

```rust
        Commands::Init {
            path,
            bin,
            ci,
            target,
            force,
        } => {
            if bin {
                commands::new::run_bin(path.as_deref(), ci, force, json)
            } else {
                if ci {
                    anyhow::bail!("--ci requires --bin (CI/CD setup is for binary skill projects)");
                }
                commands::init::run(&target, force, json)
            }
        }
```

- [ ] **Step 3: Update `run_bin` signature and body in `src/commands/new.rs`**

Change the signature to accept `ci: bool` and call `setup_ci` when true:

```rust
pub fn run_bin(path: Option<&str>, ci: bool, force: bool, json: bool) -> anyhow::Result<()> {
    let target_dir = match path {
        Some(p) => {
            let dir = resolve_path(p)?;
            if !dir.exists() {
                std::fs::create_dir_all(&dir)?;
            }
            dir
        }
        None => std::env::current_dir()?,
    };

    let dir_name = target_dir
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("my-skill");
    let name = {
        let s = slugify(dir_name);
        if s.is_empty() {
            "my-skill".to_string()
        } else {
            s
        }
    };

    scaffold_bin_project(&target_dir, &name)?;
    write_skill_md(&target_dir, &name, true, force)?;

    let ci_files = if ci {
        Some(super::ci::setup_ci(&target_dir, &name, force)?)
    } else {
        None
    };

    if json {
        crate::json::print_success(serde_json::json!({
            "name": name,
            "path": target_dir.display().to_string(),
            "binary": true,
            "ci": ci_files.is_some(),
        }));
        return Ok(());
    }

    println!("Created binary skill project in {}", target_dir.display());
    println!("  cargo build              -- compile the binary");
    println!("  cargo run -- self skill  -- test the skill subcommand");
    if let Some(files) = ci_files {
        println!();
        println!("CI/CD workflows:");
        for f in &files {
            println!("  {f}");
        }
    }
    Ok(())
}
```

- [ ] **Step 4: Run existing tests to verify nothing broke**

Run: `cargo nextest run -E 'test(init_bin)'`
Expected: Both `init_bin_creates_cargo_project_and_skill_md` and `init_bin_in_current_dir` PASS.

- [ ] **Step 5: Commit**

```bash
git add src/main.rs src/commands/new.rs
git commit -m "feat: add --ci flag to ion init --bin for CI/CD scaffolding"
```

---

### Task 3: Wire standalone `ion ci` command

**Files:**
- Modify: `src/main.rs` (add `Ci` variant to `Commands` enum and dispatch)

- [ ] **Step 1: Add `Ci` command to `Commands` enum in `src/main.rs`**

Add after the `Init` variant:

```rust
    /// Set up GitHub Actions CI/CD for a binary skill project
    Ci {
        /// Overwrite existing workflow files
        #[arg(long)]
        force: bool,
    },
```

- [ ] **Step 2: Add dispatch in the match block**

Add after the Init dispatch arm:

```rust
        Commands::Ci { force } => commands::ci::run(force, json),
```

- [ ] **Step 3: Verify it compiles and help works**

Run: `cargo build && cargo run -- ci --help`
Expected: Shows help text "Set up GitHub Actions CI/CD for a binary skill project"

- [ ] **Step 4: Commit**

```bash
git add src/main.rs
git commit -m "feat: add ion ci command for standalone CI/CD setup"
```

---

## Chunk 2: Integration Tests

### Task 4: Integration tests

**Files:**
- Create: `tests/ci_integration.rs`

- [ ] **Step 1: Write integration tests**

```rust
// tests/ci_integration.rs

use std::process::Command;

fn ion_cmd() -> Command {
    Command::new(env!("CARGO_BIN_EXE_ion"))
}

#[test]
fn ci_help_is_exposed() {
    let output = ion_cmd().args(["ci", "--help"]).output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(output.status.success());
    assert!(stdout.contains("CI/CD"));
}

#[test]
fn init_bin_ci_creates_workflows() {
    let base = tempfile::tempdir().unwrap();
    let target = base.path().join("my-ci-skill");
    std::fs::create_dir(&target).unwrap();

    let output = ion_cmd()
        .args(["init", "--bin", "--ci", target.to_str().unwrap()])
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(output.status.success(), "stdout={stdout}\nstderr={stderr}");

    // Binary project files
    assert!(target.join("Cargo.toml").exists());
    assert!(target.join("src/main.rs").exists());
    assert!(target.join("SKILL.md").exists());

    // CI/CD files
    assert!(target.join(".github/workflows/ci.yml").exists());
    assert!(target.join(".github/workflows/release.yml").exists());
    assert!(target.join(".github/workflows/release-plz.yml").exists());
    assert!(target.join("release-plz.toml").exists());

    // release.yml should reference the binary name
    let release = std::fs::read_to_string(target.join(".github/workflows/release.yml")).unwrap();
    assert!(
        release.contains("my-ci-skill-${VERSION}"),
        "release.yml should use the binary name in asset packaging"
    );
    assert!(
        !release.contains("{name}"),
        "release.yml should not contain unsubstituted placeholders"
    );
}

#[test]
fn init_bin_without_ci_has_no_workflows() {
    let base = tempfile::tempdir().unwrap();
    let target = base.path().join("no-ci-skill");
    std::fs::create_dir(&target).unwrap();

    let output = ion_cmd()
        .args(["init", "--bin", target.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(output.status.success());

    assert!(!target.join(".github").exists());
    assert!(!target.join("release-plz.toml").exists());
}

#[test]
fn ci_standalone_requires_cargo_toml() {
    let dir = tempfile::tempdir().unwrap();

    let output = ion_cmd()
        .args(["ci"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Cargo.toml"));
}

#[test]
fn ci_standalone_creates_workflows() {
    let base = tempfile::tempdir().unwrap();
    let target = base.path().join("existing-skill");
    std::fs::create_dir(&target).unwrap();

    // First scaffold a binary project
    let output = ion_cmd()
        .args(["init", "--bin", target.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(output.status.success());

    // Now run `ion ci` in that project
    let output = ion_cmd()
        .args(["ci"])
        .current_dir(&target)
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(output.status.success(), "stdout={stdout}\nstderr={stderr}");

    assert!(target.join(".github/workflows/ci.yml").exists());
    assert!(target.join(".github/workflows/release.yml").exists());
    assert!(target.join(".github/workflows/release-plz.yml").exists());
    assert!(target.join("release-plz.toml").exists());

    // Verify the binary name was read from Cargo.toml
    let release = std::fs::read_to_string(target.join(".github/workflows/release.yml")).unwrap();
    assert!(release.contains("existing-skill-${VERSION}"));
}

#[test]
fn ci_standalone_errors_without_force_if_exists() {
    let base = tempfile::tempdir().unwrap();
    let target = base.path().join("dup-ci-skill");
    std::fs::create_dir(&target).unwrap();

    // Scaffold with CI
    let output = ion_cmd()
        .args(["init", "--bin", "--ci", target.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(output.status.success());

    // Running `ion ci` again should fail
    let output = ion_cmd()
        .args(["ci"])
        .current_dir(&target)
        .output()
        .unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("already exists"));
}

#[test]
fn ci_standalone_force_overwrites() {
    let base = tempfile::tempdir().unwrap();
    let target = base.path().join("force-ci-skill");
    std::fs::create_dir(&target).unwrap();

    // Scaffold with CI
    let output = ion_cmd()
        .args(["init", "--bin", "--ci", target.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(output.status.success());

    // Running `ion ci --force` should succeed
    let output = ion_cmd()
        .args(["ci", "--force"])
        .current_dir(&target)
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(output.status.success(), "stdout={stdout}\nstderr={stderr}");
}

#[test]
fn ci_flag_without_bin_errors() {
    let dir = tempfile::tempdir().unwrap();

    let output = ion_cmd()
        .args(["init", "--ci"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("--bin"));
}

#[test]
fn ci_json_output() {
    let base = tempfile::tempdir().unwrap();
    let target = base.path().join("json-ci-skill");
    std::fs::create_dir(&target).unwrap();

    // Scaffold project first
    let output = ion_cmd()
        .args(["init", "--bin", target.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(output.status.success());

    // Run ci with --json
    let output = ion_cmd()
        .args(["--json", "ci"])
        .current_dir(&target)
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(output.status.success(), "stdout={stdout}\nstderr={stderr}");

    let parsed: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(parsed["data"]["name"], "json-ci-skill");
    assert!(parsed["data"]["files"].is_array());
}
```

- [ ] **Step 2: Run integration tests**

Run: `cargo nextest run -E 'binary(ci_integration)'`
Expected: All 10 tests PASS.

- [ ] **Step 3: Run the full pre-commit checklist**

```bash
cargo fmt --all
cargo clippy --all-targets --all-features -- -D warnings
cargo nextest run
```

Expected: All three pass with no errors.

- [ ] **Step 4: Commit**

```bash
git add tests/ci_integration.rs
git commit -m "test: add integration tests for CI/CD setup"
```

---

## Design Notes

### Why these four files?

| File | Purpose |
|------|---------|
| `ci.yml` | Runs fmt/clippy/test on every push and PR — catches regressions early |
| `release.yml` | Builds binaries for 4 targets on `v*` tag push — produces GitHub Release assets in Ion's expected `{name}-{version}-{target}.tar.gz` format |
| `release-plz.yml` | Automates version bumps via conventional commits — creates release PRs and tags on merge |
| `release-plz.toml` | Configures `publish = false` (binary skills aren't crates.io packages) and sets up a clean changelog format |

### Asset naming compatibility

The release workflow produces assets named `{name}-{version}-{target}.tar.gz`, matching the pattern Ion's `install_binary_from_github()` expects. This means `ion add --bin owner/repo` will work out of the box for skills that use these workflows.

### Why `cargo test` instead of `cargo nextest`?

The generated CI uses `cargo test` rather than `cargo nextest run` because downstream binary skill developers may not have nextest installed. `cargo test` is zero-setup and works everywhere. Users who prefer nextest can easily modify the generated workflow.
