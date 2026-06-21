# Scenario Crate: Generic CLI Flow Testing

**Date:** 2026-04-13
**Status:** Draft for review

## Problem

`scenario` already covers process execution, PTY sessions, project fixtures, and a few low-level session helpers. That is enough to test interactive CLIs, but real dialog-driven tests still require too much hand-written harness code.

The `autotune2` test suite shows the friction clearly:

- repeated session setup boilerplate (`Scenario::new(...).terminal(...).timeout(...).spawn()`)
- raw escape bytes for common navigation (`b"\x1b[B"`, `b"\r"`, `b"\x03"`)
- brittle assertions against `current_output()` slices when what matters is the *visible screen*
- repeated temporary project bootstrapping plus ad-hoc `git init` setup
- imperative multi-step dialog tests that are verbose and harder to scan than the user flow they represent

The goal is not to build an `autotune`-specific testing DSL. The goal is to let `scenario` define a CLI usage scenario and assert on expected user-visible behavior across a wide range of terminal interfaces.

## Goals

- Keep `scenario` generic and app-agnostic
- Reduce boilerplate for interactive CLI and TUI tests
- Improve reliability by preferring screen-level assertions over raw-output slicing
- Add reusable project/setup helpers for common repository-backed CLI workflows
- Preserve the current low-level APIs so advanced tests can still drop down to raw `Session` control

## Non-Goals

- Do not add application-specific concepts like “wizard”, “questionnaire”, or “approval dialog”
- Do not replace the existing builder/session APIs
- Do not create a large declarative DSL that hides process control completely
- Do not assume every CLI has a stable full-screen TUI; line-oriented REPLs and mixed stdout/TTY apps must still fit

## Design

### 1. Session action ergonomics

Add small, generic action helpers on top of `Session` so tests can express user intent instead of byte sequences.

```rust
impl Session {
    pub fn send_keys<I>(&mut self, keys: I) -> Result<(), Error>
    where
        I: IntoIterator<Item = Key>;

    pub fn press(&mut self, key: Key) -> Result<(), Error>;

    pub fn enter(&mut self) -> Result<(), Error>;
    pub fn ctrl_c(&mut self) -> Result<(), Error>;
}
```

`send_key()` remains the primitive. The new methods are sugar only.

**Why:**

- `autotune2` repeatedly uses raw bytes for arrows, Enter, and Ctrl+C
- tests become shorter and easier to scan
- the API stays generic because it is still just terminal input

### 2. Screen-level expectations

The next gap is not input, but assertions. `visible_screen()` exists, but callers still have to manually fetch the screen and assert over `Vec<String>`.

Add screen-oriented expectation helpers to `Session`:

```rust
impl Session {
    pub fn expect_screen(&self, pattern: &str) -> Result<(), Error>;
    pub fn expect_screen_regex(&self, pattern: &str) -> Result<(), Error>;

    pub fn expect_screen_not(&self, pattern: &str) -> Result<(), Error>;
    pub fn expect_screen_not_regex(&self, pattern: &str) -> Result<(), Error>;

    pub fn visible_text(&self) -> String;
}
```

Semantics:

- `expect_screen*` checks the current visible screen, not the historical raw PTY stream
- negative variants use the same quiet-period idea as `expect_not()`
- `visible_text()` joins the visible screen into a newline-delimited string for simpler regex matching and debugging

**Why:**

- `autotune2` currently slices `current_output()` after `"How should we measure"` to infer that old menu items are gone
- that is really a screen assertion, not a historical output assertion
- screen-based checks are both shorter and less flaky for redraw-heavy interfaces

### 3. Screen stabilization helper

A common source of flaky PTY tests is checking the screen while the app is still redrawing.

Add a generic “screen settled” primitive:

```rust
impl Session {
    pub fn wait_for_screen_stable(&self, quiet_period: Duration) -> Result<(), Error>;
}
```

Semantics:

- sample the visible screen repeatedly
- return once the screen is unchanged for the requested quiet period
- fail on timeout using the session timeout

This is intentionally low-level. Higher-level expectations can call it internally where appropriate, but it is also useful on its own before snapshot-like assertions.

**Why:**

- dialog UIs often repaint multiple times per keypress
- a stable-screen primitive lowers flakiness without hardcoding arbitrary sleeps in tests

### 4. Session builder defaults

Interactive test files repeat the same setup:

```rust
Scenario::new(bin)
    .current_dir(project.path())
    .terminal(Terminal::pty(120, 40))
    .timeout(Duration::from_secs(10))
    .spawn()
```

Add optional reusable execution profiles:

```rust
pub struct SessionConfig {
    pub terminal: Terminal,
    pub timeout: Duration,
}

impl SessionConfig {
    pub fn pty(cols: u16, rows: u16) -> Self;
    pub fn timeout(self, timeout: Duration) -> Self;
}

impl Scenario {
    pub fn session_config(self, config: &SessionConfig) -> Self;
}
```

This is deliberately small. It avoids inventing a separate runner type while still letting callers define a shared test-local profile.

**Why:**

- repeated PTY shape/timeout boilerplate obscures the flow under test
- a reusable config keeps `Scenario` generic and composable

### 5. Project setup actions

`Project` currently creates files and directories, but repo-backed tools still need post-build setup via ad-hoc shell commands.

Add optional setup actions to `ProjectBuilder`:

```rust
impl ProjectBuilder {
    pub fn setup_git(self) -> Self;
    pub fn git_user(self, name: &str, email: &str) -> Self;
    pub fn initial_commit(self, message: &str) -> Self;
}
```

Build semantics:

- file materialization still happens first
- requested setup actions run after files exist
- `setup_git()` initializes a repo
- `git_user()` configures local author info
- `initial_commit()` stages and commits current contents

Error handling:

- add a generic `ProjectSetup { step, source }` error variant for post-build setup failures

**Why:**

- `autotune2` repeatedly shells out to `git init`, `git add`, and `git commit`
- repo initialization is a common CLI-testing need, not an autotune-specific one
- keeping it inside `ProjectBuilder` makes fixtures more declarative

### 6. Optional flow assertion helpers

The highest-value abstraction above raw actions/assertions is a tiny flow helper, not a full DSL.

```rust
impl Session {
    pub fn step(&mut self, visible_prompt: &str) -> Result<FlowStep<'_>, Error>;
}

pub struct FlowStep<'a> {
    session: &'a mut Session,
}

impl FlowStep<'_> {
    pub fn send_line(self, text: &str) -> Result<(), Error>;
    pub fn press(self, key: Key) -> Result<(), Error>;
    pub fn expect_screen_not(self, pattern: &str) -> Result<(), Error>;
}
```

This is optional and intentionally thin. It should not become a custom scripting language. Its job is to group common “wait for prompt, act, assert cleanup” sequences into something easier to read.

If this layer feels too magical during implementation, it should be dropped in favor of the lower-level pieces above. The first five sections are the core design; this sixth section is explicitly optional.

## Example: Current vs Proposed

### Current `autotune2` style

```rust
session.expect("What metric").unwrap();
session.send(b"\x1b[B").unwrap();
session.send(b"\x1b[B").unwrap();
session.send(b"\r").unwrap();

session.expect("How should we measure").unwrap();
let output_so_far = session.current_output();
if let Some(pos) = output_so_far.find("How should we measure") {
    let after_second_q = &output_so_far[pos..];
    assert!(!after_second_q.contains("Runtime performance"));
}
```

### Proposed style

```rust
session.expect("What metric").unwrap();
session.send_keys([Key::Down, Key::Down, Key::Enter]).unwrap();

session.expect_screen("How should we measure").unwrap();
session.expect_screen_not("Runtime performance").unwrap();
```

### Proposed repo fixture setup

```rust
let project = Project::empty()
    .file(".autotune.toml", CONFIG_TOML)
    .file("src/lib.rs", "pub fn hello() -> &'static str { \"hi\" }\n")
    .setup_git()
    .git_user("Test", "test@test.com")
    .initial_commit("initial")
    .build()?;
```

## API and Compatibility Strategy

- All additions are additive
- Existing `Scenario`, `Session`, `Project`, and `Key` APIs remain valid
- Existing low-level methods stay the escape hatch for unsupported cases
- The new helpers should be implemented in terms of the existing primitives wherever possible

This keeps `scenario` useful both as:

- a lightweight process runner for simple CLI tests
- a higher-level flow-testing toolkit for interactive terminal applications

## Testing Strategy

Tests should prove the new APIs help with the exact generic behaviors they claim to support.

### Session action ergonomics

- `send_keys([Down, Enter])` sends the same bytes as repeated `send_key()`
- `enter()` and `ctrl_c()` behave identically to their explicit key equivalents

### Screen expectations

- `expect_screen()` finds text visible after redraws even when historical output contains stale content
- `expect_screen_not()` passes once prior menu content is no longer visible
- regex variants work across joined screen lines

### Screen stabilization

- stable redraw loop eventually settles and passes
- continuously mutating output times out cleanly

### Project setup actions

- `setup_git()` creates a usable repo
- `git_user()` writes local config
- `initial_commit()` creates the expected first commit
- failure cases surface the setup step that failed

### Integration examples

Add or update `crates/scenario/tests/` coverage with small shell-driven PTY cases, plus at least one example that mirrors the autotune-style “prompt -> key navigation -> next prompt -> previous options disappeared” flow.

## Risks and Trade-offs

### Too much abstraction

If the flow helper becomes a DSL, the crate gets harder to learn and more brittle to extend. The design avoids that by making the action/screen helpers the main value, with flow grouping as optional.

### False confidence from screen assertions

Screen assertions are right for user-visible behavior, but they should not replace historical-output assertions entirely. Some tests genuinely care about stream history. That is why `current_output()` and `expect()` stay intact.

### Git setup portability

Project setup actions rely on `git` being available. That is acceptable because the feature is explicitly opt-in and repo-oriented tests already require git. Failures must produce clear errors.

## Recommended Scope

Implement in two phases:

1. Core helpers
   - `send_keys`, `press`, `enter`, `ctrl_c`
   - `visible_text`
   - `expect_screen*`
   - `wait_for_screen_stable`
   - `ProjectBuilder` git setup actions

2. Evaluate whether flow grouping is still needed
   - add `step()` only if the first phase still leaves significant boilerplate in autotune-like tests

This keeps the first implementation grounded and avoids over-design.

