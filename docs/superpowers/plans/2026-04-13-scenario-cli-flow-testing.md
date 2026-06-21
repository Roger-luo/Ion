# Scenario CLI Flow Testing Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add generic `scenario` crate APIs that make interactive CLI flow tests shorter, less flaky, and more screen-oriented, using the `autotune2` test suite as the reference workload.

**Architecture:** Extend the existing `Session` and `ProjectBuilder` types instead of introducing a new DSL. Build new helpers on top of the current PTY/session primitives so existing tests keep working, while new tests can express user actions, visible-screen assertions, and repo-backed fixtures more directly.

**Tech Stack:** Rust 2024, `scenario` crate, `portable-pty`, `regex`, `tempfile`, shell `git`, crate integration tests under `crates/scenario/tests/`

---

## File Structure

- Modify: `crates/scenario/src/error.rs`
  Add one post-build setup error variant for `ProjectBuilder` git actions and reuse existing timeout-style errors where possible.
- Modify: `crates/scenario/src/project.rs`
  Add generic project setup actions (`setup_git`, `git_user`, `initial_commit`) and run them after materializing files.
- Modify: `crates/scenario/src/session.rs`
  Add input ergonomics (`send_keys`, `press`, `enter`, `ctrl_c`), visible-screen string helpers, screen expectations, and a screen stabilization helper.
- Modify: `crates/scenario/src/lib.rs`
  Export any new public helper types if needed, especially `SessionConfig`.
- Modify: `crates/scenario/src/scenario.rs`
  Add a small reusable `SessionConfig` profile type and `Scenario::session_config`.
- Test: `crates/scenario/tests/project.rs`
  Extend project tests for repo-backed setup actions and failure cases.
- Create: `crates/scenario/tests/session_flow.rs`
  Add public API integration tests for input ergonomics, screen assertions, and stable-screen waiting.
- Reference only: `../autotune2/crates/autotune/tests/scenario_init_test.rs`
  Use as the migration target when choosing API names and examples, but do not modify it in this plan.

---

### Task 1: Add Session Input Ergonomics

**Files:**
- Modify: `crates/scenario/src/session.rs`
- Test: `crates/scenario/tests/session_flow.rs`

- [ ] **Step 1: Write the failing integration tests for generic input helpers**

Add these tests to `crates/scenario/tests/session_flow.rs`:

```rust
use std::time::Duration;

use scenario::{Key, Scenario, Terminal};

fn spawn(script: &str) -> scenario::Session {
    Scenario::new("sh")
        .args(["-c", script])
        .terminal(Terminal::pty(80, 24))
        .timeout(Duration::from_secs(5))
        .spawn()
        .unwrap()
}

#[test]
fn send_keys_submits_multiple_terminal_keys_in_order() {
    let mut session = spawn(
        "printf 'ready\\n'; IFS= read -r line; printf '%s' \"$line\" | od -An -tx1",
    );

    session.expect("ready").unwrap();
    session
        .send_keys([Key::Down, Key::Down, Key::Enter])
        .unwrap();
    session.expect_regex(r\"1b\\s+5b\\s+42.*1b\\s+5b\\s+42.*0d\").unwrap();
}

#[test]
fn enter_and_ctrl_c_match_common_terminal_actions() {
    let mut session = spawn(
        \"trap 'printf trapped\\\\n; exit 130' INT; printf ready\\\\n; while :; do sleep 1; done\",
    );

    session.expect(\"ready\").unwrap();
    session.ctrl_c().unwrap();
    session.expect(\"trapped\").unwrap();

    let output = session.wait().unwrap();
    assert_eq!(output.exit_code(), 130);
}
```

- [ ] **Step 2: Run the targeted test to verify it fails**

Run:

```bash
cargo test -p scenario --test session_flow send_keys_submits_multiple_terminal_keys_in_order -- --nocapture
```

Expected: FAIL because `Session` does not yet implement `send_keys`, `enter`, or `ctrl_c`.

- [ ] **Step 3: Implement the minimal session input helpers**

Update `crates/scenario/src/session.rs` by adding these methods near the existing `send_key()` implementation:

```rust
    pub fn send_keys<I>(&mut self, keys: I) -> Result<(), Error>
    where
        I: IntoIterator<Item = Key>,
    {
        for key in keys {
            self.send_key(key)?;
        }
        Ok(())
    }

    pub fn press(&mut self, key: Key) -> Result<(), Error> {
        self.send_key(key)
    }

    pub fn enter(&mut self) -> Result<(), Error> {
        self.send_key(Key::Enter)
    }

    pub fn ctrl_c(&mut self) -> Result<(), Error> {
        self.send_key(Key::CtrlC)
    }
```

- [ ] **Step 4: Run the targeted test to verify it passes**

Run:

```bash
cargo test -p scenario --test session_flow
```

Expected: PASS for the new input-helper tests.

- [ ] **Step 5: Commit**

```bash
git add crates/scenario/src/session.rs crates/scenario/tests/session_flow.rs
git commit -m "feat(scenario): add session input helper methods"
```

---

### Task 2: Add Visible-Screen Assertions and Stabilization

**Files:**
- Modify: `crates/scenario/src/session.rs`
- Test: `crates/scenario/tests/session_flow.rs`

- [ ] **Step 1: Write the failing integration tests for screen assertions**

Append these tests to `crates/scenario/tests/session_flow.rs`:

```rust
#[test]
fn expect_screen_checks_visible_state_after_redraw() {
    let mut session = spawn(
        r#"printf 'Option A'; printf '\r\033[K'; printf '> Option B'; sleep 0.1"#,
    );

    session.expect("> Option B").unwrap();
    session.expect_screen("> Option B").unwrap();
    session.expect_screen_not("Option A").unwrap();
}

#[test]
fn wait_for_screen_stable_observes_settled_terminal_output() {
    let mut session = spawn(
        r#"printf 'loading'; sleep 0.1; printf '\r\033[Kdone'; sleep 0.1"#,
    );

    session.expect("done").unwrap();
    session
        .wait_for_screen_stable(Duration::from_millis(100))
        .unwrap();

    assert_eq!(session.visible_text().lines().next().unwrap_or(""), "done");
}
```

- [ ] **Step 2: Run the targeted test to verify it fails**

Run:

```bash
cargo test -p scenario --test session_flow expect_screen_checks_visible_state_after_redraw -- --nocapture
```

Expected: FAIL because `expect_screen`, `expect_screen_not`, `visible_text`, and `wait_for_screen_stable` do not exist yet.

- [ ] **Step 3: Implement minimal visible-screen APIs**

In `crates/scenario/src/session.rs`, add:

```rust
    pub fn visible_text(&self) -> String {
        self.visible_screen().join("\n")
    }

    pub fn expect_screen(&self, pattern: &str) -> Result<(), Error> {
        let deadline = Instant::now() + self.timeout;
        loop {
            if self.visible_text().contains(pattern) {
                return Ok(());
            }
            if self.is_done() || Instant::now() >= deadline {
                return Err(Error::ExpectTimeout {
                    pattern: pattern.to_string(),
                    timeout: self.timeout,
                    buffer: self.visible_text(),
                });
            }
            thread::sleep(Duration::from_millis(10));
        }
    }

    pub fn expect_screen_regex(&self, pattern: &str) -> Result<(), Error> {
        let re = Regex::new(pattern)?;
        let deadline = Instant::now() + self.timeout;
        loop {
            let visible = self.visible_text();
            if re.is_match(&visible) {
                return Ok(());
            }
            if self.is_done() || Instant::now() >= deadline {
                return Err(Error::ExpectTimeout {
                    pattern: pattern.to_string(),
                    timeout: self.timeout,
                    buffer: visible,
                });
            }
            thread::sleep(Duration::from_millis(10));
        }
    }

    pub fn expect_screen_not(&self, pattern: &str) -> Result<(), Error> {
        self.wait_for_screen_stable(Duration::from_millis(100))?;
        let visible = self.visible_text();
        if visible.contains(pattern) {
            return Err(Error::UnexpectedPattern {
                pattern: pattern.to_string(),
                buffer: visible,
            });
        }
        Ok(())
    }

    pub fn expect_screen_not_regex(&self, pattern: &str) -> Result<(), Error> {
        self.wait_for_screen_stable(Duration::from_millis(100))?;
        let re = Regex::new(pattern)?;
        let visible = self.visible_text();
        if re.is_match(&visible) {
            return Err(Error::UnexpectedPattern {
                pattern: pattern.to_string(),
                buffer: visible,
            });
        }
        Ok(())
    }

    pub fn wait_for_screen_stable(&self, quiet_period: Duration) -> Result<(), Error> {
        let deadline = Instant::now() + self.timeout;
        let mut last = self.visible_text();
        let mut stable_since = Instant::now();

        loop {
            thread::sleep(Duration::from_millis(10));
            let current = self.visible_text();
            if current == last {
                if stable_since.elapsed() >= quiet_period {
                    return Ok(());
                }
            } else {
                last = current;
                stable_since = Instant::now();
            }

            if self.is_done() && self.visible_text() == last && stable_since.elapsed() >= quiet_period {
                return Ok(());
            }

            if Instant::now() >= deadline {
                return Err(Error::ExpectTimeout {
                    pattern: format!("stable screen for {:?}", quiet_period),
                    timeout: self.timeout,
                    buffer: self.visible_text(),
                });
            }
        }
    }
```

- [ ] **Step 4: Run the targeted test to verify it passes**

Run:

```bash
cargo test -p scenario --test session_flow
```

Expected: PASS for both the input-helper tests and the new visible-screen tests.

- [ ] **Step 5: Commit**

```bash
git add crates/scenario/src/session.rs crates/scenario/tests/session_flow.rs
git commit -m "feat(scenario): add screen-based session assertions"
```

---

### Task 3: Add Reusable Session Profiles

**Files:**
- Modify: `crates/scenario/src/scenario.rs`
- Modify: `crates/scenario/src/lib.rs`
- Test: `crates/scenario/tests/session_flow.rs`

- [ ] **Step 1: Write the failing test for reusable session config**

Append this test to `crates/scenario/tests/session_flow.rs`:

```rust
use scenario::{SessionConfig, Terminal};

#[test]
fn session_config_applies_shared_terminal_and_timeout_settings() {
    let config = SessionConfig::pty(90, 20).timeout(Duration::from_secs(3));

    let session = Scenario::new("cat")
        .session_config(&config)
        .spawn()
        .unwrap();

    assert_eq!(session.pty_size(), (20, 90));
}
```

- [ ] **Step 2: Run the targeted test to verify it fails**

Run:

```bash
cargo test -p scenario --test session_flow session_config_applies_shared_terminal_and_timeout_settings -- --nocapture
```

Expected: FAIL because `SessionConfig` and `Scenario::session_config` do not exist.

- [ ] **Step 3: Implement the reusable session profile type**

In `crates/scenario/src/scenario.rs`, add:

```rust
pub struct SessionConfig {
    terminal: Terminal,
    timeout: Duration,
}

impl SessionConfig {
    pub fn pty(cols: u16, rows: u16) -> Self {
        Self {
            terminal: Terminal::pty(cols, rows),
            timeout: Duration::from_secs(30),
        }
    }

    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }
}
```

And add:

```rust
    pub fn session_config(mut self, config: &SessionConfig) -> Self {
        self.terminal = config.terminal.clone();
        self.timeout = config.timeout;
        self
    }
```

Export `SessionConfig` from `crates/scenario/src/lib.rs`:

```rust
pub use scenario::{Scenario, SessionConfig, Terminal};
```

- [ ] **Step 4: Run the targeted test to verify it passes**

Run:

```bash
cargo test -p scenario --test session_flow
```

Expected: PASS including the new shared-config test.

- [ ] **Step 5: Commit**

```bash
git add crates/scenario/src/scenario.rs crates/scenario/src/lib.rs crates/scenario/tests/session_flow.rs
git commit -m "feat(scenario): add reusable session configuration"
```

---

### Task 4: Add Generic Git Setup Actions to ProjectBuilder

**Files:**
- Modify: `crates/scenario/src/error.rs`
- Modify: `crates/scenario/src/project.rs`
- Test: `crates/scenario/tests/project.rs`

- [ ] **Step 1: Write the failing tests for repo-backed project setup**

Add these tests to `crates/scenario/tests/project.rs`:

```rust
#[test]
fn project_setup_git_initializes_a_repository() {
    let project = Project::empty().file("README.md", "hello\n").setup_git().build().unwrap();

    assert!(project.path().join(".git").exists());
}

#[test]
fn project_initial_commit_creates_head_commit() {
    let project = Project::empty()
        .file("README.md", "hello\n")
        .setup_git()
        .git_user("Test", "test@example.com")
        .initial_commit("initial")
        .build()
        .unwrap();

    let output = std::process::Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(project.path())
        .output()
        .unwrap();

    assert!(output.status.success(), "git rev-parse HEAD should succeed");
}
```

- [ ] **Step 2: Run the targeted test to verify it fails**

Run:

```bash
cargo test -p scenario --test project project_initial_commit_creates_head_commit -- --nocapture
```

Expected: FAIL because `ProjectBuilder` does not yet implement `setup_git`, `git_user`, or `initial_commit`.

- [ ] **Step 3: Implement post-build project setup actions**

Add a new error variant to `crates/scenario/src/error.rs`:

```rust
    #[error("project setup step {step} failed: {source}")]
    ProjectSetup {
        step: String,
        source: std::io::Error,
    },
```

In `crates/scenario/src/project.rs`, extend `ProjectBuilder` with fields to track setup requests and add methods:

```rust
    pub fn setup_git(mut self) -> Self {
        self.setup_git = true;
        self
    }

    pub fn git_user(mut self, name: &str, email: &str) -> Self {
        self.git_user = Some((name.to_string(), email.to_string()));
        self
    }

    pub fn initial_commit(mut self, message: &str) -> Self {
        self.initial_commit = Some(message.to_string());
        self
    }
```

After file materialization in `build()` / `build_in()`, run shell commands in order:

```rust
if self.setup_git {
    run_setup_command(path, "git init", &["git", "init"])?;
}
if let Some((name, email)) = &self.git_user {
    run_setup_command(path, "git config user.name", &["git", "config", "user.name", name])?;
    run_setup_command(path, "git config user.email", &["git", "config", "user.email", email])?;
}
if let Some(message) = &self.initial_commit {
    run_setup_command(path, "git add .", &["git", "add", "."])?;
    run_setup_command(path, "git commit", &["git", "commit", "-m", message])?;
}
```

Implement a small helper in the same file:

```rust
fn run_setup_command(path: &Path, step: &str, args: &[&str]) -> Result<(), Error> {
    let output = std::process::Command::new(args[0])
        .args(&args[1..])
        .current_dir(path)
        .output()
        .map_err(|source| Error::ProjectSetup {
            step: step.to_string(),
            source,
        })?;

    if output.status.success() {
        Ok(())
    } else {
        Err(Error::Pty(format!(
            "project setup step {step} failed: {}",
            String::from_utf8_lossy(&output.stderr)
        )))
    }
}
```

- [ ] **Step 4: Run the targeted test to verify it passes**

Run:

```bash
cargo test -p scenario --test project
```

Expected: PASS including the new repository setup tests.

- [ ] **Step 5: Commit**

```bash
git add crates/scenario/src/error.rs crates/scenario/src/project.rs crates/scenario/tests/project.rs
git commit -m "feat(scenario): add project git setup helpers"
```

---

### Task 5: Verify the Full Phase-1 API Surface and Autotune-Style Example

**Files:**
- Modify: `crates/scenario/tests/session_flow.rs`
- Modify: `crates/scenario/src/lib.rs` (docs only if needed)

- [ ] **Step 1: Write the final integration test that mirrors the autotune-style flow**

Add this test to `crates/scenario/tests/session_flow.rs`:

```rust
#[test]
fn screen_helpers_replace_current_output_slicing_for_redraw_flows() {
    let mut session = spawn(
        r#"printf 'What metric?\n> Runtime performance\n  Memory usage';
           sleep 0.1;
           printf '\033[2A\r\033[J';
           printf 'How should we measure?\n> Benchmark runtime\n  Track memory';
           sleep 0.1"#,
    );

    session.expect("How should we measure").unwrap();
    session.wait_for_screen_stable(Duration::from_millis(100)).unwrap();
    session.expect_screen("How should we measure").unwrap();
    session.expect_screen_not("Runtime performance").unwrap();
    session.expect_screen("Benchmark runtime").unwrap();
}
```

- [ ] **Step 2: Run the focused scenario crate test suite**

Run:

```bash
cargo nextest run -p scenario
```

Expected: PASS with all existing and new tests green.

- [ ] **Step 3: Run formatting and lint checks**

Run:

```bash
cargo fmt --all --check
cargo clippy -p scenario --all-targets --all-features -- -D warnings
```

Expected: both commands succeed with no output requiring follow-up changes.

- [ ] **Step 4: Update public docs if needed**

If `crates/scenario/src/lib.rs` module docs need a new example, add a short one using the new APIs:

```rust
//! let mut session = Scenario::new("my-cli")
//!     .session_config(&SessionConfig::pty(100, 30).timeout(Duration::from_secs(5)))
//!     .spawn()
//!     .unwrap();
//! session.send_keys([Key::Down, Key::Enter]).unwrap();
//! session.expect_screen("Done").unwrap();
```

- [ ] **Step 5: Commit**

```bash
git add crates/scenario/tests/session_flow.rs crates/scenario/src/lib.rs
git commit -m "test(scenario): cover flow testing helpers end to end"
```

---

## Self-Review

### Spec coverage

- Session action ergonomics: covered in Task 1
- Screen-level expectations: covered in Task 2
- Screen stabilization helper: covered in Task 2
- Reusable session profiles: covered in Task 3
- Project git setup actions: covered in Task 4
- Autotune-style example/migration target: covered in Task 5
- Optional flow helper (`step()`): intentionally deferred, matching the approved spec’s recommended scope

### Placeholder scan

- No `TODO`, `TBD`, or “implement later” placeholders remain in the tasks
- Each task includes concrete files, commands, and code snippets

### Type consistency

- `SessionConfig` is consistently named across tasks
- Session helper names are consistent: `send_keys`, `press`, `enter`, `ctrl_c`, `visible_text`, `expect_screen`, `expect_screen_not`, `wait_for_screen_stable`
- Error variant name is consistent: `ProjectSetup`

