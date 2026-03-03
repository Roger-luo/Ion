# `ion init` Command Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add an `ion init` command that scaffolds new skill projects with a SKILL.md template.

**Architecture:** New `src/commands/init.rs` command handler with embedded templates, a name-sanitization helper, and optional `cargo init --bin` delegation. Follows the existing `commands/<cmd>::run()` pattern used by all other ion commands.

**Tech Stack:** Rust, clap (derive macros), std::process::Command (for cargo delegation), tempfile (tests)

---

### Task 1: Add `init` module skeleton and wire up CLI

**Files:**
- Create: `src/commands/init.rs`
- Modify: `src/commands/mod.rs`
- Modify: `src/main.rs`

**Step 1: Write the failing integration test**

Create `tests/init_integration.rs`:

```rust
use std::process::Command;

fn ion_cmd() -> Command {
    Command::new(env!("CARGO_BIN_EXE_ion"))
}

#[test]
fn init_help_is_exposed() {
    let output = ion_cmd().args(["init", "--help"]).output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(output.status.success());
    assert!(stdout.contains("Initialize a new skill"));
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test --test init_integration init_help_is_exposed`
Expected: FAIL — `init` is not a recognized subcommand

**Step 3: Write minimal implementation**

Add to `src/commands/mod.rs`:

```rust
pub mod init;
```

Create `src/commands/init.rs`:

```rust
use std::path::PathBuf;

pub fn run(_path: Option<&str>, _bin: bool, _force: bool) -> anyhow::Result<()> {
    todo!("init command not yet implemented")
}
```

Add to the `Commands` enum in `src/main.rs`:

```rust
    /// Initialize a new skill project
    Init {
        /// Target directory (default: current directory)
        #[arg(long)]
        path: Option<String>,
        /// Also run `cargo init --bin` to scaffold a Rust CLI project
        #[arg(long)]
        bin: bool,
        /// Overwrite existing SKILL.md
        #[arg(long)]
        force: bool,
    },
```

Add the match arm in `main()`:

```rust
        Commands::Init { path, bin, force } => commands::init::run(path.as_deref(), bin, force),
```

**Step 4: Run test to verify it passes**

Run: `cargo test --test init_integration init_help_is_exposed`
Expected: PASS

**Step 5: Commit**

```bash
git add src/commands/init.rs src/commands/mod.rs src/main.rs tests/init_integration.rs
git commit -m "feat(init): wire up ion init subcommand skeleton"
```

---

### Task 2: Implement name derivation from directory

**Files:**
- Modify: `src/commands/init.rs`

**Step 1: Write the failing test**

Add unit tests inside `src/commands/init.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slugify_lowercase() {
        assert_eq!(slugify("my-skill"), "my-skill");
    }

    #[test]
    fn slugify_spaces_and_caps() {
        assert_eq!(slugify("My Cool Skill"), "my-cool-skill");
    }

    #[test]
    fn slugify_underscores() {
        assert_eq!(slugify("my_cool_skill"), "my-cool-skill");
    }

    #[test]
    fn slugify_special_chars() {
        assert_eq!(slugify("skill@v2.0!"), "skill-v2-0");
    }

    #[test]
    fn slugify_leading_trailing_hyphens() {
        assert_eq!(slugify("--my-skill--"), "my-skill");
    }

    #[test]
    fn slugify_consecutive_hyphens() {
        assert_eq!(slugify("my---skill"), "my-skill");
    }
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p ion --lib commands::init::tests`
Expected: FAIL — `slugify` not defined

**Step 3: Write minimal implementation**

Add to `src/commands/init.rs`:

```rust
fn slugify(name: &str) -> String {
    let slug: String = name
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect();

    // Collapse consecutive hyphens and trim leading/trailing hyphens
    let mut result = String::new();
    for ch in slug.chars() {
        if ch == '-' {
            if !result.ends_with('-') {
                result.push('-');
            }
        } else {
            result.push(ch);
        }
    }
    result.trim_matches('-').to_string()
}
```

**Step 4: Run test to verify it passes**

Run: `cargo test -p ion --lib commands::init::tests`
Expected: PASS — all 6 tests pass

**Step 5: Commit**

```bash
git add src/commands/init.rs
git commit -m "feat(init): add slugify helper for skill name derivation"
```

---

### Task 3: Implement default SKILL.md creation

**Files:**
- Modify: `src/commands/init.rs`
- Modify: `tests/init_integration.rs`

**Step 1: Write the failing integration test**

Add to `tests/init_integration.rs`:

```rust
#[test]
fn init_creates_skill_md_in_current_dir() {
    let dir = tempfile::tempdir().unwrap();

    let output = ion_cmd()
        .args(["init"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(output.status.success(), "stdout={stdout}\nstderr={stderr}");

    let skill_md = dir.path().join("SKILL.md");
    assert!(skill_md.exists(), "SKILL.md should be created");

    let content = std::fs::read_to_string(&skill_md).unwrap();
    // Name should be derived from temp dir name
    assert!(content.contains("name:"));
    assert!(content.contains("description:"));
    assert!(content.contains("## Overview"));
}

#[test]
fn init_with_path_creates_skill_md_in_specified_dir() {
    let base = tempfile::tempdir().unwrap();
    let target = base.path().join("my-new-skill");

    let output = ion_cmd()
        .args(["init", "--path", target.to_str().unwrap()])
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(output.status.success(), "stdout={stdout}\nstderr={stderr}");

    let skill_md = target.join("SKILL.md");
    assert!(skill_md.exists(), "SKILL.md should be created at --path");

    let content = std::fs::read_to_string(&skill_md).unwrap();
    assert!(content.contains("name: my-new-skill"));
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test --test init_integration`
Expected: FAIL — `run()` still contains `todo!()`

**Step 3: Write implementation**

Replace `src/commands/init.rs` `run` function body:

```rust
use std::path::PathBuf;

const DEFAULT_TEMPLATE: &str = r#"---
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
"#;

fn titleize(slug: &str) -> String {
    slug.split('-')
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                None => String::new(),
                Some(c) => c.to_uppercase().to_string() + chars.as_str(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

pub fn run(path: Option<&str>, bin: bool, force: bool) -> anyhow::Result<()> {
    let target_dir = match path {
        Some(p) => {
            let p = PathBuf::from(p);
            if p.is_absolute() {
                p
            } else {
                std::env::current_dir()?.join(p)
            }
        }
        None => std::env::current_dir()?,
    };

    // Create directory if it doesn't exist
    if !target_dir.exists() {
        std::fs::create_dir_all(&target_dir)?;
    }

    let skill_md_path = target_dir.join("SKILL.md");

    // Check for existing SKILL.md
    if skill_md_path.exists() && !force {
        anyhow::bail!(
            "SKILL.md already exists in {}. Use --force to overwrite.",
            target_dir.display()
        );
    }

    // Run cargo init --bin if requested
    if bin {
        let status = std::process::Command::new("cargo")
            .args(["init", "--bin"])
            .current_dir(&target_dir)
            .status()
            .map_err(|e| anyhow::anyhow!("Failed to run cargo: {e}. Is the Rust toolchain installed?"))?;

        if !status.success() {
            anyhow::bail!("cargo init --bin failed");
        }
    }

    // Derive skill name from directory name
    let dir_name = target_dir
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("my-skill");
    let name = slugify(dir_name);
    let title = titleize(&name);

    // Write SKILL.md
    let content = DEFAULT_TEMPLATE
        .replace("{name}", &name)
        .replace("{title}", &title);
    std::fs::write(&skill_md_path, content)?;

    println!("Created SKILL.md in {}", target_dir.display());
    Ok(())
}
```

**Step 4: Run test to verify it passes**

Run: `cargo test --test init_integration`
Expected: PASS

**Step 5: Commit**

```bash
git add src/commands/init.rs tests/init_integration.rs
git commit -m "feat(init): implement default SKILL.md creation with template"
```

---

### Task 4: Implement --force and error handling

**Files:**
- Modify: `tests/init_integration.rs`

**Step 1: Write the failing integration tests**

Add to `tests/init_integration.rs`:

```rust
#[test]
fn init_errors_if_skill_md_exists() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("SKILL.md"), "existing content").unwrap();

    let output = ion_cmd()
        .args(["init"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("already exists"));
    assert!(stderr.contains("--force"));

    // Verify original file is untouched
    let content = std::fs::read_to_string(dir.path().join("SKILL.md")).unwrap();
    assert_eq!(content, "existing content");
}

#[test]
fn init_force_overwrites_existing_skill_md() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("SKILL.md"), "old content").unwrap();

    let output = ion_cmd()
        .args(["init", "--force"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(output.status.success(), "stdout={stdout}\nstderr={stderr}");

    let content = std::fs::read_to_string(dir.path().join("SKILL.md")).unwrap();
    assert!(content.contains("## Overview"), "should have new template content");
    assert!(!content.contains("old content"));
}
```

**Step 2: Run tests to verify they pass**

These should already pass since the logic was implemented in Task 3. Verify:

Run: `cargo test --test init_integration`
Expected: PASS — all tests pass

**Step 3: Commit**

```bash
git add tests/init_integration.rs
git commit -m "test(init): add tests for --force flag and existing SKILL.md error"
```

---

### Task 5: Implement --bin flag with cargo delegation

**Files:**
- Modify: `tests/init_integration.rs`

**Step 1: Write the failing integration test**

Add to `tests/init_integration.rs`:

```rust
#[test]
fn init_bin_creates_cargo_project_and_skill_md() {
    let base = tempfile::tempdir().unwrap();
    let target = base.path().join("my-bin-skill");
    std::fs::create_dir(&target).unwrap();

    let output = ion_cmd()
        .args(["init", "--bin", "--path", target.to_str().unwrap()])
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(output.status.success(), "stdout={stdout}\nstderr={stderr}");

    // SKILL.md created
    assert!(target.join("SKILL.md").exists());

    // Cargo project created
    assert!(target.join("Cargo.toml").exists());
    assert!(target.join("src/main.rs").exists());

    let content = std::fs::read_to_string(target.join("SKILL.md")).unwrap();
    assert!(content.contains("name: my-bin-skill"));
}
```

**Step 2: Run test to verify it passes**

This should already pass since cargo delegation was implemented in Task 3. Verify:

Run: `cargo test --test init_integration init_bin`
Expected: PASS

**Step 3: Commit**

```bash
git add tests/init_integration.rs
git commit -m "test(init): add integration test for --bin flag with cargo delegation"
```

---

### Task 6: Update help test and final verification

**Files:**
- Modify: `tests/integration.rs`

**Step 1: Update the help_shows_all_commands test**

In `tests/integration.rs`, add `init` to the help assertion:

```rust
    assert!(stdout.contains("init"));
```

**Step 2: Run full test suite**

Run: `cargo test`
Expected: All tests pass

**Step 3: Commit**

```bash
git add tests/integration.rs
git commit -m "test: add init to help_shows_all_commands assertion"
```
