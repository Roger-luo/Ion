# AGENTS.md Management Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add AGENTS.md template sourcing, upstream syncing, and agent-tool symlink management to Ion.

**Architecture:** Three layers — (1) data model changes to Manifest/Lockfile structs + manifest_writer, (2) a shared `agents` module in ion-skill for symlink and template logic, (3) CLI commands (`ion agents init/update/diff`) and integration into existing commands (`ion init`, `ion add`, `ion update`).

**Tech Stack:** Rust, clap (CLI), toml/toml_edit (manifest), serde (serialization), tempfile (tests)

**Spec:** `docs/superpowers/specs/2026-03-27-agents-md-management-design.md`

---

## File Structure

### New files

| File | Responsibility |
|---|---|
| `crates/ion-skill/src/agents.rs` | Core agents logic: `ensure_agent_symlinks()`, `fetch_template()`, `check_template_update()`, `AgentsConfig`, `AgentsLockEntry` |
| `src/commands/agents.rs` | CLI commands: `ion agents init`, `ion agents update`, `ion agents diff` |
| `tests/agents_integration.rs` | Integration tests for all agents subcommands and symlink behavior |

### Modified files

| File | Change |
|---|---|
| `crates/ion-skill/src/manifest.rs` | Add `agents: Option<AgentsConfig>` to `Manifest` |
| `crates/ion-skill/src/lockfile.rs` | Add `agents: Option<AgentsLockEntry>` to `Lockfile` |
| `crates/ion-skill/src/manifest_writer.rs` | Add `write_agents_config()` function |
| `crates/ion-skill/src/lib.rs` | Add `pub mod agents;` |
| `src/main.rs` | Add `Agents` variant to `Commands` enum + dispatch |
| `src/commands/mod.rs` | Add `pub mod agents;` |
| `src/commands/init.rs` | Call `ensure_agent_symlinks()` after target setup |
| `src/commands/install.rs` | Call `ensure_agent_symlinks()` after skill installation |
| `src/commands/update.rs` | Call agents template update after skill updates |

---

## Chunk 1: Data Model (Manifest + Lockfile)

### Task 1: Add AgentsConfig to Manifest

**Files:**
- Modify: `crates/ion-skill/src/agents.rs` (create)
- Modify: `crates/ion-skill/src/manifest.rs:153-161`
- Modify: `crates/ion-skill/src/lib.rs:1-16`
- Test: `crates/ion-skill/src/manifest.rs` (inline tests)

- [ ] **Step 1: Write the failing test — parse manifest with [agents] section**

Add to the `#[cfg(test)] mod tests` block in `crates/ion-skill/src/manifest.rs`:

```rust
#[test]
fn parse_agents_config() {
    let toml_str = r#"
[skills]

[agents]
template = "org/agents-templates"
rev = "v2.0"
path = "templates/AGENTS.md"
"#;
    let manifest = Manifest::parse(toml_str).unwrap();
    let agents = manifest.agents.as_ref().unwrap();
    assert_eq!(agents.template.as_deref(), Some("org/agents-templates"));
    assert_eq!(agents.rev.as_deref(), Some("v2.0"));
    assert_eq!(agents.path.as_deref(), Some("templates/AGENTS.md"));
}

#[test]
fn parse_manifest_without_agents() {
    let toml_str = "[skills]\n";
    let manifest = Manifest::parse(toml_str).unwrap();
    assert!(manifest.agents.is_none());
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo nextest run -E 'test(parse_agents_config)' -E 'test(parse_manifest_without_agents)'`
Expected: FAIL — `AgentsConfig` type does not exist

- [ ] **Step 3: Create agents module and AgentsConfig struct**

Create `crates/ion-skill/src/agents.rs`:

```rust
use serde::{Deserialize, Serialize};

/// Configuration for AGENTS.md template management.
/// Parsed from [agents] in Ion.toml.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct AgentsConfig {
    /// Template source (GitHub shorthand, Git URL, HTTP, or local path)
    #[serde(default)]
    pub template: Option<String>,
    /// Pin to a specific git revision
    #[serde(default)]
    pub rev: Option<String>,
    /// Path to AGENTS.md within the source repo (default: "AGENTS.md" at root)
    #[serde(default)]
    pub path: Option<String>,
}
```

- [ ] **Step 4: Add agents module to lib.rs**

In `crates/ion-skill/src/lib.rs`, add `pub mod agents;` after the existing module declarations (line 1).

- [ ] **Step 5: Add agents field to Manifest struct**

In `crates/ion-skill/src/manifest.rs`, add to the `Manifest` struct (after line 160):

```rust
#[serde(default, skip_serializing_if = "Option::is_none")]
pub agents: Option<crate::agents::AgentsConfig>,
```

Also update `Manifest::empty()` to include `agents: None`.

- [ ] **Step 6: Run tests to verify they pass**

Run: `cargo nextest run -E 'test(parse_agents_config)' -E 'test(parse_manifest_without_agents)'`
Expected: PASS

- [ ] **Step 7: Verify all existing tests still pass**

Run: `cargo nextest run`
Expected: All tests pass (the new optional field doesn't break existing parsing)

- [ ] **Step 8: Commit**

```bash
git add crates/ion-skill/src/agents.rs crates/ion-skill/src/manifest.rs crates/ion-skill/src/lib.rs
git commit -m "feat: add AgentsConfig struct and [agents] section to Manifest"
```

### Task 2: Add AgentsLockEntry to Lockfile

**Files:**
- Modify: `crates/ion-skill/src/agents.rs`
- Modify: `crates/ion-skill/src/lockfile.rs:29-33`
- Test: `crates/ion-skill/src/lockfile.rs` (inline tests)

- [ ] **Step 1: Write the failing test — parse lockfile with [agents] section**

Add to `#[cfg(test)] mod tests` in `crates/ion-skill/src/lockfile.rs`:

```rust
#[test]
fn parse_lockfile_with_agents() {
    let content = r#"
[[skill]]
name = "brainstorming"
source = "https://github.com/obra/superpowers.git"
path = "brainstorming"
commit = "abc123"

[agents]
template = "org/agents-templates"
rev = "def456"
checksum = "sha256:deadbeef"
updated-at = "2026-03-27T00:00:00Z"
"#;
    let lockfile: Lockfile = toml::from_str(content).unwrap();
    assert_eq!(lockfile.skills.len(), 1);
    let agents = lockfile.agents.as_ref().unwrap();
    assert_eq!(agents.template, "org/agents-templates");
    assert_eq!(agents.rev.as_deref(), Some("def456"));
    assert_eq!(agents.checksum, "sha256:deadbeef");
    assert_eq!(agents.updated_at, "2026-03-27T00:00:00Z");
}

#[test]
fn parse_lockfile_without_agents_is_backward_compatible() {
    let content = r#"
[[skill]]
name = "test"
source = "https://github.com/org/repo.git"
"#;
    let lockfile: Lockfile = toml::from_str(content).unwrap();
    assert!(lockfile.agents.is_none());
    assert_eq!(lockfile.skills.len(), 1);
}

#[test]
fn roundtrip_lockfile_with_agents() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("Ion.lock");

    let mut lockfile = Lockfile::default();
    lockfile.agents = Some(crate::agents::AgentsLockEntry {
        template: "org/agents-templates".to_string(),
        rev: Some("abc123".to_string()),
        checksum: "sha256:deadbeef".to_string(),
        updated_at: "2026-03-27T00:00:00Z".to_string(),
    });

    lockfile.write_to(&path).unwrap();
    let loaded = Lockfile::from_file(&path).unwrap();
    let agents = loaded.agents.unwrap();
    assert_eq!(agents.template, "org/agents-templates");
    assert_eq!(agents.checksum, "sha256:deadbeef");
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo nextest run -E 'test(parse_lockfile_with_agents)' -E 'test(parse_lockfile_without_agents_is_backward_compatible)' -E 'test(roundtrip_lockfile_with_agents)'`
Expected: FAIL — `AgentsLockEntry` does not exist

- [ ] **Step 3: Add AgentsLockEntry struct to agents.rs**

Append to `crates/ion-skill/src/agents.rs`:

```rust
/// Lock entry for the AGENTS.md template.
/// Tracks the last-synced state in Ion.lock.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct AgentsLockEntry {
    pub template: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rev: Option<String>,
    pub checksum: String,
    pub updated_at: String, // ISO 8601, stored as plain string
}
```

- [ ] **Step 4: Add agents field to Lockfile struct**

In `crates/ion-skill/src/lockfile.rs`, add to `Lockfile` struct (after line 32):

```rust
#[serde(default, skip_serializing_if = "Option::is_none")]
pub agents: Option<crate::agents::AgentsLockEntry>,
```

- [ ] **Step 5: Run tests to verify they pass**

Run: `cargo nextest run -E 'test(parse_lockfile_with_agents)' -E 'test(parse_lockfile_without_agents_is_backward_compatible)' -E 'test(roundtrip_lockfile_with_agents)'`
Expected: PASS

- [ ] **Step 6: Run all tests**

Run: `cargo nextest run`
Expected: All pass

- [ ] **Step 7: Commit**

```bash
git add crates/ion-skill/src/agents.rs crates/ion-skill/src/lockfile.rs
git commit -m "feat: add AgentsLockEntry to Lockfile for template tracking"
```

### Task 3: Add write_agents_config to manifest_writer

**Files:**
- Modify: `crates/ion-skill/src/manifest_writer.rs`
- Test: `crates/ion-skill/src/manifest_writer.rs` (inline tests)

- [ ] **Step 1: Write the failing test**

Add to `#[cfg(test)] mod tests` in `crates/ion-skill/src/manifest_writer.rs`:

```rust
#[test]
fn write_agents_config_to_manifest() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("Ion.toml");
    std::fs::write(&path, "[skills]\nbrainstorming = \"obra/superpowers/brainstorming\"\n").unwrap();

    write_agents_config(&path, "org/agents-templates", None, None).unwrap();

    let content = std::fs::read_to_string(&path).unwrap();
    assert!(content.contains("[agents]"));
    assert!(content.contains("template = \"org/agents-templates\""));
    assert!(content.contains("brainstorming"), "existing skills preserved");
}

#[test]
fn write_agents_config_with_rev_and_path() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("Ion.toml");
    std::fs::write(&path, "[skills]\n").unwrap();

    write_agents_config(&path, "org/agents-templates", Some("v2.0"), Some("templates/AGENTS.md")).unwrap();

    let content = std::fs::read_to_string(&path).unwrap();
    assert!(content.contains("template = \"org/agents-templates\""));
    assert!(content.contains("rev = \"v2.0\""));
    assert!(content.contains("path = \"templates/AGENTS.md\""));
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo nextest run -E 'test(write_agents_config)'`
Expected: FAIL — function does not exist

- [ ] **Step 3: Implement write_agents_config**

Add to `crates/ion-skill/src/manifest_writer.rs`:

```rust
/// Write an [agents] section to an Ion.toml file.
/// Creates the file with a [skills] section if it doesn't exist.
/// Preserves all existing content.
pub fn write_agents_config(
    manifest_path: &Path,
    template: &str,
    rev: Option<&str>,
    path: Option<&str>,
) -> Result<String> {
    let content =
        std::fs::read_to_string(manifest_path).unwrap_or_else(|_| "[skills]\n".to_string());
    let mut doc: DocumentMut = content.parse().map_err(Error::TomlEdit)?;

    if !doc.contains_key("skills") {
        doc["skills"] = Item::Table(Table::new());
    }

    let mut agents_table = Table::new();
    agents_table["template"] = value(template);
    if let Some(r) = rev {
        agents_table["rev"] = value(r);
    }
    if let Some(p) = path {
        agents_table["path"] = value(p);
    }
    doc["agents"] = Item::Table(agents_table);

    let result = doc.to_string();
    std::fs::write(manifest_path, &result).map_err(Error::Io)?;
    Ok(result)
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo nextest run -E 'test(write_agents_config)'`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add crates/ion-skill/src/manifest_writer.rs
git commit -m "feat: add write_agents_config to manifest_writer"
```

---

## Chunk 2: Symlink Management

### Task 4: Implement ensure_agent_symlinks

**Files:**
- Modify: `crates/ion-skill/src/agents.rs`
- Test: `crates/ion-skill/src/agents.rs` (inline tests)

- [ ] **Step 1: Write the failing tests**

Add to `crates/ion-skill/src/agents.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    #[test]
    fn creates_claude_symlink_when_agents_md_exists() {
        let project = tempfile::tempdir().unwrap();
        std::fs::write(project.path().join("AGENTS.md"), "# My Agents\n").unwrap();

        let mut targets = BTreeMap::new();
        targets.insert("claude".to_string(), ".claude/skills".to_string());

        ensure_agent_symlinks(project.path(), &targets).unwrap();

        let symlink = project.path().join("CLAUDE.md");
        assert!(symlink.exists(), "CLAUDE.md symlink should exist");
        assert!(symlink.symlink_metadata().unwrap().is_symlink());
    }

    #[test]
    fn no_symlink_when_agents_md_missing() {
        let project = tempfile::tempdir().unwrap();

        let mut targets = BTreeMap::new();
        targets.insert("claude".to_string(), ".claude/skills".to_string());

        ensure_agent_symlinks(project.path(), &targets).unwrap();

        assert!(!project.path().join("CLAUDE.md").exists());
    }

    #[test]
    fn no_symlink_for_non_claude_target() {
        let project = tempfile::tempdir().unwrap();
        std::fs::write(project.path().join("AGENTS.md"), "# Agents\n").unwrap();

        let mut targets = BTreeMap::new();
        targets.insert("cursor".to_string(), ".cursor/skills".to_string());

        ensure_agent_symlinks(project.path(), &targets).unwrap();

        assert!(!project.path().join("CLAUDE.md").exists());
    }

    #[test]
    fn skips_existing_regular_file() {
        let project = tempfile::tempdir().unwrap();
        std::fs::write(project.path().join("AGENTS.md"), "# Agents\n").unwrap();
        std::fs::write(project.path().join("CLAUDE.md"), "# Existing\n").unwrap();

        let mut targets = BTreeMap::new();
        targets.insert("claude".to_string(), ".claude/skills".to_string());

        // Should not error, just skip
        ensure_agent_symlinks(project.path(), &targets).unwrap();

        // Verify it's still a regular file, not a symlink
        let meta = std::fs::symlink_metadata(project.path().join("CLAUDE.md")).unwrap();
        assert!(!meta.is_symlink());
    }

    #[test]
    fn skips_existing_symlink_pointing_elsewhere() {
        let project = tempfile::tempdir().unwrap();
        std::fs::write(project.path().join("AGENTS.md"), "# Agents\n").unwrap();
        std::fs::write(project.path().join("OTHER.md"), "# Other\n").unwrap();

        #[cfg(unix)]
        std::os::unix::fs::symlink("OTHER.md", project.path().join("CLAUDE.md")).unwrap();

        let mut targets = BTreeMap::new();
        targets.insert("claude".to_string(), ".claude/skills".to_string());

        // Should not error, just skip
        ensure_agent_symlinks(project.path(), &targets).unwrap();

        // Verify it still points to OTHER.md, not AGENTS.md
        let target = std::fs::read_link(project.path().join("CLAUDE.md")).unwrap();
        assert_eq!(target, std::path::Path::new("OTHER.md"));
    }

    #[test]
    fn idempotent_symlink_creation() {
        let project = tempfile::tempdir().unwrap();
        std::fs::write(project.path().join("AGENTS.md"), "# Agents\n").unwrap();

        let mut targets = BTreeMap::new();
        targets.insert("claude".to_string(), ".claude/skills".to_string());

        ensure_agent_symlinks(project.path(), &targets).unwrap();
        ensure_agent_symlinks(project.path(), &targets).unwrap();

        let symlink = project.path().join("CLAUDE.md");
        assert!(symlink.symlink_metadata().unwrap().is_symlink());
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo nextest run -E 'test(creates_claude_symlink)' -E 'test(no_symlink_when_agents_md_missing)' -E 'test(no_symlink_for_non_claude_target)' -E 'test(skips_existing_regular_file)' -E 'test(idempotent_symlink_creation)'`
Expected: FAIL — `ensure_agent_symlinks` does not exist

- [ ] **Step 3: Implement ensure_agent_symlinks**

Add to `crates/ion-skill/src/agents.rs`:

```rust
use std::collections::BTreeMap;
use std::path::Path;

use crate::Result;

/// Mapping of target names to the agent instructions filename that needs a symlink.
/// Only targets whose tools don't read AGENTS.md natively need an entry here.
const AGENT_FILE_SYMLINKS: &[(&str, &str)] = &[
    ("claude", "CLAUDE.md"),
];

/// For each configured target that has an entry in AGENT_FILE_SYMLINKS,
/// create a symlink (e.g. CLAUDE.md -> AGENTS.md) if AGENTS.md exists
/// and the symlink doesn't.
///
/// Symlinks are only created for targets configured in [options.targets].
/// If a target filename already exists as a regular file or a symlink
/// pointing elsewhere, a warning is printed and it is skipped.
pub fn ensure_agent_symlinks(project_dir: &Path, targets: &BTreeMap<String, String>) -> Result<()> {
    let agents_md = project_dir.join("AGENTS.md");
    if !agents_md.exists() {
        return Ok(());
    }

    for (target_name, symlink_filename) in AGENT_FILE_SYMLINKS {
        if !targets.contains_key(*target_name) {
            continue;
        }

        let symlink_path = project_dir.join(symlink_filename);

        // Check if something already exists at the symlink path
        match std::fs::symlink_metadata(&symlink_path) {
            Ok(meta) if meta.is_symlink() => {
                // Already a symlink — check if it points to AGENTS.md
                if let Ok(target) = std::fs::read_link(&symlink_path) {
                    if target == std::path::Path::new("AGENTS.md") {
                        continue; // Already correct
                    }
                }
                eprintln!(
                    "Warning: {} already exists as a symlink pointing elsewhere, skipping",
                    symlink_filename
                );
                continue;
            }
            Ok(_) => {
                // Regular file exists
                eprintln!(
                    "Warning: {} already exists as a file, skipping symlink \
                     (remove it manually if you want ion to manage it)",
                    symlink_filename
                );
                continue;
            }
            Err(_) => {
                // Doesn't exist — create it
            }
        }

        #[cfg(unix)]
        std::os::unix::fs::symlink("AGENTS.md", &symlink_path)
            .map_err(crate::Error::Io)?;
    }

    Ok(())
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo nextest run -E 'test(creates_claude_symlink)' -E 'test(no_symlink_when_agents_md_missing)' -E 'test(no_symlink_for_non_claude_target)' -E 'test(skips_existing_regular_file)' -E 'test(idempotent_symlink_creation)'`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add crates/ion-skill/src/agents.rs
git commit -m "feat: add ensure_agent_symlinks for CLAUDE.md -> AGENTS.md"
```

### Task 5: Integrate symlinks into ion init

**Files:**
- Modify: `src/commands/init.rs:155-188`
- Test: `tests/agents_integration.rs` (create)

- [ ] **Step 1: Write the failing integration test**

Create `tests/agents_integration.rs`:

```rust
use std::process::Command;

fn ion_cmd() -> Command {
    Command::new(env!("CARGO_BIN_EXE_ion"))
}

#[test]
fn init_creates_claude_symlink_when_agents_md_exists() {
    let project = tempfile::tempdir().unwrap();
    std::fs::write(project.path().join("AGENTS.md"), "# My Agents\n").unwrap();

    let output = ion_cmd()
        .args(["init", "--target", "claude"])
        .current_dir(project.path())
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "init failed: stdout={stdout}\nstderr={stderr}"
    );

    let symlink = project.path().join("CLAUDE.md");
    assert!(symlink.exists(), "CLAUDE.md should exist after init");
    assert!(
        symlink.symlink_metadata().unwrap().is_symlink(),
        "CLAUDE.md should be a symlink"
    );
}

#[test]
fn init_no_symlink_without_agents_md() {
    let project = tempfile::tempdir().unwrap();

    let output = ion_cmd()
        .args(["init", "--target", "claude"])
        .current_dir(project.path())
        .output()
        .unwrap();
    assert!(output.status.success());

    assert!(
        !project.path().join("CLAUDE.md").exists(),
        "CLAUDE.md should not exist without AGENTS.md"
    );
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo nextest run -E 'test(init_creates_claude_symlink_when_agents_md_exists)' -E 'test(init_no_symlink_without_agents_md)'`
Expected: FAIL — init does not create symlinks yet

- [ ] **Step 3: Add symlink call to init.rs**

In `src/commands/init.rs`, immediately after the `ctx.ensure_builtin_skill(&merged_options);` call (line 160) and **before** the `if json {` early return block (line 162), add:

```rust
// Create agent file symlinks (e.g. CLAUDE.md -> AGENTS.md)
if let Err(e) = ion_skill::agents::ensure_agent_symlinks(&ctx.project_dir, &merged_options.targets) {
    eprintln!("Warning: failed to create agent symlinks: {e}");
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo nextest run -E 'test(init_creates_claude_symlink)' -E 'test(init_no_symlink_without_agents_md)'`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src/commands/init.rs tests/agents_integration.rs
git commit -m "feat: create CLAUDE.md symlink during ion init"
```

### Task 6: Integrate symlinks into ion add (install-all)

**Files:**
- Modify: `src/commands/install.rs:246-258`
- Test: `tests/agents_integration.rs`

- [ ] **Step 1: Write the failing integration test**

Add to `tests/agents_integration.rs`:

```rust
#[test]
fn install_all_creates_claude_symlink() {
    let project = tempfile::tempdir().unwrap();
    std::fs::write(project.path().join("AGENTS.md"), "# My Agents\n").unwrap();
    std::fs::write(
        project.path().join("Ion.toml"),
        "[skills]\n\n[options.targets]\nclaude = \".claude/skills\"\n",
    )
    .unwrap();

    let output = ion_cmd()
        .args(["add"])
        .current_dir(project.path())
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "add failed: stdout={stdout}\nstderr={stderr}"
    );

    let symlink = project.path().join("CLAUDE.md");
    assert!(symlink.exists(), "CLAUDE.md should exist after install-all");
    assert!(symlink.symlink_metadata().unwrap().is_symlink());
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo nextest run -E 'test(install_all_creates_claude_symlink)'`
Expected: FAIL

- [ ] **Step 3: Add symlink call to install.rs**

In `src/commands/install.rs`, after the lockfile write at line 246 and **before** the `if json {` early return block (line 248), add:

```rust
// Create agent file symlinks (e.g. CLAUDE.md -> AGENTS.md)
if let Err(e) = ion_skill::agents::ensure_agent_symlinks(&ctx.project_dir, &merged_options.targets) {
    eprintln!("Warning: failed to create agent symlinks: {e}");
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo nextest run -E 'test(install_all_creates_claude_symlink)'`
Expected: PASS

- [ ] **Step 5: Run all tests**

Run: `cargo nextest run`
Expected: All pass

- [ ] **Step 6: Commit**

```bash
git add src/commands/install.rs tests/agents_integration.rs
git commit -m "feat: ensure agent symlinks during install-all"
```

---

## Chunk 3: Template Sourcing (ion agents init)

### Task 7: Implement template fetch logic in agents.rs

**Files:**
- Modify: `crates/ion-skill/src/agents.rs`
- Test: `crates/ion-skill/src/agents.rs` (inline tests)

- [ ] **Step 1: Write the failing test**

Add to the `#[cfg(test)] mod tests` in `crates/ion-skill/src/agents.rs`:

```rust
#[test]
fn fetch_template_from_local_path() {
    let template_dir = tempfile::tempdir().unwrap();
    std::fs::write(
        template_dir.path().join("AGENTS.md"),
        "# Template Agents\n\nStandard workflows.\n",
    )
    .unwrap();

    let project = tempfile::tempdir().unwrap();

    let result = fetch_template(
        template_dir.path().to_str().unwrap(),
        None, // rev
        None, // path within repo
        project.path(),
    )
    .unwrap();

    assert_eq!(result.content, "# Template Agents\n\nStandard workflows.\n");
}

#[test]
fn fetch_template_with_custom_path() {
    let template_dir = tempfile::tempdir().unwrap();
    let subdir = template_dir.path().join("templates");
    std::fs::create_dir(&subdir).unwrap();
    std::fs::write(subdir.join("AGENTS.md"), "# Custom Path\n").unwrap();

    let project = tempfile::tempdir().unwrap();

    let result = fetch_template(
        template_dir.path().to_str().unwrap(),
        None,
        Some("templates/AGENTS.md"),
        project.path(),
    )
    .unwrap();

    assert_eq!(result.content, "# Custom Path\n");
}

#[test]
fn fetch_template_missing_file_errors() {
    let template_dir = tempfile::tempdir().unwrap();
    // No AGENTS.md file
    let project = tempfile::tempdir().unwrap();

    let result = fetch_template(
        template_dir.path().to_str().unwrap(),
        None,
        None,
        project.path(),
    );

    assert!(result.is_err());
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo nextest run -E 'test(fetch_template)'`
Expected: FAIL — function does not exist

- [ ] **Step 3: Implement fetch_template**

Add to `crates/ion-skill/src/agents.rs`:

```rust
use std::path::PathBuf;

use crate::source::SkillSource;
use crate::installer;
use crate::git;
use crate::Error;

/// Result of fetching an AGENTS.md template
pub struct FetchedTemplate {
    pub content: String,
    pub rev: Option<String>,
    pub checksum: String,
}

/// Fetch an AGENTS.md template from a source.
///
/// Resolves the source using SkillSource::infer, fetches the repo/path,
/// and extracts the AGENTS.md file at the specified path (default: root).
pub fn fetch_template(
    source_str: &str,
    rev: Option<&str>,
    file_path: Option<&str>,
    _project_dir: &Path,
) -> Result<FetchedTemplate> {
    let mut source = SkillSource::infer(source_str)?;
    if let Some(r) = rev {
        source.rev = Some(r.to_string());
    }

    let agents_md_path = file_path.unwrap_or("AGENTS.md");

    let repo_dir = fetch_source_base(&source)?;
    let template_file = repo_dir.join(agents_md_path);

    if !template_file.exists() {
        return Err(Error::Other(format!(
            "AGENTS.md not found in {} at path '{}'",
            source_str, agents_md_path
        )));
    }

    let content = std::fs::read_to_string(&template_file).map_err(Error::Io)?;
    let checksum = checksum_content(content.as_bytes());

    // Get the commit hash for git sources
    let resolved_rev = match source.source_type {
        crate::source::SourceType::Github | crate::source::SourceType::Git => {
            git::head_commit(&repo_dir).ok()
        }
        _ => None,
    };

    Ok(FetchedTemplate {
        content,
        rev: resolved_rev,
        checksum,
    })
}

/// Fetch source base directory — reuses installer's git clone/cache logic.
fn fetch_source_base(source: &SkillSource) -> Result<PathBuf> {
    match source.source_type {
        crate::source::SourceType::Github | crate::source::SourceType::Git => {
            let url = source.git_url()?;
            let repo_hash = format!("{:x}", installer::hash_simple(&url));
            let repo_dir = installer::data_dir().join(&repo_hash);
            git::clone_or_fetch(&url, &repo_dir)?;
            if let Some(ref rev) = source.rev {
                git::checkout(&repo_dir, rev)?;
            }
            Ok(repo_dir)
        }
        crate::source::SourceType::Path => {
            let path = PathBuf::from(&source.source);
            if !path.exists() {
                return Err(Error::Source(format!(
                    "Local path does not exist: {}",
                    source.source
                )));
            }
            Ok(path)
        }
        _ => Err(Error::Source(format!(
            "Source type {:?} is not supported for AGENTS.md templates",
            source.source_type
        ))),
    }
}
```

The `checksum_content` helper and `git::head_commit` are used above. `git::head_commit` exists in `crates/ion-skill/src/git.rs:53`. `checksum_content` must be added as a local helper in `agents.rs` — add this alongside the other functions:

```rust
/// SHA-256 checksum of raw content, formatted as "sha256:{hex}".
fn checksum_content(content: &[u8]) -> String {
    use sha2::{Sha256, Digest};
    let hash = Sha256::new().chain_update(content).finalize();
    format!("sha256:{:x}", hash)
}
```

The `sha2` crate is already a workspace dependency used by `ion-skill`.

Also add a timestamp helper (since `chrono` is not a dependency):

```rust
/// Current UTC time as ISO 8601 string (e.g. "2026-03-27T12:00:00Z").
pub fn now_iso8601() -> String {
    use std::time::SystemTime;
    let dur = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default();
    let secs = dur.as_secs();
    // Simple UTC breakdown
    let days = secs / 86400;
    let time_secs = secs % 86400;
    let hours = time_secs / 3600;
    let minutes = (time_secs % 3600) / 60;
    let seconds = time_secs % 60;
    // Days since epoch to Y-M-D (simplified)
    let (year, month, day) = epoch_days_to_ymd(days);
    format!("{year:04}-{month:02}-{day:02}T{hours:02}:{minutes:02}:{seconds:02}Z")
}

fn epoch_days_to_ymd(days: u64) -> (u64, u64, u64) {
    // Adapted from Howard Hinnant's algorithm
    let z = days + 719468;
    let era = z / 146097;
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    (y, m, d)
}
```

Both helpers live in `crates/ion-skill/src/agents.rs` alongside the other functions. `now_iso8601` must be `pub fn` since it's called from the CLI crate (`src/commands/agents.rs`) as `ion_skill::agents::now_iso8601()`. `checksum_content` can remain private (only used within the same module).

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo nextest run -E 'test(fetch_template)'`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add crates/ion-skill/src/agents.rs
git commit -m "feat: add fetch_template for AGENTS.md template sourcing"
```

### Task 8: Add ion agents subcommand group and init command

**Files:**
- Create: `src/commands/agents.rs`
- Modify: `src/commands/mod.rs`
- Modify: `src/main.rs`
- Test: `tests/agents_integration.rs`

- [ ] **Step 1: Write the failing integration test**

Add to `tests/agents_integration.rs`:

```rust
#[test]
fn agents_init_from_local_path() {
    let project = tempfile::tempdir().unwrap();
    std::fs::write(project.path().join("Ion.toml"), "[skills]\n").unwrap();

    let template_dir = tempfile::tempdir().unwrap();
    std::fs::write(
        template_dir.path().join("AGENTS.md"),
        "# Org Standard Agents\n\nDo things the org way.\n",
    )
    .unwrap();

    let output = ion_cmd()
        .args([
            "agents",
            "init",
            template_dir.path().to_str().unwrap(),
        ])
        .current_dir(project.path())
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "agents init failed: stdout={stdout}\nstderr={stderr}"
    );

    // Should copy AGENTS.md as starting point
    let agents_md = project.path().join("AGENTS.md");
    assert!(agents_md.exists(), "AGENTS.md should be created");
    let content = std::fs::read_to_string(&agents_md).unwrap();
    assert!(content.contains("Org Standard Agents"));

    // Should write [agents] to Ion.toml
    let manifest = std::fs::read_to_string(project.path().join("Ion.toml")).unwrap();
    assert!(manifest.contains("[agents]"));
    assert!(manifest.contains("template"));

    // Should write to Ion.lock
    let lockfile = std::fs::read_to_string(project.path().join("Ion.lock")).unwrap();
    assert!(lockfile.contains("[agents]"));
    assert!(lockfile.contains("checksum"));
}

#[test]
fn agents_init_preserves_existing_agents_md() {
    let project = tempfile::tempdir().unwrap();
    std::fs::write(project.path().join("Ion.toml"), "[skills]\n").unwrap();
    std::fs::write(
        project.path().join("AGENTS.md"),
        "# My Custom Agents\n\nMy local content.\n",
    )
    .unwrap();

    let template_dir = tempfile::tempdir().unwrap();
    std::fs::write(
        template_dir.path().join("AGENTS.md"),
        "# Org Template\n",
    )
    .unwrap();

    let output = ion_cmd()
        .args([
            "agents",
            "init",
            template_dir.path().to_str().unwrap(),
        ])
        .current_dir(project.path())
        .output()
        .unwrap();
    assert!(output.status.success());

    // Original AGENTS.md should be preserved
    let content = std::fs::read_to_string(project.path().join("AGENTS.md")).unwrap();
    assert!(content.contains("My Custom Agents"));

    // Upstream should be staged
    let upstream = project.path().join(".agents/templates/AGENTS.md.upstream");
    assert!(upstream.exists());
}

#[test]
fn agents_init_errors_when_already_configured() {
    let project = tempfile::tempdir().unwrap();
    std::fs::write(
        project.path().join("Ion.toml"),
        "[skills]\n\n[agents]\ntemplate = \"org/old\"\n",
    )
    .unwrap();

    let template_dir = tempfile::tempdir().unwrap();
    std::fs::write(template_dir.path().join("AGENTS.md"), "# New\n").unwrap();

    let output = ion_cmd()
        .args([
            "agents",
            "init",
            template_dir.path().to_str().unwrap(),
        ])
        .current_dir(project.path())
        .output()
        .unwrap();
    assert!(!output.status.success(), "should error when already configured");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("already configured"));
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo nextest run -E 'test(agents_init)'`
Expected: FAIL — `agents` subcommand does not exist

- [ ] **Step 3: Add agents subcommand to main.rs**

In `src/main.rs`, add to the `Commands` enum (after `Update`, around line 98):

```rust
/// Manage AGENTS.md templates
Agents {
    #[command(subcommand)]
    action: AgentsCommands,
},
```

Add the `AgentsCommands` enum (after `CacheCommands`, around line 225):

```rust
#[derive(Subcommand)]
enum AgentsCommands {
    /// Set up AGENTS.md template sourcing from a remote repository
    Init {
        /// Template source (e.g., org/repo, git URL, or local path)
        source: String,
        /// Pin to a specific git ref (branch, tag, or commit SHA)
        #[arg(long)]
        rev: Option<String>,
        /// Path to AGENTS.md within the source repo (default: AGENTS.md at root)
        #[arg(long)]
        path: Option<String>,
    },
    /// Fetch latest upstream template and stage for merging
    Update,
    /// Show diff between local AGENTS.md and upstream template
    Diff,
}
```

Add the dispatch in the `match cli.command` block:

```rust
Commands::Agents { action } => match action {
    AgentsCommands::Init { source, rev, path } => {
        commands::agents::init(&source, rev.as_deref(), path.as_deref(), json)
    }
    AgentsCommands::Update => commands::agents::update(json),
    AgentsCommands::Diff => commands::agents::diff(),
},
```

- [ ] **Step 4: Add agents module to commands/mod.rs**

Add `pub mod agents;` to `src/commands/mod.rs`.

- [ ] **Step 5: Create src/commands/agents.rs with init command**

```rust
use crate::context::ProjectContext;
use crate::style::Paint;

pub fn init(source: &str, rev: Option<&str>, path: Option<&str>, json: bool) -> anyhow::Result<()> {
    let ctx = ProjectContext::load()?;
    let p = Paint::new(&ctx.global_config);

    // Check if [agents] already exists
    let manifest = ctx.manifest_or_empty()?;
    if manifest.agents.is_some() {
        anyhow::bail!(
            "Template already configured in Ion.toml. \
             Use `ion agents update` to fetch the latest, \
             or edit [agents] in Ion.toml manually to change the source."
        );
    }

    // Resolve source through global config aliases
    let resolved_source = ctx.global_config.resolve_source(source);

    // Fetch template
    let fetched = ion_skill::agents::fetch_template(
        &resolved_source,
        rev,
        path,
        &ctx.project_dir,
    )?;

    let agents_md_path = ctx.project_dir.join("AGENTS.md");
    let upstream_dir = ctx.project_dir.join(".agents/templates");
    let upstream_path = upstream_dir.join("AGENTS.md.upstream");
    let already_existed = agents_md_path.exists();

    if already_existed {
        // Existing AGENTS.md — stage upstream for merging
        std::fs::create_dir_all(&upstream_dir)?;
        std::fs::write(&upstream_path, &fetched.content)?;
        if !json {
            println!(
                "{} AGENTS.md already exists — upstream template staged to {}",
                p.warn("note:"),
                p.dim(".agents/templates/AGENTS.md.upstream")
            );
            println!("  Merge changes manually or ask your agent to help.");
        }
    } else {
        // No existing AGENTS.md — copy as starting point
        std::fs::write(&agents_md_path, &fetched.content)?;
        if !json {
            println!("{} AGENTS.md from template", p.success("Created"));
        }
    }

    // Write [agents] to Ion.toml
    ion_skill::manifest_writer::write_agents_config(
        &ctx.manifest_path,
        source,
        rev,
        path,
    )?;

    // Write lock entry
    let mut lockfile = ctx.lockfile()?;
    lockfile.agents = Some(ion_skill::agents::AgentsLockEntry {
        template: source.to_string(),
        rev: fetched.rev,
        checksum: fetched.checksum,
        updated_at: ion_skill::agents::now_iso8601(),
    });
    lockfile.write_to(&ctx.lockfile_path)?;

    // Add specific gitignore entry for the upstream staging file
    let entries = [".agents/templates/AGENTS.md.upstream"];
    let missing = ion_skill::gitignore::find_missing_gitignore_entries(&ctx.project_dir, &entries)?;
    if !missing.is_empty() {
        let refs: Vec<&str> = missing.iter().map(|s| s.as_str()).collect();
        ion_skill::gitignore::append_to_gitignore(&ctx.project_dir, &refs)?;
    }

    // Create agent symlinks (e.g. CLAUDE.md -> AGENTS.md)
    let merged_options = ctx.merged_options(&manifest);
    if let Err(e) = ion_skill::agents::ensure_agent_symlinks(&ctx.project_dir, &merged_options.targets) {
        eprintln!("Warning: failed to create agent symlinks: {e}");
    }

    if !json {
        println!("  {} Ion.toml with template source", p.success("Updated"));
    }

    if json {
        crate::json::print_success(serde_json::json!({
            "template": source,
            "agents_md_created": !already_existed,
        }));
    }

    Ok(())
}

pub fn update(_json: bool) -> anyhow::Result<()> {
    // Placeholder — implemented in Task 9
    anyhow::bail!("not yet implemented")
}

pub fn diff() -> anyhow::Result<()> {
    // Placeholder — implemented in Task 10
    anyhow::bail!("not yet implemented")
}
```

Note: `now_iso8601()` and `checksum_content()` are helpers defined in `crates/ion-skill/src/agents.rs` (see Task 7, Step 3 for their implementation).

- [ ] **Step 6: Run tests to verify they pass**

Run: `cargo nextest run -E 'test(agents_init)'`
Expected: PASS

- [ ] **Step 7: Run all tests**

Run: `cargo nextest run`
Expected: All pass

- [ ] **Step 8: Commit**

```bash
git add src/main.rs src/commands/mod.rs src/commands/agents.rs tests/agents_integration.rs
git commit -m "feat: add ion agents init command for template sourcing"
```

---

## Chunk 4: Template Update & Diff

### Task 9: Implement ion agents update

**Files:**
- Modify: `src/commands/agents.rs`
- Modify: `src/commands/update.rs`
- Test: `tests/agents_integration.rs`

- [ ] **Step 1: Write the failing integration test**

Add to `tests/agents_integration.rs`:

```rust
#[test]
fn agents_update_detects_changes() {
    let project = tempfile::tempdir().unwrap();
    let template_dir = tempfile::tempdir().unwrap();
    std::fs::write(
        template_dir.path().join("AGENTS.md"),
        "# Version 1\n",
    )
    .unwrap();

    // Init the template
    let output = ion_cmd()
        .args([
            "agents", "init",
            template_dir.path().to_str().unwrap(),
        ])
        .current_dir(project.path())
        .output()
        .unwrap();
    assert!(output.status.success());

    // Update the template source
    std::fs::write(
        template_dir.path().join("AGENTS.md"),
        "# Version 2\n\nNew content.\n",
    )
    .unwrap();

    // Run agents update
    let output = ion_cmd()
        .args(["agents", "update"])
        .current_dir(project.path())
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "agents update failed: stdout={stdout}\nstderr={stderr}"
    );

    // Upstream file should contain new content
    let upstream = std::fs::read_to_string(
        project.path().join(".agents/templates/AGENTS.md.upstream"),
    )
    .unwrap();
    assert!(upstream.contains("Version 2"));

    // Local AGENTS.md should NOT be changed
    let local = std::fs::read_to_string(project.path().join("AGENTS.md")).unwrap();
    assert!(local.contains("Version 1"));
}

#[test]
fn agents_update_noop_when_unchanged() {
    let project = tempfile::tempdir().unwrap();
    let template_dir = tempfile::tempdir().unwrap();
    std::fs::write(template_dir.path().join("AGENTS.md"), "# Same\n").unwrap();

    let output = ion_cmd()
        .args(["agents", "init", template_dir.path().to_str().unwrap()])
        .current_dir(project.path())
        .output()
        .unwrap();
    assert!(output.status.success());

    // Update without changes
    let output = ion_cmd()
        .args(["agents", "update"])
        .current_dir(project.path())
        .output()
        .unwrap();
    assert!(output.status.success());

    // .agents/templates/AGENTS.md.upstream should not exist (no changes to stage)
    // OR should contain the same content — either is acceptable
}

#[test]
fn agents_update_errors_without_config() {
    let project = tempfile::tempdir().unwrap();
    std::fs::write(project.path().join("Ion.toml"), "[skills]\n").unwrap();

    let output = ion_cmd()
        .args(["agents", "update"])
        .current_dir(project.path())
        .output()
        .unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("No [agents]") || stderr.contains("no agents") || stderr.contains("not configured"),
        "should error about missing config: {stderr}"
    );
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo nextest run -E 'test(agents_update)'`
Expected: FAIL

- [ ] **Step 3: Implement the update command**

Replace the `update` function in `src/commands/agents.rs`:

```rust
pub fn update(json: bool) -> anyhow::Result<()> {
    let ctx = ProjectContext::load()?;
    let p = Paint::new(&ctx.global_config);
    ctx.require_manifest()?;

    let manifest = ctx.manifest()?;
    let agents_config = manifest
        .agents
        .as_ref()
        .and_then(|a| a.template.as_ref())
        .ok_or_else(|| anyhow::anyhow!(
            "No [agents] template configured in Ion.toml. Run `ion agents init <source>` first."
        ))?;

    let resolved_source = ctx.global_config.resolve_source(agents_config);
    let agents = manifest.agents.as_ref().unwrap();

    let fetched = ion_skill::agents::fetch_template(
        &resolved_source,
        agents.rev.as_deref(),
        agents.path.as_deref(),
        &ctx.project_dir,
    )?;

    // Compare with locked checksum
    let mut lockfile = ctx.lockfile()?;
    let old_checksum = lockfile.agents.as_ref().map(|a| a.checksum.as_str());
    let old_rev = lockfile.agents.as_ref().and_then(|a| a.rev.as_deref()).unwrap_or("unknown");

    if old_checksum == Some(fetched.checksum.as_str()) {
        if !json {
            println!("agents: {} up to date with upstream", p.dim("AGENTS.md"));
        }
        return Ok(());
    }

    // Stage the new upstream
    let upstream_dir = ctx.project_dir.join(".agents/templates");
    std::fs::create_dir_all(&upstream_dir)?;
    let upstream_path = upstream_dir.join("AGENTS.md.upstream");
    std::fs::write(&upstream_path, &fetched.content)?;

    let new_rev = fetched.rev.as_deref().unwrap_or("unknown");

    // Update lockfile
    lockfile.agents = Some(ion_skill::agents::AgentsLockEntry {
        template: agents_config.clone(),
        rev: fetched.rev,
        checksum: fetched.checksum,
        updated_at: ion_skill::agents::now_iso8601(),
    });
    lockfile.write_to(&ctx.lockfile_path)?;

    if json {
        crate::json::print_success(serde_json::json!({
            "updated": true,
            "old_rev": old_rev,
            "new_rev": new_rev,
            "upstream_path": upstream_path.display().to_string(),
        }));
    } else {
        println!(
            "agents: upstream template updated ({} → {})",
            p.dim(&old_rev[..7.min(old_rev.len())]),
            p.info(&new_rev[..7.min(new_rev.len())])
        );
        println!(
            "  upstream saved to {}",
            p.dim(".agents/templates/AGENTS.md.upstream")
        );
        println!("  run your agent to merge, or manually diff:");
        println!(
            "    {}",
            p.bold("ion agents diff")
        );
    }

    Ok(())
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo nextest run -E 'test(agents_update)'`
Expected: PASS

- [ ] **Step 5: Integrate into ion update**

In `src/commands/update.rs`, after the lockfile write (line 184) and before the JSON output (line 187), add:

```rust
// Check for agents template update (non-fatal)
if manifest.agents.as_ref().and_then(|a| a.template.as_ref()).is_some() {
    if let Err(e) = crate::commands::agents::update_template_non_fatal(
        &ctx, &mut lockfile, &p, json,
    ) {
        if !json {
            println!(
                "  {} agents template: {}",
                p.warn("⚠"),
                p.warn(&e.to_string())
            );
        }
    }
}
```

Add a helper to `src/commands/agents.rs`:

```rust
/// Template update logic for use within `ion update`. Non-fatal — returns
/// errors for the caller to display as warnings.
pub fn update_template_non_fatal(
    ctx: &ProjectContext,
    lockfile: &mut ion_skill::lockfile::Lockfile,
    p: &Paint,
    json: bool,
) -> anyhow::Result<()> {
    let manifest = ctx.manifest()?;
    let agents_config = manifest
        .agents
        .as_ref()
        .and_then(|a| a.template.as_ref())
        .ok_or_else(|| anyhow::anyhow!("no agents template configured"))?;

    let resolved_source = ctx.global_config.resolve_source(agents_config);
    let agents = manifest.agents.as_ref().unwrap();

    let fetched = ion_skill::agents::fetch_template(
        &resolved_source,
        agents.rev.as_deref(),
        agents.path.as_deref(),
        &ctx.project_dir,
    )?;

    let old_checksum = lockfile.agents.as_ref().map(|a| a.checksum.as_str());
    if old_checksum == Some(fetched.checksum.as_str()) {
        return Ok(()); // Unchanged — silent
    }

    let upstream_dir = ctx.project_dir.join(".agents/templates");
    std::fs::create_dir_all(&upstream_dir)?;
    std::fs::write(upstream_dir.join("AGENTS.md.upstream"), &fetched.content)?;

    let old_rev = lockfile.agents.as_ref().and_then(|a| a.rev.as_deref()).unwrap_or("unknown");
    let new_rev = fetched.rev.as_deref().unwrap_or("unknown");

    lockfile.agents = Some(ion_skill::agents::AgentsLockEntry {
        template: agents_config.clone(),
        rev: fetched.rev,
        checksum: fetched.checksum,
        updated_at: ion_skill::agents::now_iso8601(),
    });

    if !json {
        println!(
            "  {} agents template: {} → {}",
            p.success("✓"),
            old_rev.get(..7).unwrap_or(old_rev),
            p.info(new_rev.get(..7).unwrap_or(new_rev))
        );
        println!(
            "    upstream saved to {}",
            p.dim(".agents/templates/AGENTS.md.upstream")
        );
    }

    Ok(())
}
```

- [ ] **Step 6: Run all tests**

Run: `cargo nextest run`
Expected: All pass

- [ ] **Step 7: Commit**

```bash
git add src/commands/agents.rs src/commands/update.rs
git commit -m "feat: add ion agents update and integrate with ion update"
```

### Task 10: Implement ion agents diff

**Files:**
- Modify: `src/commands/agents.rs`
- Test: `tests/agents_integration.rs`

- [ ] **Step 1: Write the failing integration test**

Add to `tests/agents_integration.rs`:

```rust
#[test]
fn agents_diff_shows_differences() {
    let project = tempfile::tempdir().unwrap();
    std::fs::write(project.path().join("AGENTS.md"), "# Local version\n").unwrap();
    std::fs::create_dir_all(project.path().join(".agents/templates")).unwrap();
    std::fs::write(
        project.path().join(".agents/templates/AGENTS.md.upstream"),
        "# Upstream version\n",
    )
    .unwrap();
    std::fs::write(project.path().join("Ion.toml"), "[skills]\n").unwrap();

    let output = ion_cmd()
        .args(["agents", "diff"])
        .current_dir(project.path())
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    // Should show some diff output
    assert!(!stdout.is_empty() || !String::from_utf8_lossy(&output.stderr).is_empty());
}

#[test]
fn agents_diff_errors_without_upstream() {
    let project = tempfile::tempdir().unwrap();
    std::fs::write(project.path().join("AGENTS.md"), "# Local\n").unwrap();
    std::fs::write(project.path().join("Ion.toml"), "[skills]\n").unwrap();

    let output = ion_cmd()
        .args(["agents", "diff"])
        .current_dir(project.path())
        .output()
        .unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("no upstream") || stderr.contains("not found") || stderr.contains("update"),
        "should mention missing upstream: {stderr}"
    );
}

#[test]
fn agents_diff_reports_up_to_date() {
    let project = tempfile::tempdir().unwrap();
    std::fs::write(project.path().join("AGENTS.md"), "# Same content\n").unwrap();
    std::fs::create_dir_all(project.path().join(".agents/templates")).unwrap();
    std::fs::write(
        project.path().join(".agents/templates/AGENTS.md.upstream"),
        "# Same content\n",
    )
    .unwrap();
    std::fs::write(project.path().join("Ion.toml"), "[skills]\n").unwrap();

    let output = ion_cmd()
        .args(["agents", "diff"])
        .current_dir(project.path())
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("up to date"),
        "should say up to date: {stdout}"
    );
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo nextest run -E 'test(agents_diff)'`
Expected: FAIL

- [ ] **Step 3: Implement the diff command**

Replace the `diff` function in `src/commands/agents.rs`:

```rust
pub fn diff() -> anyhow::Result<()> {
    let ctx = ProjectContext::load()?;

    let agents_md = ctx.project_dir.join("AGENTS.md");
    let upstream_path = ctx.project_dir.join(".agents/templates/AGENTS.md.upstream");

    if !upstream_path.exists() {
        anyhow::bail!(
            "No upstream template staged. Run `ion agents update` first."
        );
    }

    if !agents_md.exists() {
        anyhow::bail!("No AGENTS.md found in project root.");
    }

    let local_content = std::fs::read_to_string(&agents_md)?;
    let upstream_content = std::fs::read_to_string(&upstream_path)?;

    if local_content == upstream_content {
        println!("AGENTS.md is up to date with upstream.");
        return Ok(());
    }

    // Use diff command for colorized output
    let status = std::process::Command::new("diff")
        .args(["-u", "--label", "local/AGENTS.md", "--label", "upstream/AGENTS.md"])
        .arg(&agents_md)
        .arg(&upstream_path)
        .status();

    match status {
        Ok(s) if s.code() == Some(1) => {
            // diff returns 1 when files differ — that's expected
            Ok(())
        }
        Ok(s) if s.success() => {
            // diff returns 0 when files are identical (shouldn't reach here due to check above)
            println!("AGENTS.md is up to date with upstream.");
            Ok(())
        }
        Ok(s) => {
            anyhow::bail!("diff command failed with exit code: {:?}", s.code())
        }
        Err(_) => {
            // diff not available — fall back to simple comparison
            println!("--- local/AGENTS.md");
            println!("+++ upstream/AGENTS.md");
            println!("(files differ — install `diff` for detailed output)");
            Ok(())
        }
    }
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo nextest run -E 'test(agents_diff)'`
Expected: PASS

- [ ] **Step 5: Run all tests**

Run: `cargo nextest run`
Expected: All pass

- [ ] **Step 6: Commit**

```bash
git add src/commands/agents.rs tests/agents_integration.rs
git commit -m "feat: add ion agents diff command"
```

---

## Chunk 5: Built-in Skill & Final Polish

### Task 11: Create agents-update built-in skill

**Files:**
- Modify: `src/builtin_skill.rs` (or create new file `src/agents_skill.rs`)
- Modify: `src/commands/agents.rs` (deploy skill during init)

- [ ] **Step 1: Define the agents-update SKILL.md content**

The skill is a pure-text SKILL.md embedded in the binary. Add to `src/commands/agents.rs` (or a dedicated constants file):

```rust
const AGENTS_UPDATE_SKILL_CONTENT: &str = r#"---
name: agents-update
description: Merge upstream AGENTS.md template changes into local AGENTS.md
---

# AGENTS.md Template Update

When the user asks to update or merge their AGENTS.md with upstream changes, follow this process:

## Prerequisites

Check that `.agents/templates/AGENTS.md.upstream` exists. If it doesn't, tell the user to run `ion agents update` first.

## Process

1. Read `.agents/templates/AGENTS.md.upstream` (the new upstream template version)
2. Read `AGENTS.md` (the current local version)
3. Compare the two files and identify:
   - Sections added in upstream that don't exist locally
   - Sections modified in upstream that the user hasn't changed
   - Sections the user has customized (preserve these)
4. Intelligently merge:
   - Add new upstream sections
   - Update unchanged sections to match upstream
   - Preserve user customizations
   - Flag conflicts where both upstream and local changed the same section
5. Write the merged result to `AGENTS.md`
6. Inform the user what changed

## Guidelines

- Always preserve user customizations over upstream changes when they conflict
- Add clear comments if you're unsure about a merge decision
- Show the user what you changed before writing
"#;
```

- [ ] **Step 2: Deploy the skill during ion agents init**

In the `init` function in `src/commands/agents.rs`, after writing the lockfile, add:

```rust
// Deploy agents-update built-in skill
deploy_agents_update_skill(&ctx, &merged_options)?;
```

Add the helper:

```rust
fn deploy_agents_update_skill(
    ctx: &ProjectContext,
    options: &ion_skill::manifest::ManifestOptions,
) -> anyhow::Result<()> {
    use ion_skill::installer::{SkillInstaller, builtin_skills_dir};

    let skill_name = "agents-update";
    let global_dir = builtin_skills_dir().join(skill_name);
    let global_skill_md = global_dir.join("SKILL.md");

    // Write/update SKILL.md in global storage
    let needs_write = if global_skill_md.exists() {
        std::fs::read_to_string(&global_skill_md).ok().as_deref() != Some(AGENTS_UPDATE_SKILL_CONTENT)
    } else {
        true
    };

    if needs_write {
        std::fs::create_dir_all(&global_dir)?;
        std::fs::write(&global_skill_md, AGENTS_UPDATE_SKILL_CONTENT)?;
    }

    // Deploy symlinks
    let installer = SkillInstaller::new(&ctx.project_dir, options);
    installer.deploy(skill_name, &global_dir)?;

    // Gitignore the symlinks
    let target_paths: Vec<&str> = options.targets.values().map(|s| s.as_str()).collect();
    ion_skill::gitignore::add_skill_entries(&ctx.project_dir, skill_name, &target_paths)?;

    // Register as local skill in Ion.toml
    let content = std::fs::read_to_string(&ctx.manifest_path).unwrap_or_default();
    if !content.contains(&format!("{skill_name} ="))
        && !content.contains(&format!("\"{skill_name}\""))
    {
        let source = ion_skill::source::SkillSource::local();
        ion_skill::manifest_writer::add_skill(&ctx.manifest_path, skill_name, &source)?;
    }

    Ok(())
}
```

- [ ] **Step 3: Write integration test**

Add to `tests/agents_integration.rs`:

```rust
#[test]
fn agents_init_deploys_agents_update_skill() {
    let project = tempfile::tempdir().unwrap();
    std::fs::write(project.path().join("Ion.toml"), "[skills]\n\n[options.targets]\nclaude = \".claude/skills\"\n").unwrap();

    let template_dir = tempfile::tempdir().unwrap();
    std::fs::write(template_dir.path().join("AGENTS.md"), "# Template\n").unwrap();

    let output = ion_cmd()
        .args(["agents", "init", template_dir.path().to_str().unwrap()])
        .current_dir(project.path())
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "failed: stdout={stdout}\nstderr={stderr}"
    );

    // agents-update skill should be deployed
    assert!(
        project.path().join(".agents/skills/agents-update/SKILL.md").exists(),
        "agents-update skill should be deployed"
    );

    // Should also be registered in Ion.toml
    let manifest = std::fs::read_to_string(project.path().join("Ion.toml")).unwrap();
    assert!(manifest.contains("agents-update"));
}
```

- [ ] **Step 4: Run test**

Run: `cargo nextest run -E 'test(agents_init_deploys_agents_update_skill)'`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src/commands/agents.rs tests/agents_integration.rs
git commit -m "feat: deploy agents-update built-in skill during ion agents init"
```

### Task 12: Conditionally deploy agents-update skill in init and install-all

**Files:**
- Modify: `src/commands/init.rs`
- Modify: `src/commands/install.rs`
- Modify: `src/commands/agents.rs` (make `deploy_agents_update_skill` public)

The spec requires that when someone clones a repo that already has `[agents]` configured and runs `ion init` or `ion add`, the `agents-update` skill is deployed.

- [ ] **Step 1: Make deploy function public**

In `src/commands/agents.rs`, change `fn deploy_agents_update_skill` to `pub fn deploy_agents_update_skill`.

- [ ] **Step 2: Add conditional deploy to init.rs**

In `src/commands/init.rs`, after the `ensure_agent_symlinks` call added in Task 5, add:

```rust
// Deploy agents-update skill if [agents] template is configured
if manifest.agents.as_ref().and_then(|a| a.template.as_ref()).is_some() {
    if let Err(e) = crate::commands::agents::deploy_agents_update_skill(&ctx, &merged_options) {
        log::warn!("Failed to deploy agents-update skill: {e}");
    }
}
```

- [ ] **Step 3: Add conditional deploy to install.rs**

In `src/commands/install.rs`, after the `ensure_agent_symlinks` call added in Task 6, add:

```rust
// Deploy agents-update skill if [agents] template is configured
if manifest.agents.as_ref().and_then(|a| a.template.as_ref()).is_some() {
    if let Err(e) = crate::commands::agents::deploy_agents_update_skill(&ctx, &merged_options) {
        log::warn!("Failed to deploy agents-update skill: {e}");
    }
}
```

- [ ] **Step 4: Write integration test**

Add to `tests/agents_integration.rs`:

```rust
#[test]
fn install_all_deploys_agents_update_skill_when_configured() {
    let project = tempfile::tempdir().unwrap();
    let template_dir = tempfile::tempdir().unwrap();
    std::fs::write(template_dir.path().join("AGENTS.md"), "# Template\n").unwrap();

    // Set up project with [agents] already configured (simulating a repo clone)
    std::fs::write(
        project.path().join("Ion.toml"),
        &format!(
            "[skills]\n\n[agents]\ntemplate = \"{}\"\n\n[options.targets]\nclaude = \".claude/skills\"\n",
            template_dir.path().display()
        ),
    )
    .unwrap();

    let output = ion_cmd()
        .args(["add"])
        .current_dir(project.path())
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "add failed: stdout={stdout}\nstderr={stderr}"
    );

    assert!(
        project.path().join(".agents/skills/agents-update/SKILL.md").exists(),
        "agents-update skill should be deployed by install-all"
    );
}
```

- [ ] **Step 5: Run test and verify**

Run: `cargo nextest run -E 'test(install_all_deploys_agents_update_skill_when_configured)'`
Expected: PASS

- [ ] **Step 6: Commit**

```bash
git add src/commands/init.rs src/commands/install.rs src/commands/agents.rs tests/agents_integration.rs
git commit -m "feat: deploy agents-update skill conditionally from init and install-all"
```

### Task 13: Pre-commit checks and final validation

**Files:** All modified files

- [ ] **Step 1: Run cargo fmt**

Run: `cargo fmt --all`

- [ ] **Step 2: Run cargo clippy**

Run: `cargo clippy --all-targets --all-features -- -D warnings`
Fix any warnings.

- [ ] **Step 3: Run all tests**

Run: `cargo nextest run`
Expected: All pass

- [ ] **Step 4: Verify help output**

Run: `cargo run -- agents --help`
Expected: Shows init, update, diff subcommands with descriptions

Run: `cargo run -- agents init --help`
Expected: Shows source positional arg, --rev, --path flags

- [ ] **Step 5: Final commit (if any formatting/clippy fixes)**

```bash
git add -A
git commit -m "chore: apply fmt and clippy fixes for agents feature"
```

---

## Implementation Notes

### Dependencies — all verified

1. **sha2 crate** — workspace dependency, used in `git.rs`. Available for use in `agents.rs`.
2. **GlobalConfig::resolve_source** — exists at `crates/ion-skill/src/config.rs:86`.
3. **git::head_commit** — exists at `crates/ion-skill/src/git.rs:53`. Returns `Result<String>` with the commit hash.
4. **Timestamps** — use the custom `now_iso8601()` helper (defined in Task 7) since `chrono` is not a dependency.

### Key patterns to follow

- **Error handling:** `anyhow::Result` in CLI commands, `ion_skill::Result` (thiserror) in library code
- **Output style:** Use `Paint` for colored output, support `--json` flag
- **Testing:** Integration tests invoke the binary via `env!("CARGO_BIN_EXE_ion")`, unit tests use `tempfile::tempdir()`
- **Manifest writing:** Use `toml_edit::DocumentMut` to preserve formatting
- **Gitignore:** Use `ion_skill::gitignore` functions, idempotent

### What NOT to implement

- Windows symlink support (not a supported platform)
- HTTP-only templates (Git and Path sources only for v1)
- Automatic merge (agent-assisted merge is the user's responsibility)
- Template removal command (edit Ion.toml manually)
