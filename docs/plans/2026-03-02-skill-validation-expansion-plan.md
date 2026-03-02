# Skill Validation Expansion Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Expand skill validation beyond schema checks, add tree-sitter-backed code-block validation, and introduce `ion validate` for recursive local skill validation.

**Architecture:** Keep validation in `ion-skill` behind the existing `validate` module and `SkillChecker` pattern. Add domain checkers (security, structure, code blocks), then consume the same pipeline from installer flows and a new CLI `validate` command. Installer blocks on errors and prompts on warnings; `ion validate` is non-interactive and returns exit code by aggregate findings.

**Tech Stack:** Rust, existing `ion`/`ion-skill` crates, `tree-sitter` (+ language grammars), `pulldown-cmark`, `walkdir`, `regex`, `serde_*` parsers.

---

### Task 1: Add dependencies for recursive scanning and code-block parsing

**Files:**
- Modify: `crates/ion-skill/Cargo.toml`

**Step 1: Write a failing compile test by adding imports in a scratch unit test**

Add to `crates/ion-skill/src/validate/mod.rs` test module (temporary):

```rust
#[test]
fn dependency_smoke() {
    let _ = walkdir::WalkDir::new(".");
    let _ = pulldown_cmark::Parser::new("text");
    let _ = tree_sitter::Parser::new();
}
```

**Step 2: Run test to verify it fails (missing crates)**

Run: `cargo test -p ion-skill validate::tests::dependency_smoke -q`
Expected: FAIL with unresolved imports.

**Step 3: Add minimal dependencies**

In `crates/ion-skill/Cargo.toml` add:

```toml
walkdir = "2"
pulldown-cmark = "0.12"
tree-sitter = "0.24"
tree-sitter-bash = "0.24"
tree-sitter-python = "0.24"
tree-sitter-rust = "0.24"
```

**Step 4: Re-run test and remove temporary smoke test**

Run: `cargo test -p ion-skill validate::tests::dependency_smoke -q`
Expected: PASS, then delete the temporary test.

**Step 5: Commit**

```bash
git add crates/ion-skill/Cargo.toml crates/ion-skill/src/validate/mod.rs
git commit -m "chore: add validation dependencies for scanning and tree-sitter"
```

---

### Task 2: Add skill discovery API for recursive `SKILL.md` lookup

**Files:**
- Create: `crates/ion-skill/src/validate/discovery.rs`
- Modify: `crates/ion-skill/src/validate/mod.rs`
- Test: `crates/ion-skill/src/validate/discovery.rs`

**Step 1: Write failing tests for discovery behavior**

In `discovery.rs` tests, add cases for:

```rust
#[test]
fn discovers_skill_md_recursively() { /* workspace/a/SKILL.md + workspace/b/nested/SKILL.md */ }

#[test]
fn ignores_common_heavy_dirs() { /* .git/skills/SKILL.md should be ignored */ }

#[test]
fn returns_sorted_results() { /* assert lexical path ordering */ }
```

**Step 2: Run test to verify failure**

Run: `cargo test -p ion-skill validate::discovery::tests -q`
Expected: FAIL because module/functions are not implemented.

**Step 3: Implement minimal discovery function**

Implement:

```rust
pub fn discover_skill_files(root: &Path) -> Result<Vec<PathBuf>> {
    // walk recursively with walkdir
    // skip .git, node_modules, target, .cache
    // collect files named SKILL.md
    // sort + dedup
}
```

Export from `validate/mod.rs`:

```rust
pub mod discovery;
```

**Step 4: Re-run tests**

Run: `cargo test -p ion-skill validate::discovery::tests -q`
Expected: PASS.

**Step 5: Commit**

```bash
git add crates/ion-skill/src/validate/discovery.rs crates/ion-skill/src/validate/mod.rs
git commit -m "feat: add recursive SKILL.md discovery utilities"
```

---

### Task 3: Add markdown analysis helpers (fenced blocks, links, tool mentions)

**Files:**
- Create: `crates/ion-skill/src/validate/markdown.rs`
- Modify: `crates/ion-skill/src/validate/mod.rs`
- Test: `crates/ion-skill/src/validate/markdown.rs`

**Step 1: Write failing parser tests**

Add tests for:

```rust
#[test]
fn extracts_fenced_blocks_with_language_and_line() { /* bash/python/rust fences */ }

#[test]
fn extracts_markdown_links_and_filters_local_targets() { /* ./refs.md and #anchor */ }

#[test]
fn detects_tool_mentions_in_body_text() { /* Bash, Read, Write */ }
```

**Step 2: Run tests and confirm failure**

Run: `cargo test -p ion-skill validate::markdown::tests -q`
Expected: FAIL.

**Step 3: Implement helpers**

Implement a small API:

```rust
pub struct CodeBlock { pub lang: String, pub code: String, pub start_line: usize }
pub fn extract_code_blocks(body: &str) -> Vec<CodeBlock>;
pub fn extract_local_links(body: &str) -> Vec<String>;
pub fn extract_tool_mentions(body: &str) -> std::collections::BTreeSet<String>;
```

Use `pulldown-cmark` for fenced code extraction and links.

**Step 4: Re-run tests**

Run: `cargo test -p ion-skill validate::markdown::tests -q`
Expected: PASS.

**Step 5: Commit**

```bash
git add crates/ion-skill/src/validate/markdown.rs crates/ion-skill/src/validate/mod.rs
git commit -m "feat: add markdown extraction helpers for validation"
```

---

### Task 4: Implement security checkers

**Files:**
- Create: `crates/ion-skill/src/validate/security.rs`
- Modify: `crates/ion-skill/src/validate/mod.rs`
- Test: `crates/ion-skill/src/validate/security.rs`

**Step 1: Write failing tests for each rule family**

Add tests for:

```rust
#[test]
fn flags_invisible_unicode_as_error() {}
#[test]
fn flags_curl_pipe_sh_as_warning() {}
#[test]
fn flags_sensitive_paths_as_warning() {}
#[test]
fn flags_suspicious_files_in_skill_dir() {}
```

**Step 2: Run tests and confirm failure**

Run: `cargo test -p ion-skill validate::security::tests -q`
Expected: FAIL.

**Step 3: Implement checkers with `SkillChecker` trait**

Implement structs:

```rust
pub struct PromptInjectionChecker;
pub struct DangerousCommandChecker;
pub struct SensitivePathChecker;
pub struct SuspiciousFileChecker;
```

Each returns `Vec<Finding>` with `checker`, `severity`, `message`, `detail` and optional location data (line/file in detail).

**Step 4: Register in `run_all_checkers()`**

In `validate/mod.rs` register these checkers in the checker vector.

**Step 5: Re-run tests**

Run: `cargo test -p ion-skill validate::security::tests -q`
Expected: PASS.

**Step 6: Commit**

```bash
git add crates/ion-skill/src/validate/security.rs crates/ion-skill/src/validate/mod.rs
git commit -m "feat: add security validation checkers"
```

---

### Task 5: Implement structural integrity checkers

**Files:**
- Create: `crates/ion-skill/src/validate/structure.rs`
- Modify: `crates/ion-skill/src/validate/mod.rs`
- Test: `crates/ion-skill/src/validate/structure.rs`

**Step 1: Write failing structure tests**

Add tests for:

```rust
#[test]
fn reports_missing_referenced_local_files() {}
#[test]
fn reports_path_traversal_references() {}
#[test]
fn reports_missing_allowed_tools_when_tools_are_mentioned() {}
#[test]
fn accepts_existing_relative_references() {}
```

**Step 2: Run tests and confirm failure**

Run: `cargo test -p ion-skill validate::structure::tests -q`
Expected: FAIL.

**Step 3: Implement structure checker(s)**

Implement:

```rust
pub struct ReferenceIntegrityChecker;
pub struct ToolDeclarationConsistencyChecker;
```

Rules:
- Validate local link/file targets exist under skill root.
- Flag `../` path traversal escapes.
- Compare body tool mentions vs `allowed-tools` declaration.

**Step 4: Register checkers and run tests**

Run: `cargo test -p ion-skill validate::structure::tests -q`
Expected: PASS.

**Step 5: Commit**

```bash
git add crates/ion-skill/src/validate/structure.rs crates/ion-skill/src/validate/mod.rs
git commit -m "feat: add structural validation checkers"
```

---

### Task 6: Implement tree-sitter code-block checker

**Files:**
- Create: `crates/ion-skill/src/validate/codeblock.rs`
- Modify: `crates/ion-skill/src/validate/mod.rs`
- Test: `crates/ion-skill/src/validate/codeblock.rs`

**Step 1: Write failing parse tests**

Add tests for:

```rust
#[test]
fn invalid_python_block_is_error() {}
#[test]
fn invalid_bash_block_is_error() {}
#[test]
fn invalid_rust_block_is_error() {}
#[test]
fn unknown_language_block_is_ignored_or_info() {}
```

**Step 2: Run tests and confirm failure**

Run: `cargo test -p ion-skill validate::codeblock::tests -q`
Expected: FAIL.

**Step 3: Implement `TreeSitterCodeBlockChecker`**

Implement parser dispatch by language tag:

```rust
fn parser_for_lang(lang: &str) -> Option<tree_sitter::Language>;
```

Supported tags initially:
- `bash`, `sh`
- `python`
- `rust`

Use tree-sitter parse result/root node error flags for high-confidence syntax failures.

**Step 4: Register checker and re-run tests**

Run: `cargo test -p ion-skill validate::codeblock::tests -q`
Expected: PASS.

**Step 5: Commit**

```bash
git add crates/ion-skill/src/validate/codeblock.rs crates/ion-skill/src/validate/mod.rs
git commit -m "feat: add tree-sitter code block validation checker"
```

---

### Task 7: Add shared validation report API and installer gating

**Files:**
- Modify: `crates/ion-skill/src/validate/mod.rs`
- Modify: `crates/ion-skill/src/error.rs`
- Modify: `crates/ion-skill/src/installer.rs`
- Test: `crates/ion-skill/src/installer.rs`

**Step 1: Write failing installer tests for warning/error gating**

Add tests for:

```rust
#[test]
fn install_blocks_on_validation_errors() {}
#[test]
fn install_returns_warning_error_when_warnings_not_allowed() {}
#[test]
fn install_proceeds_when_warnings_allowed() {}
```

**Step 2: Run tests and confirm failure**

Run: `cargo test -p ion-skill installer::tests -q`
Expected: FAIL.

**Step 3: Implement report + gating APIs**

Add in `validate/mod.rs`:

```rust
pub struct ValidationReport {
    pub findings: Vec<Finding>,
    pub error_count: usize,
    pub warning_count: usize,
    pub info_count: usize,
}

pub fn validate_skill_dir(skill_dir: &Path, meta: &SkillMetadata, body: &str) -> ValidationReport;
```

Add in `error.rs` new variants for validation failures/warnings.

Update installer:

```rust
pub struct InstallValidationOptions {
    pub skip_validation: bool,
    pub allow_warnings: bool,
}

pub fn install_with_options(&self, name: &str, source: &SkillSource, opts: InstallValidationOptions) -> Result<LockedSkill>;
```

Keep `install()` as wrapper using strict default.

**Step 4: Re-run tests**

Run: `cargo test -p ion-skill installer::tests -q`
Expected: PASS.

**Step 5: Commit**

```bash
git add crates/ion-skill/src/validate/mod.rs crates/ion-skill/src/error.rs crates/ion-skill/src/installer.rs
git commit -m "feat: gate installs with validation reports and warning policy"
```

---

### Task 8: Add warning prompt flow to `add` and `install` commands

**Files:**
- Modify: `src/commands/add.rs`
- Modify: `src/commands/install.rs`
- Modify: `src/commands/migrate.rs`
- Test: `tests/integration.rs`

**Step 1: Write failing integration tests for non-error warning flow**

Add integration tests:

```rust
#[test]
fn add_prompts_on_warnings_and_aborts_by_default() {}

#[test]
fn install_prompts_on_warnings_and_accepts_yes_input() {}
```

Use `Command::spawn()` with piped stdin to send `n\n` / `y\n`.

**Step 2: Run tests and confirm failure**

Run: `cargo test --test integration add_prompts_on_warnings_and_aborts_by_default -q`
Expected: FAIL.

**Step 3: Implement prompt + retry behavior**

In `add.rs`/`install.rs`/`migrate.rs`:
- First call `install_with_options(... allow_warnings = false)`.
- If warning-only validation error is returned, print findings and prompt.
- On `y` rerun with `allow_warnings = true`; otherwise return error.

Add a small shared helper in each file or a tiny local function:

```rust
fn confirm_install_on_warnings() -> anyhow::Result<bool> { /* prompt + read line */ }
```

**Step 4: Re-run focused tests**

Run:
- `cargo test --test integration add_prompts_on_warnings_and_aborts_by_default -q`
- `cargo test --test integration install_prompts_on_warnings_and_accepts_yes_input -q`

Expected: PASS.

**Step 5: Commit**

```bash
git add src/commands/add.rs src/commands/install.rs src/commands/migrate.rs tests/integration.rs
git commit -m "feat: prompt on validation warnings during install flows"
```

---

### Task 9: Implement `ion validate` command with recursive project default

**Files:**
- Create: `src/commands/validate.rs`
- Modify: `src/commands/mod.rs`
- Modify: `src/main.rs`
- Test: `tests/validate_integration.rs`

**Step 1: Write failing CLI integration tests**

Create tests for:

```rust
#[test]
fn validate_help_is_exposed() {}

#[test]
fn validate_default_scans_current_dir_recursively() {}

#[test]
fn validate_single_skill_path() {}

#[test]
fn validate_returns_nonzero_when_any_error_exists() {}
```

**Step 2: Run tests and confirm failure**

Run: `cargo test --test validate_integration -q`
Expected: FAIL (missing command).

**Step 3: Add CLI wiring**

In `src/main.rs` add subcommand:

```rust
Validate {
    path: Option<String>,
}
```

Route to `commands::validate::run(path.as_deref())`.

Update `src/commands/mod.rs`:

```rust
pub mod validate;
```

**Step 4: Implement command behavior**

In `src/commands/validate.rs`:
- Resolve target path (default `.`).
- If file == `SKILL.md`, validate one.
- If directory contains `SKILL.md`, validate one.
- Else recursively discover all `SKILL.md` via `validate::discovery::discover_skill_files`.
- Parse each skill via `SkillMetadata::from_file` + run report.
- Print grouped findings and aggregate summary.
- Return `Err` if any error across skills.

**Step 5: Re-run tests**

Run: `cargo test --test validate_integration -q`
Expected: PASS.

**Step 6: Commit**

```bash
git add src/main.rs src/commands/mod.rs src/commands/validate.rs tests/validate_integration.rs
git commit -m "feat: add ion validate command with recursive skill scanning"
```

---

### Task 10: Stabilize output formatting and run full verification

**Files:**
- Modify: `src/commands/validate.rs`
- Modify: `tests/validate_integration.rs`
- Modify: `tests/integration.rs` (only if flake/order issues)

**Step 1: Add deterministic ordering assertions in tests**

Add assertions for sorted skill output and severity grouping order.

**Step 2: Run full validation-focused suite**

Run:
- `cargo test -p ion-skill validate -q`
- `cargo test --test integration -q`
- `cargo test --test validate_integration -q`

Expected: PASS.

**Step 3: Run full project test suite**

Run: `cargo test`
Expected: PASS.

**Step 4: Commit final polish**

```bash
git add src/commands/validate.rs tests/validate_integration.rs tests/integration.rs
git commit -m "test: stabilize validation output coverage and ordering"
```

---

## Notes for Execution

1. Follow `@test-driven-development` on every task: fail -> minimal fix -> pass.
2. Before claiming completion, run `@verification-before-completion` and capture command output.
3. Keep behavior minimal for v1: static checks only, no external tool execution.
4. If warning-prompt integration tests prove brittle, isolate prompt parsing into pure helper functions and unit-test those directly.
