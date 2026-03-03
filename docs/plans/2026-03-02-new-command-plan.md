# `ion new` Command Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Rename `ion init` to `ion new` and add `--collection` flag for scaffolding multi-skill collection projects.

**Architecture:** Rename the existing `init` module/command to `new`, add a `--collection` flag that's mutually exclusive with `--bin`, and add a collection code path that creates `skills/` + `README.md` instead of `SKILL.md`.

**Tech Stack:** Rust, clap (CLI), anyhow (errors), tempfile (tests)

---

### Task 1: Rename `init` module to `new`

**Files:**
- Rename: `src/commands/init.rs` → `src/commands/new.rs`
- Modify: `src/commands/mod.rs`
- Modify: `src/main.rs`
- Rename: `tests/init_integration.rs` → `tests/new_integration.rs`

**Step 1: Rename the source file**

```bash
git mv src/commands/init.rs src/commands/new.rs
```

**Step 2: Update `src/commands/mod.rs`**

Change line 4 from:
```rust
pub mod init;
```
to:
```rust
pub mod new;
```

**Step 3: Update `src/main.rs` — rename the enum variant**

Change the `Init` variant (lines 72-83) to:

```rust
    /// Create a new skill or skill collection
    New {
        /// Target directory (default: current directory)
        #[arg(long)]
        path: Option<String>,
        /// Also run `cargo init --bin` to scaffold a Rust CLI project
        #[arg(long)]
        bin: bool,
        /// Overwrite existing files
        #[arg(long)]
        force: bool,
    },
```

**Step 4: Update `src/main.rs` — rename the match arm**

Change line 109 from:
```rust
        Commands::Init { path, bin, force } => commands::init::run(path.as_deref(), bin, force),
```
to:
```rust
        Commands::New { path, bin, force } => commands::new::run(path.as_deref(), bin, false, force),
```

Note: the third argument `false` is for the `collection` flag we'll add in Task 2. Add it now so it compiles after we update the function signature.

**Step 5: Update `src/commands/new.rs` — update function signature**

Change the `run` function signature (line 70) from:
```rust
pub fn run(path: Option<&str>, bin: bool, force: bool) -> anyhow::Result<()> {
```
to:
```rust
pub fn run(path: Option<&str>, bin: bool, collection: bool, force: bool) -> anyhow::Result<()> {
```

Add a conflict check as the first line inside the function:

```rust
    if collection && bin {
        anyhow::bail!("Cannot combine --collection with --bin");
    }
```

**Step 6: Rename integration test file**

```bash
git mv tests/init_integration.rs tests/new_integration.rs
```

**Step 7: Update test file — change all `"init"` args to `"new"`**

In `tests/new_integration.rs`, replace every occurrence of `"init"` in the `args` arrays with `"new"`. Also update the help test assertion:

- `init_help_is_exposed`: change args from `["init", "--help"]` to `["new", "--help"]` and assertion from `"Initialize a new skill"` to `"Create a new skill"`
- `init_creates_skill_md_in_current_dir`: change args from `["init"]` to `["new"]`
- `init_with_path_creates_skill_md_in_specified_dir`: change args from `["init", "--path", ...]` to `["new", "--path", ...]`
- `init_errors_if_skill_md_exists`: change args from `["init"]` to `["new"]`
- `init_force_overwrites_existing_skill_md`: change args from `["init", "--force"]` to `["new", "--force"]`
- `init_bin_creates_cargo_project_and_skill_md`: change args from `["init", "--bin", "--path", ...]` to `["new", "--bin", "--path", ...]`

Optionally rename the test functions from `init_*` to `new_*` for consistency, but not required.

**Step 8: Run tests to verify the rename**

Run: `cargo test`
Expected: All tests pass. The behavior is identical, just the command name changed.

**Step 9: Commit**

```bash
git add -A
git commit -m "refactor: rename ion init to ion new"
```

---

### Task 2: Add `--collection` flag to CLI

**Files:**
- Modify: `src/main.rs`

**Step 1: Add the `--collection` flag to the `New` variant**

In `src/main.rs`, update the `New` variant to add the `collection` field. The full variant becomes:

```rust
    /// Create a new skill or skill collection
    New {
        /// Target directory (default: current directory)
        #[arg(long)]
        path: Option<String>,
        /// Also run `cargo init --bin` to scaffold a Rust CLI project
        #[arg(long)]
        bin: bool,
        /// Create a multi-skill collection with a skills/ directory
        #[arg(long)]
        collection: bool,
        /// Overwrite existing files
        #[arg(long)]
        force: bool,
    },
```

**Step 2: Update the match arm to pass `collection`**

Change the match arm from:
```rust
        Commands::New { path, bin, force } => commands::new::run(path.as_deref(), bin, false, force),
```
to:
```rust
        Commands::New { path, bin, collection, force } => commands::new::run(path.as_deref(), bin, collection, force),
```

**Step 3: Run tests to verify**

Run: `cargo test`
Expected: All tests pass. The new flag defaults to `false` so existing behavior is unchanged.

**Step 4: Commit**

```bash
git add src/main.rs
git commit -m "feat: add --collection flag to ion new CLI"
```

---

### Task 3: Write failing tests for collection mode

**Files:**
- Modify: `tests/new_integration.rs`

**Step 1: Write the failing tests**

Add these tests to `tests/new_integration.rs`:

```rust
#[test]
fn new_collection_creates_skills_dir_and_readme() {
    let dir = tempfile::tempdir().unwrap();

    let output = ion_cmd()
        .args(["new", "--collection"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(output.status.success(), "stdout={stdout}\nstderr={stderr}");

    assert!(dir.path().join("skills").is_dir(), "skills/ directory should be created");
    assert!(dir.path().join("README.md").exists(), "README.md should be created");

    let readme = std::fs::read_to_string(dir.path().join("README.md")).unwrap();
    assert!(readme.contains("collection of skills"));
    assert!(readme.contains("ion new"));
}

#[test]
fn new_collection_with_path_creates_in_specified_dir() {
    let base = tempfile::tempdir().unwrap();
    let target = base.path().join("my-collection");

    let output = ion_cmd()
        .args(["new", "--collection", "--path", target.to_str().unwrap()])
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(output.status.success(), "stdout={stdout}\nstderr={stderr}");

    assert!(target.join("skills").is_dir());
    assert!(target.join("README.md").exists());

    let readme = std::fs::read_to_string(target.join("README.md")).unwrap();
    assert!(readme.contains("My Collection"));
}

#[test]
fn new_collection_errors_if_readme_exists() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("README.md"), "existing readme").unwrap();

    let output = ion_cmd()
        .args(["new", "--collection"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("already exists"));
    assert!(stderr.contains("--force"));

    let content = std::fs::read_to_string(dir.path().join("README.md")).unwrap();
    assert_eq!(content, "existing readme");
}

#[test]
fn new_collection_force_overwrites_readme() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("README.md"), "old readme").unwrap();

    let output = ion_cmd()
        .args(["new", "--collection", "--force"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(output.status.success(), "stdout={stdout}\nstderr={stderr}");

    let content = std::fs::read_to_string(dir.path().join("README.md")).unwrap();
    assert!(content.contains("collection of skills"));
    assert!(!content.contains("old readme"));
}

#[test]
fn new_collection_and_bin_errors() {
    let dir = tempfile::tempdir().unwrap();

    let output = ion_cmd()
        .args(["new", "--collection", "--bin"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Cannot combine"));
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo test --test new_integration`
Expected: The 5 new tests fail (collection mode not implemented yet). Existing tests still pass.

**Step 3: Commit**

```bash
git add tests/new_integration.rs
git commit -m "test: add failing tests for ion new --collection"
```

---

### Task 4: Implement collection mode

**Files:**
- Modify: `src/commands/new.rs`

**Step 1: Add the collection README template**

Add this constant after the existing `DEFAULT_TEMPLATE` in `src/commands/new.rs`:

```rust
const COLLECTION_README_TEMPLATE: &str = r#"# {title}

A collection of skills for AI agents.

## Skills

Add skills with:

```bash
ion new --path skills/<skill-name>
```
"#;
```

**Step 2: Add the collection code path**

In the `run` function, after the conflict check (`if collection && bin`), add an early return for collection mode. Insert this right after the conflict check and the `target_dir` resolution and directory creation (keep those), but before the SKILL.md logic:

Replace the entire `run` function body with:

```rust
pub fn run(path: Option<&str>, bin: bool, collection: bool, force: bool) -> anyhow::Result<()> {
    if collection && bin {
        anyhow::bail!("Cannot combine --collection with --bin");
    }

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

    if !target_dir.exists() {
        std::fs::create_dir_all(&target_dir)?;
    }

    if collection {
        return run_collection(&target_dir, force);
    }

    let skill_md_path = target_dir.join("SKILL.md");

    if skill_md_path.exists() && !force {
        anyhow::bail!(
            "SKILL.md already exists in {}. Use --force to overwrite.",
            target_dir.display()
        );
    }

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
    let title = titleize(&name);

    let content = DEFAULT_TEMPLATE
        .replace("{name}", &name)
        .replace("{title}", &title);
    std::fs::write(&skill_md_path, content)?;

    println!("Created SKILL.md in {}", target_dir.display());
    Ok(())
}

fn run_collection(target_dir: &std::path::Path, force: bool) -> anyhow::Result<()> {
    let readme_path = target_dir.join("README.md");

    if readme_path.exists() && !force {
        anyhow::bail!(
            "README.md already exists in {}. Use --force to overwrite.",
            target_dir.display()
        );
    }

    std::fs::create_dir_all(target_dir.join("skills"))?;

    let dir_name = target_dir
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("my-skills");
    let name = {
        let s = slugify(dir_name);
        if s.is_empty() {
            "my-skills".to_string()
        } else {
            s
        }
    };
    let title = titleize(&name);

    let content = COLLECTION_README_TEMPLATE
        .replace("{title}", &title);
    std::fs::write(&readme_path, content)?;

    println!("Created skill collection in {}", target_dir.display());
    Ok(())
}
```

**Step 3: Run all tests**

Run: `cargo test`
Expected: All tests pass — both existing renamed tests and new collection tests.

**Step 4: Commit**

```bash
git add src/commands/new.rs
git commit -m "feat: implement ion new --collection for multi-skill projects"
```
