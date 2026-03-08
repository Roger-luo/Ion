# Shell Completion Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add `ion completion <shell>` command to generate shell auto-completion scripts.

**Architecture:** Uses `clap_complete` crate to generate completions from the existing clap-derived CLI definition. Top-level command taking a shell name as a required positional argument, outputs completion script to stdout.

**Tech Stack:** clap_complete 4.x, clap 4.x (existing)

---

### Task 1: Add clap_complete dependency

**Files:**
- Modify: `Cargo.toml:8` (dependencies section)

**Step 1: Add the dependency**

In `Cargo.toml`, add `clap_complete` to `[dependencies]`:

```toml
clap_complete = "4"
```

Place it after the `clap` line.

**Step 2: Verify it compiles**

Run: `cargo check`
Expected: compiles with no errors

**Step 3: Commit**

```bash
git add Cargo.toml Cargo.lock
git commit -m "build: add clap_complete dependency"
```

---

### Task 2: Write failing integration test

**Files:**
- Create: `tests/completion_integration.rs`

**Step 1: Write the test file**

```rust
use std::process::Command;

fn ion_cmd() -> Command {
    Command::new(env!("CARGO_BIN_EXE_ion"))
}

#[test]
fn completion_bash_produces_output() {
    let output = ion_cmd()
        .args(["completion", "bash"])
        .output()
        .expect("failed to run ion");
    assert!(output.status.success(), "exit code was not 0");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(!stdout.is_empty(), "completion output was empty");
    assert!(stdout.contains("ion"), "completion should reference the binary name");
}

#[test]
fn completion_zsh_produces_output() {
    let output = ion_cmd()
        .args(["completion", "zsh"])
        .output()
        .expect("failed to run ion");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(!stdout.is_empty());
}

#[test]
fn completion_fish_produces_output() {
    let output = ion_cmd()
        .args(["completion", "fish"])
        .output()
        .expect("failed to run ion");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(!stdout.is_empty());
}

#[test]
fn completion_invalid_shell_fails() {
    let output = ion_cmd()
        .args(["completion", "nushell"])
        .output()
        .expect("failed to run ion");
    assert!(!output.status.success(), "should reject unknown shells");
}

#[test]
fn completion_missing_shell_fails() {
    let output = ion_cmd()
        .args(["completion"])
        .output()
        .expect("failed to run ion");
    assert!(!output.status.success(), "should require a shell argument");
}
```

**Step 2: Run the tests to verify they fail**

Run: `cargo test --test completion_integration`
Expected: compilation error — `completion` is not a recognized subcommand yet

**Step 3: Commit**

```bash
git add tests/completion_integration.rs
git commit -m "test: add failing integration tests for completion command"
```

---

### Task 3: Implement completion command and wire it up

**Files:**
- Create: `src/commands/completion.rs`
- Modify: `src/commands/mod.rs:1` (add module declaration)
- Modify: `src/main.rs:15-16` (add Commands variant)
- Modify: `src/main.rs:170` (add match arm)

**Step 1: Create `src/commands/completion.rs`**

```rust
use std::io;

pub fn run(shell: clap_complete::Shell, mut cmd: clap::Command) {
    clap_complete::generate(shell, &mut cmd, "ion", &mut io::stdout());
}
```

**Step 2: Register the module in `src/commands/mod.rs`**

Add this line (alphabetical order, after `config`):

```rust
pub mod completion;
```

**Step 3: Add the `Completion` variant to `Commands` enum in `src/main.rs`**

Add this variant to the `Commands` enum, after the `Config` variant:

```rust
    /// Generate shell completion scripts
    Completion {
        /// Shell to generate completions for
        shell: clap_complete::Shell,
    },
```

**Step 4: Add the match arm in `main()` in `src/main.rs`**

In the `match cli.command` block, add after the `Config` arm:

```rust
        Commands::Completion { shell } => {
            commands::completion::run(shell, Cli::command());
            Ok(())
        }
```

Note: `Cli::command()` requires `use clap::CommandFactory;` — but since we already have `use clap::Parser` and `Parser` is a supertrait of `CommandFactory`, we need to add `CommandFactory` to the import. Change line 1:

```rust
use clap::{CommandFactory, Parser, Subcommand};
```

**Step 5: Run the tests**

Run: `cargo test --test completion_integration`
Expected: all 5 tests pass

**Step 6: Also run the full test suite**

Run: `cargo test`
Expected: all tests pass, no regressions

**Step 7: Commit**

```bash
git add src/commands/completion.rs src/commands/mod.rs src/main.rs
git commit -m "feat: add ion completion command for shell auto-completion"
```

---

### Task 4: Manual smoke test

**Step 1: Try each shell**

Run each and verify non-empty, sensible output:

```bash
cargo run -- completion bash | head -5
cargo run -- completion zsh | head -5
cargo run -- completion fish | head -5
```

**Step 2: Verify error cases**

```bash
cargo run -- completion         # should error: missing required arg
cargo run -- completion nushell # should error: invalid value
```

No commit needed for this task.
