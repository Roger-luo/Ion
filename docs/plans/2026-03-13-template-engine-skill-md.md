# Template Engine for SKILL.md Generation — Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the hand-maintained `SKILL.md` with a minijinja template (`templates/ion-cli.md.j2`) whose JSON examples are generated programmatically by running `ion --json` commands against fixture environments. A test ensures the rendered template matches the checked-in `SKILL.md`, so it can never drift from the binary's actual output.

**Architecture:** Add `minijinja` as a dev-dependency. A Rust integration test sets up tempdir fixtures, runs real `ion --json` commands, captures their JSON output, renders the template with those values, and asserts the result matches the committed `SKILL.md`. An ignored test generates `SKILL.md` when formats change (`cargo test regenerate_skill_md -- --ignored`). The template uses `{% raw %}` blocks for static examples (network-dependent or machine-specific commands) and `{{ var }}` for programmatically-captured examples.

**Tech Stack:** minijinja (template engine), serde_json (JSON pretty-printing), tempfile (fixture setup), existing `CARGO_BIN_EXE_ion` test pattern.

---

## File Structure

| Action | Path | Responsibility |
|--------|------|----------------|
| Create | `templates/ion-cli.md.j2` | Jinja2 template for the ion-cli SKILL.md |
| Modify | `SKILL.md` | Now generated from template |
| Modify | `Cargo.toml` | Add `minijinja` as dev-dependency |
| Create | `tests/skill_md_generation.rs` | Integration test: renders template, asserts matches SKILL.md |
| Modify | `src/commands/remove.rs:70` | Guard `println!("Removing skill...")` with `if !json` — it currently leaks into JSON stdout |

---

## Chunk 1: Fix remove.rs JSON Purity and Add minijinja

### Task 1: Fix remove.rs stdout pollution in --json mode

**Files:**
- Modify: `src/commands/remove.rs:70`

The `println!("Removing skill {}...", ...)` at line 70 fires unconditionally, including in `--json` mode. This mixes human text with JSON on stdout, breaking any JSON parser consuming the output.

- [ ] **Step 1: Write a failing test that demonstrates the bug**

Add to `tests/json_integration.rs`:

```rust
#[test]
fn json_remove_yes_returns_pure_json() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("Ion.toml"),
        "[skills]\ntest-skill = \"owner/repo\"\n",
    ).unwrap();
    std::fs::write(dir.path().join("Ion.lock"), "").unwrap();
    std::fs::create_dir_all(dir.path().join(".agents/skills/test-skill")).unwrap();

    let output = ion()
        .args(["--json", "remove", "test-skill", "--yes"])
        .current_dir(dir.path())
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value = serde_json::from_str(&stdout)
        .expect("stdout should be valid JSON with no extra text");
    assert_eq!(parsed["success"], true);
    assert!(parsed["data"]["removed"].is_array());
}
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `cargo test json_remove_yes_returns_pure_json`
Expected: FAIL — stdout contains `"Removing skill 'test-skill'...\n{...}"` which is not valid JSON.

- [ ] **Step 3: Fix the bug**

In `src/commands/remove.rs`, wrap the println at line 70 in a `!json` guard:

Change:
```rust
        println!("Removing skill {}...", p.bold(&format!("'{skill_name}'")));
```

To:
```rust
        if !json {
            println!("Removing skill {}...", p.bold(&format!("'{skill_name}'")));
        }
```

This requires adding `json: bool` visibility into the loop. The function signature already has `json`, so this is accessible.

- [ ] **Step 4: Run the test to verify it passes**

Run: `cargo test json_remove_yes_returns_pure_json`
Expected: PASS

- [ ] **Step 5: Run all tests**

Run: `cargo test`
Expected: All pass

- [ ] **Step 6: Commit**

```bash
git add src/commands/remove.rs tests/json_integration.rs
git commit -m "fix: guard println in remove command for --json mode"
```

### Task 2: Add minijinja dev-dependency

**Files:**
- Modify: `Cargo.toml`

- [ ] **Step 1: Add minijinja to dev-dependencies**

In `Cargo.toml`, add under `[dev-dependencies]`:

```toml
[dev-dependencies]
minijinja = "2"
```

Note: `tempfile` and `serde_json` are already in `[dependencies]` so they're available for tests too.

- [ ] **Step 2: Verify it compiles**

Run: `cargo check --tests`
Expected: compiles without errors

- [ ] **Step 3: Commit**

```bash
git add Cargo.toml Cargo.lock
git commit -m "build: add minijinja as dev-dependency for template-based SKILL.md generation"
```

---

## Chunk 2: Create the Template

### Task 3: Create the Jinja2 template

**Files:**
- Create: `templates/ion-cli.md.j2`

The template contains the full SKILL.md content. Design decisions:

1. **Programmatic variables** (`{{ var }}`) — for commands testable in a local tempdir without network
2. **Static `{% raw %}` blocks** — for commands that need network (search, add, update, self check/update) or produce machine-specific output (self info)
3. Each testable command gets its own named variable (e.g., `example_init_no_targets`)

Variable inventory:
- `{{ example_init_no_targets }}` — `ion --json project init` (exit 2)
- `{{ example_init_with_targets }}` — `ion --json project init --target claude --target cursor`
- `{{ example_remove_confirm }}` — `ion --json remove test-skill` (exit 2)
- `{{ example_remove_yes }}` — `ion --json remove test-skill --yes`
- `{{ example_skill_list }}` — `ion --json skill list` (empty project)
- `{{ example_validate }}` — `ion --json skill validate path/`
- `{{ example_config_list }}` — `ion --json config list --project`
- `{{ example_config_get }}` — `ion --json config get targets.claude --project`
- `{{ example_config_set }}` — `ion --json config set targets.claude .claude/commands --project`
- `{{ example_gc_dry_run }}` — `ion --json cache gc --dry-run`

- [ ] **Step 1: Create templates/ directory and the template file**

Create `templates/ion-cli.md.j2` with the content below. Key things to note:
- The YAML frontmatter is literal (no variables)
- `{% raw %}...{% endraw %}` wraps all static example blocks
- `{{ variable }}` is used only for locally-testable commands
- The self info, search, add, update, skill info sections use static examples inside `{% raw %}`

```
---
name: ion-cli
description: "Operate the Ion skill manager from CLI using --json flag for structured, non-interactive control of skill installation, search, and project management."
compatibility: "claude, cursor, windsurf"
---

# Ion CLI for Agents

Operate the Ion skill manager programmatically using the `--json` flag.

## JSON Interface

All commands support `ion --json <command>`. The `--json` flag:

- Outputs structured JSON to stdout
- Disables all interactive prompts (TUI, confirmations)
- Uses a two-stage pattern: commands that need decisions return options (exit 2), you re-run with explicit flags

### Response Envelope

Every response is one of three shapes:

{% raw %}
**Success** (exit 0) — operation completed:
```json
{"success": true, "data": { ... }}
```

**Action required** (exit 2) — you must re-run with explicit flags:
```json
{"success": false, "action_required": "<type>", "data": { ... }}
```

**Error** (exit 1) — operation failed:
```json
{"success": false, "error": "message"}
```
{% endraw %}

## Commands with Examples

Each example below shows the exact command and its JSON output, so you can learn the input-output format.

### Initialize a project

Without `--target`, ion discovers available targets and asks you to choose:

```bash
$ ion --json project init
```
```json
{{ example_init_no_targets }}
```

Re-run with explicit targets:

```bash
$ ion --json project init --target claude --target cursor
```
```json
{{ example_init_with_targets }}
```

### Search for skills

{% raw %}
```bash
$ ion --json search "code review"
```
```json
{
  "success": true,
  "data": [
    {
      "name": "code-review",
      "description": "Automated code review skill",
      "source": "obra/skills/code-review",
      "registry": "github",
      "stars": 42
    },
    {
      "name": "pr-reviewer",
      "description": "Pull request review assistant",
      "source": "acme/pr-reviewer",
      "registry": "skills.sh",
      "stars": 18
    }
  ]
}
```
{% endraw %}

Use the `source` field from results to install a skill.

### Add a single skill

{% raw %}
```bash
$ ion --json add obra/skills/code-review
```
```json
{
  "success": true,
  "data": {
    "name": "code-review",
    "installed_to": ".agents/skills/code-review/",
    "targets": ["claude", "cursor"]
  }
}
```

If the skill has validation warnings, you get exit 2 instead:

```bash
$ ion --json add acme/experimental-skill
```
```json
{
  "success": false,
  "action_required": "validation_warnings",
  "data": {
    "skill": "experimental-skill",
    "warnings": [
      {"severity": "warning", "checker": "security", "message": "Skill requests shell access"}
    ]
  }
}
```

Re-run with `--allow-warnings` to accept:

```bash
$ ion --json add acme/experimental-skill --allow-warnings
```
```json
{
  "success": true,
  "data": {
    "name": "experimental-skill",
    "installed_to": ".agents/skills/experimental-skill/",
    "targets": ["claude"]
  }
}
```

### Add from a skill collection

When a repo contains multiple skills, ion lists them for you to choose:

```bash
$ ion --json add obra/skills
```
```json
{
  "success": false,
  "action_required": "skill_selection",
  "data": {
    "skills": [
      {"name": "code-review", "status": "clean"},
      {"name": "test-driven-dev", "status": "clean"},
      {"name": "experimental", "status": "warnings", "warning_count": 2}
    ]
  }
}
```

Pick specific skills:

```bash
$ ion --json add obra/skills --skills code-review,test-driven-dev
```
```json
{
  "success": true,
  "data": {
    "name": "code-review",
    "installed_to": ".agents/skills/code-review/",
    "targets": ["claude"]
  }
}
```

### Install all from Ion.toml

```bash
$ ion --json add
```
```json
{
  "success": true,
  "data": {
    "installed": ["code-review", "test-driven-dev"],
    "skipped": ["pinned-skill"]
  }
}
```
{% endraw %}

### Remove a skill

First call returns a confirmation prompt:

```bash
$ ion --json remove test-skill
```
```json
{{ example_remove_confirm }}
```

Confirm with `--yes`:

```bash
$ ion --json remove test-skill --yes
```
```json
{{ example_remove_yes }}
```

### List installed skills

```bash
$ ion --json skill list
```
```json
{{ example_skill_list }}
```

### Show skill info

{% raw %}
```bash
$ ion --json skill info code-review
```
```json
{
  "success": true,
  "data": {
    "name": "code-review",
    "description": "Automated code review skill",
    "source_type": "Github",
    "source": "obra/skills",
    "path": "code-review",
    "git_url": "https://github.com/obra/skills.git"
  }
}
```

### Update skills

```bash
$ ion --json update
```
```json
{
  "success": true,
  "data": {
    "updated": [
      {"name": "code-review", "old_version": "v1.1.0", "new_version": "v1.2.0", "binary": false}
    ],
    "skipped": [
      {"name": "pinned-skill", "reason": "pinned to refs/tags/v1.0"}
    ],
    "failed": [],
    "up_to_date": [
      {"name": "test-driven-dev"}
    ]
  }
}
```

Update a single skill:

```bash
$ ion --json update code-review
```
```json
{
  "success": true,
  "data": {
    "updated": [
      {"name": "code-review", "old_version": "v1.1.0", "new_version": "v1.2.0", "binary": false}
    ],
    "skipped": [],
    "failed": [],
    "up_to_date": []
  }
}
```
{% endraw %}

### Validate skills

```bash
$ ion --json skill validate
```
```json
{{ example_validate }}
```

### Configuration

```bash
$ ion --json config list
```
```json
{{ example_config_list }}
```

```bash
$ ion --json config get targets.claude
```
```json
{{ example_config_get }}
```

```bash
$ ion --json config set targets.claude .claude/commands
```
```json
{{ example_config_set }}
```

### Cache management

```bash
$ ion --json cache gc --dry-run
```
```json
{{ example_gc_dry_run }}
```

### Self management

{% raw %}
```bash
$ ion --json self info
```
```json
{
  "success": true,
  "data": {
    "version": "0.2.1",
    "target": "aarch64-apple-darwin",
    "exe": "/usr/local/bin/ion"
  }
}
```

```bash
$ ion --json self check
```
```json
{
  "success": true,
  "data": {
    "installed": "0.2.0",
    "latest": "0.2.1",
    "update_available": true
  }
}
```

```bash
$ ion --json self update
```
```json
{
  "success": true,
  "data": {
    "updated": true,
    "old_version": "0.2.0",
    "new_version": "0.2.1",
    "exe": "/usr/local/bin/ion"
  }
}
```
{% endraw %}

## Typical Agent Workflow

Here is a complete example showing how to search for and install a skill:

{% raw %}
```bash
# 1. Initialize project (if no Ion.toml exists)
$ ion --json project init --target claude
# → {"success": true, "data": {"targets": {"claude": ".claude/skills"}, "manifest": "Ion.toml"}}

# 2. Search for a skill
$ ion --json search "testing"
# → {"success": true, "data": [{"name": "test-driven-development", "source": "obra/skills/test-driven-development", ...}]}

# 3. Install it (use the "source" field from search results)
$ ion --json add obra/skills/test-driven-development
# → {"success": true, "data": {"name": "test-driven-development", "installed_to": ".agents/skills/test-driven-development/", "targets": ["claude"]}}

# 4. If exit code 2 with warnings, re-run with --allow-warnings
$ ion --json add some/skill --allow-warnings
# → {"success": true, "data": {"name": "some-skill", ...}}

# 5. Verify what's installed
$ ion --json skill list
# → {"success": true, "data": [{"name": "test-driven-development", "source": "obra/skills", ...}]}

# 6. Remove a skill when no longer needed
$ ion --json remove old-skill --yes
# → {"success": true, "data": {"removed": ["old-skill"]}}
```
{% endraw %}

## Key Flags

| Flag | Scope | Purpose |
|------|-------|---------|
| `--json` | Global | Structured JSON output, no prompts |
| `--allow-warnings` | `add` | Proceed despite validation warnings |
| `--skills a,b,c` | `add` | Select specific skills from a collection |
| `--yes` / `-y` | `remove` | Skip removal confirmation |
| `--target name` | `project init` | Specify targets non-interactively |
| `--force` | `project init`, `skill new` | Overwrite existing files |
```

- [ ] **Step 2: Verify template file was created**

Run: `ls templates/ion-cli.md.j2`
Expected: file exists

- [ ] **Step 3: Commit**

```bash
git add templates/ion-cli.md.j2
git commit -m "feat: add minijinja template for SKILL.md generation"
```

---

## Chunk 3: The Generation Test

### Task 4: Create the integration test

**Files:**
- Create: `tests/skill_md_generation.rs`

This test file has two tests:
1. `skill_md_matches_template` — asserts the committed `SKILL.md` matches what the template produces. Runs in CI.
2. `regenerate_skill_md` (ignored) — writes `SKILL.md` from the template. Run manually with `cargo test regenerate_skill_md -- --ignored`.

Both share a `render_skill_md()` function that sets up fixtures, runs commands, captures JSON, and renders the template.

- [ ] **Step 1: Write the full test file**

Create `tests/skill_md_generation.rs`:

```rust
use std::path::Path;
use std::process::Command;

fn ion() -> Command {
    Command::new(env!("CARGO_BIN_EXE_ion"))
}

/// Run an ion command and return stdout. Accepts exit 0 and 2 (action_required).
fn capture_json(args: &[&str], dir: &Path) -> String {
    let output = ion()
        .args(args)
        .current_dir(dir)
        .output()
        .expect("failed to execute ion");

    let code = output.status.code().unwrap_or(-1);
    assert!(
        code == 0 || code == 2,
        "ion {:?} failed with exit {code}\nstderr: {}",
        args,
        String::from_utf8_lossy(&output.stderr)
    );

    String::from_utf8(output.stdout)
        .expect("non-utf8 stdout")
        .trim()
        .to_string()
}

/// Replace dynamic substrings in JSON output for determinism.
fn stabilize(json: &str, replacements: &[(&str, &str)]) -> String {
    let mut result = json.to_string();
    for (from, to) in replacements {
        result = result.replace(from, to);
    }
    result
}

fn render_skill_md() -> String {
    let template_src = std::fs::read_to_string(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("templates/ion-cli.md.j2"),
    )
    .expect("failed to read template");

    // -- project init (no targets) --
    let init_dir = tempfile::tempdir().unwrap();
    // Create .claude/ so "detected: true" appears for claude (realistic example)
    std::fs::create_dir(init_dir.path().join(".claude")).unwrap();
    let example_init_no_targets = capture_json(
        &["--json", "project", "init"],
        init_dir.path(),
    );

    // -- project init (with targets) --
    let init_dir2 = tempfile::tempdir().unwrap();
    let example_init_with_targets = capture_json(
        &["--json", "project", "init", "--target", "claude", "--target", "cursor"],
        init_dir2.path(),
    );

    // -- remove (confirm, exit 2) --
    let remove_dir = tempfile::tempdir().unwrap();
    std::fs::write(
        remove_dir.path().join("Ion.toml"),
        "[skills]\ntest-skill = \"owner/repo\"\n",
    ).unwrap();
    std::fs::write(remove_dir.path().join("Ion.lock"), "").unwrap();
    let example_remove_confirm = capture_json(
        &["--json", "remove", "test-skill"],
        remove_dir.path(),
    );

    // -- remove (with --yes) --
    let remove_dir2 = tempfile::tempdir().unwrap();
    std::fs::write(
        remove_dir2.path().join("Ion.toml"),
        "[skills]\ntest-skill = \"owner/repo\"\n",
    ).unwrap();
    std::fs::write(remove_dir2.path().join("Ion.lock"), "").unwrap();
    std::fs::create_dir_all(remove_dir2.path().join(".agents/skills/test-skill")).unwrap();
    let example_remove_yes = capture_json(
        &["--json", "remove", "test-skill", "--yes"],
        remove_dir2.path(),
    );

    // -- skill list (empty project) --
    let list_dir = tempfile::tempdir().unwrap();
    std::fs::write(list_dir.path().join("Ion.toml"), "[skills]\n").unwrap();
    let example_skill_list = capture_json(
        &["--json", "skill", "list"],
        list_dir.path(),
    );

    // -- validate --
    let validate_dir = tempfile::tempdir().unwrap();
    let skill_dir = validate_dir.path().join("test-skill");
    std::fs::create_dir_all(&skill_dir).unwrap();
    std::fs::write(
        skill_dir.join("SKILL.md"),
        "---\nname: test-skill\ndescription: A test skill\n---\n\n# Test Skill\n",
    ).unwrap();
    let skill_dir_str = skill_dir.display().to_string();
    let raw_validate = capture_json(
        &["--json", "skill", "validate", &skill_dir_str],
        validate_dir.path(),
    );
    // Replace absolute path with relative for determinism
    let example_validate = stabilize(
        &raw_validate,
        &[(&format!("{}/SKILL.md", skill_dir_str), "test-skill/SKILL.md")],
    );

    // -- config list (project-scoped to avoid depending on user's global config) --
    let config_dir = tempfile::tempdir().unwrap();
    std::fs::write(
        config_dir.path().join("Ion.toml"),
        "[options.targets]\nclaude = \".claude/skills\"\ncursor = \".cursor/skills\"\n",
    ).unwrap();
    let example_config_list = capture_json(
        &["--json", "config", "list", "--project"],
        config_dir.path(),
    );

    // -- config get --
    let example_config_get = capture_json(
        &["--json", "config", "get", "targets.claude", "--project"],
        config_dir.path(),
    );

    // -- config set --
    let config_set_dir = tempfile::tempdir().unwrap();
    std::fs::write(
        config_set_dir.path().join("Ion.toml"),
        "[options.targets]\nclaude = \".claude/skills\"\n",
    ).unwrap();
    let example_config_set = capture_json(
        &["--json", "config", "set", "targets.claude", ".claude/commands", "--project"],
        config_set_dir.path(),
    );

    // -- cache gc --
    let example_gc_dry_run = capture_json(
        &["--json", "cache", "gc", "--dry-run"],
        init_dir.path(), // any dir works
    );

    // -- self info: verify structure only (values are machine-specific, template uses static example) --
    let self_info_output = capture_json(&["--json", "self", "info"], init_dir.path());
    let self_info: serde_json::Value = serde_json::from_str(&self_info_output).unwrap();
    assert_eq!(self_info["success"], true);
    assert!(self_info["data"]["version"].is_string());
    assert!(self_info["data"]["target"].is_string());
    assert!(self_info["data"]["exe"].is_string());

    // Render template
    let mut env = minijinja::Environment::new();
    env.add_template("skill.md", &template_src).unwrap();
    let tmpl = env.get_template("skill.md").unwrap();
    tmpl.render(minijinja::context! {
        example_init_no_targets,
        example_init_with_targets,
        example_remove_confirm,
        example_remove_yes,
        example_skill_list,
        example_validate,
        example_config_list,
        example_config_get,
        example_config_set,
        example_gc_dry_run,
    }).unwrap()
}

#[test]
fn skill_md_matches_template() {
    let rendered = render_skill_md();
    let committed = std::fs::read_to_string(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("SKILL.md"),
    )
    .expect("failed to read SKILL.md");

    if rendered.trim() != committed.trim() {
        let rendered_lines: Vec<&str> = rendered.trim().lines().collect();
        let committed_lines: Vec<&str> = committed.trim().lines().collect();

        for (i, (r, c)) in rendered_lines.iter().zip(committed_lines.iter()).enumerate() {
            if r != c {
                eprintln!("First difference at line {}:", i + 1);
                eprintln!("  rendered:  {:?}", r);
                eprintln!("  committed: {:?}", c);
                break;
            }
        }

        if rendered_lines.len() != committed_lines.len() {
            eprintln!(
                "Line count differs: rendered={}, committed={}",
                rendered_lines.len(),
                committed_lines.len()
            );
        }

        panic!(
            "SKILL.md is out of date with template. Run:\n  \
             cargo test regenerate_skill_md -- --ignored\n\
             to regenerate it."
        );
    }
}

#[test]
#[ignore]
fn regenerate_skill_md() {
    let rendered = render_skill_md();
    let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("SKILL.md");
    std::fs::write(&path, rendered).expect("failed to write SKILL.md");
    println!("Regenerated SKILL.md");
}
```

- [ ] **Step 2: Generate the initial SKILL.md from the template**

Run: `cargo test regenerate_skill_md -- --ignored`
Expected: PASS, writes a new `SKILL.md`

- [ ] **Step 3: Run the comparison test**

Run: `cargo test skill_md_matches_template`
Expected: PASS

- [ ] **Step 4: Validate the generated SKILL.md**

Run: `cargo run -- skill validate SKILL.md`
Expected: No schema errors (suspicious-file errors from worktree build artifacts are unrelated to our change)

- [ ] **Step 5: Run all tests**

Run: `cargo test`
Expected: All pass

- [ ] **Step 6: Commit**

```bash
git add tests/skill_md_generation.rs SKILL.md
git commit -m "feat: generate SKILL.md from minijinja template with real JSON examples

Adds an integration test that renders templates/ion-cli.md.j2 using real
ion --json command output captured from fixture environments. The test
asserts the committed SKILL.md matches the rendered template, preventing
drift between documented and actual JSON formats.

Run 'cargo test regenerate_skill_md -- --ignored' to regenerate SKILL.md
when JSON formats change."
```

---

## Implementation Notes

### What's testable vs static

| Command | Testable locally? | Template approach |
|---------|------------------|-------------------|
| `project init` | Yes | `{{ variable }}` — tempdir fixture |
| `search` | No (network) | `{% raw %}` static |
| `add` | No (network) | `{% raw %}` static |
| `remove` | Yes | `{{ variable }}` — tempdir with Ion.toml |
| `skill list` | Yes | `{{ variable }}` — tempdir with Ion.toml |
| `skill info` | No (needs installed skill from remote) | `{% raw %}` static |
| `update` | No (network) | `{% raw %}` static |
| `validate` | Yes | `{{ variable }}` — tempdir with SKILL.md |
| `config` | Yes | `{{ variable }}` — tempdir with `--project` |
| `cache gc` | Yes | `{{ variable }}` — empty registry |
| `self info` | Machine-dependent output | `{% raw %}` static |
| `self check` | No (network) | `{% raw %}` static |
| `self update` | No (network) | `{% raw %}` static |

### Template variable naming convention

All variables: `example_<command>_<variant>`. See Task 3 Step 1 for the full list.

### Future template usage

The `templates/` directory is intended to house other templates beyond `ion-cli.md.j2`. Potential future uses:
- `skill-new.md.j2` — replacing the hardcoded `DEFAULT_TEMPLATE` and `BIN_SKILL_TEMPLATE` in `src/commands/new.rs`
- `collection-readme.md.j2` — for collection README generation
- Custom user-defined templates for skill creation
