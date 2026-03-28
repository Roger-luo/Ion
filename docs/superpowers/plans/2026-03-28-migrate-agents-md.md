# AGENTS.md/CLAUDE.md Migration & Management Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make CLAUDE.md a gitignored symlink to AGENTS.md, and add migration support for projects with manual CLAUDE.md files.

**Architecture:** Two-part change — (1) modify `ensure_agent_symlinks` in `agents.rs` to gitignore CLAUDE.md and auto-replace pointer files, (2) add an AGENTS.md/CLAUDE.md conversion phase to `ion migrate` that handles real-content files with interactive prompts.

**Tech Stack:** Rust, ion-skill crate, integration tests with `tempfile`

---

## File Structure

| File | Action | Responsibility |
|------|--------|---------------|
| `crates/ion-skill/src/agents.rs` | Modify | Add `is_agents_pointer()`, update `ensure_agent_symlinks` for pointer detection + gitignore |
| `crates/ion-skill/src/gitignore.rs` | Modify | Add `ensure_agent_file_ignored()` |
| `src/commands/migrate.rs` | Modify | Add `migrate_agents_md()` phase before skill discovery |
| `tests/migrate_integration.rs` | Modify | Add integration tests for AGENTS.md/CLAUDE.md migration |

---

### Task 1: Add `ensure_agent_file_ignored` to gitignore.rs

**Files:**
- Modify: `crates/ion-skill/src/gitignore.rs:86` (after `add_skill_entries`, before `remove_skill_entries`)

- [ ] **Step 1: Write the failing test**

Add at the end of the existing `mod tests` block in `crates/ion-skill/src/gitignore.rs`:

```rust
#[test]
fn ensure_agent_file_ignored_adds_entry() {
    let project = tempfile::tempdir().unwrap();

    ensure_agent_file_ignored(project.path(), "CLAUDE.md").unwrap();

    let content = std::fs::read_to_string(project.path().join(".gitignore")).unwrap();
    assert!(content.contains("CLAUDE.md"));
    assert!(content.contains("# Managed by ion"));
}

#[test]
fn ensure_agent_file_ignored_is_idempotent() {
    let project = tempfile::tempdir().unwrap();

    ensure_agent_file_ignored(project.path(), "CLAUDE.md").unwrap();
    ensure_agent_file_ignored(project.path(), "CLAUDE.md").unwrap();

    let content = std::fs::read_to_string(project.path().join(".gitignore")).unwrap();
    let count = content.matches("CLAUDE.md").count();
    assert_eq!(count, 1, "should not duplicate entries");
}

#[test]
fn ensure_agent_file_ignored_preserves_existing() {
    let project = tempfile::tempdir().unwrap();
    std::fs::write(project.path().join(".gitignore"), "node_modules/\n").unwrap();

    ensure_agent_file_ignored(project.path(), "CLAUDE.md").unwrap();

    let content = std::fs::read_to_string(project.path().join(".gitignore")).unwrap();
    assert!(content.contains("node_modules/"));
    assert!(content.contains("CLAUDE.md"));
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo nextest run -E 'test(ensure_agent_file_ignored)' -p ion-skill`
Expected: FAIL — `ensure_agent_file_ignored` does not exist

- [ ] **Step 3: Write the implementation**

Add this function in `crates/ion-skill/src/gitignore.rs` after the `add_skill_entries` function (before `remove_skill_entries`):

```rust
/// Ensure a single file (e.g. `CLAUDE.md`) is listed in `.gitignore`.
/// Idempotent — won't duplicate an existing entry.
pub fn ensure_agent_file_ignored(project_dir: &Path, filename: &str) -> Result<()> {
    let gitignore_path = project_dir.join(".gitignore");
    let mut content = std::fs::read_to_string(&gitignore_path).unwrap_or_default();

    // Already present — nothing to do
    if content.lines().any(|l| l.trim() == filename) {
        return Ok(());
    }

    // Ensure trailing newline
    if !content.is_empty() && !content.ends_with('\n') {
        content.push('\n');
    }

    // Add managed section header if not present
    if !content.contains("# Managed by ion") {
        content.push_str("\n# Managed by ion\n");
    }

    content.push_str(filename);
    content.push('\n');

    std::fs::write(&gitignore_path, &content).map_err(Error::Io)?;
    Ok(())
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo nextest run -E 'test(ensure_agent_file_ignored)' -p ion-skill`
Expected: All 3 tests PASS

- [ ] **Step 5: Commit**

```bash
git add crates/ion-skill/src/gitignore.rs
git commit -m "feat: add ensure_agent_file_ignored to gitignore module"
```

---

### Task 2: Add `is_agents_pointer` to agents.rs

**Files:**
- Modify: `crates/ion-skill/src/agents.rs:93` (after `ensure_agent_symlinks`, before `checksum_content`)

- [ ] **Step 1: Write the failing tests**

Add at the end of the existing `mod tests` block in `crates/ion-skill/src/agents.rs`:

```rust
#[test]
fn pointer_bare_reference() {
    assert!(is_agents_pointer("@AGENTS.md"));
}

#[test]
fn pointer_with_prose() {
    assert!(is_agents_pointer("treat @AGENTS.md the same as this file"));
}

#[test]
fn pointer_with_whitespace() {
    assert!(is_agents_pointer("\n  @AGENTS.md  \n\n"));
}

#[test]
fn pointer_multiline_prose() {
    assert!(is_agents_pointer(
        "Contents of @AGENTS.md\n\ntreat @AGENTS.md the same as this file"
    ));
}

#[test]
fn not_pointer_no_reference() {
    assert!(!is_agents_pointer("# My Project\n\nSome instructions.\n"));
}

#[test]
fn not_pointer_has_extra_content() {
    assert!(!is_agents_pointer(
        "treat @AGENTS.md the same as this file\n\n# Extra Rules\n\nAlways use TypeScript.\n"
    ));
}

#[test]
fn not_pointer_empty() {
    assert!(!is_agents_pointer(""));
}

#[test]
fn not_pointer_only_whitespace() {
    assert!(!is_agents_pointer("   \n\n  "));
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo nextest run -E 'test(pointer)' -p ion-skill`
Expected: FAIL — `is_agents_pointer` does not exist

- [ ] **Step 3: Write the implementation**

Add this function in `crates/ion-skill/src/agents.rs` after the closing brace of `ensure_agent_symlinks` (line 93), before `checksum_content`:

```rust
/// Check whether a file's content is just a pointer to AGENTS.md.
///
/// Returns `true` if every non-blank line's only purpose is to reference
/// `@AGENTS.md` — e.g. the bare string `@AGENTS.md` or prose like
/// "treat @AGENTS.md the same as this file".
///
/// Returns `false` if the file contains additional instructions beyond
/// the reference, has no `@AGENTS.md` at all, or is empty.
pub fn is_agents_pointer(content: &str) -> bool {
    let non_blank: Vec<&str> = content
        .lines()
        .filter(|l| !l.trim().is_empty())
        .collect();

    if non_blank.is_empty() {
        return false;
    }

    // Must contain @AGENTS.md somewhere
    if !non_blank.iter().any(|l| l.contains("@AGENTS.md")) {
        return false;
    }

    // Every non-blank line must mention @AGENTS.md
    non_blank.iter().all(|l| l.contains("@AGENTS.md"))
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo nextest run -E 'test(pointer)' -p ion-skill`
Expected: All 8 tests PASS

- [ ] **Step 5: Commit**

```bash
git add crates/ion-skill/src/agents.rs
git commit -m "feat: add is_agents_pointer detection function"
```

---

### Task 3: Update `ensure_agent_symlinks` — pointer detection + gitignore

**Files:**
- Modify: `crates/ion-skill/src/agents.rs:49-93` (the `ensure_agent_symlinks` function)

This task changes two behaviors:
1. When CLAUDE.md is a regular file AND a pointer, delete it and create the symlink (instead of warning and skipping).
2. After creating or verifying a correct symlink, call `gitignore::ensure_agent_file_ignored`.

- [ ] **Step 1: Write the failing tests**

Add at the end of the existing `mod tests` block in `crates/ion-skill/src/agents.rs`:

```rust
#[test]
fn replaces_pointer_file_with_symlink() {
    let project = tempfile::tempdir().unwrap();
    std::fs::write(project.path().join("AGENTS.md"), "# Agents\n").unwrap();
    std::fs::write(project.path().join("CLAUDE.md"), "@AGENTS.md").unwrap();

    let mut targets = BTreeMap::new();
    targets.insert("claude".to_string(), ".claude/skills".to_string());

    ensure_agent_symlinks(project.path(), &targets).unwrap();

    let meta = std::fs::symlink_metadata(project.path().join("CLAUDE.md")).unwrap();
    assert!(meta.is_symlink(), "pointer file should be replaced with symlink");
}

#[test]
fn skips_real_content_file() {
    let project = tempfile::tempdir().unwrap();
    std::fs::write(project.path().join("AGENTS.md"), "# Agents\n").unwrap();
    std::fs::write(
        project.path().join("CLAUDE.md"),
        "# My Project Rules\n\nAlways use Rust.\n",
    )
    .unwrap();

    let mut targets = BTreeMap::new();
    targets.insert("claude".to_string(), ".claude/skills".to_string());

    ensure_agent_symlinks(project.path(), &targets).unwrap();

    let meta = std::fs::symlink_metadata(project.path().join("CLAUDE.md")).unwrap();
    assert!(!meta.is_symlink(), "real content file should NOT be replaced");
}

#[test]
fn symlink_creation_gitignores_claude_md() {
    let project = tempfile::tempdir().unwrap();
    std::fs::write(project.path().join("AGENTS.md"), "# Agents\n").unwrap();

    let mut targets = BTreeMap::new();
    targets.insert("claude".to_string(), ".claude/skills".to_string());

    ensure_agent_symlinks(project.path(), &targets).unwrap();

    let gitignore = std::fs::read_to_string(project.path().join(".gitignore")).unwrap();
    assert!(gitignore.contains("CLAUDE.md"));
}

#[test]
fn existing_correct_symlink_gitignores_claude_md() {
    let project = tempfile::tempdir().unwrap();
    std::fs::write(project.path().join("AGENTS.md"), "# Agents\n").unwrap();

    #[cfg(unix)]
    std::os::unix::fs::symlink("AGENTS.md", project.path().join("CLAUDE.md")).unwrap();

    let mut targets = BTreeMap::new();
    targets.insert("claude".to_string(), ".claude/skills".to_string());

    ensure_agent_symlinks(project.path(), &targets).unwrap();

    let gitignore = std::fs::read_to_string(project.path().join(".gitignore")).unwrap();
    assert!(gitignore.contains("CLAUDE.md"));
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo nextest run -E 'test(replaces_pointer_file) | test(skips_real_content) | test(symlink_creation_gitignores) | test(existing_correct_symlink_gitignores)' -p ion-skill`
Expected: FAIL — new behavior not yet implemented

- [ ] **Step 3: Update the implementation**

Replace the `ensure_agent_symlinks` function body in `crates/ion-skill/src/agents.rs` with:

```rust
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

        match std::fs::symlink_metadata(&symlink_path) {
            Ok(meta) if meta.is_symlink() => {
                if let Ok(target) = std::fs::read_link(&symlink_path)
                    && target == std::path::Path::new("AGENTS.md")
                {
                    // Already correct — ensure gitignored
                    crate::gitignore::ensure_agent_file_ignored(project_dir, symlink_filename)?;
                    continue;
                }
                eprintln!(
                    "Warning: {} already exists as a symlink pointing elsewhere, skipping",
                    symlink_filename
                );
                continue;
            }
            Ok(_) => {
                // Regular file — check if it's a pointer
                let content =
                    std::fs::read_to_string(&symlink_path).unwrap_or_default();
                if is_agents_pointer(&content) {
                    std::fs::remove_file(&symlink_path).map_err(crate::Error::Io)?;
                    // Fall through to create symlink below
                } else {
                    eprintln!(
                        "Warning: {} already exists as a file, skipping symlink \
                         (remove it manually or run `ion migrate` to convert)",
                        symlink_filename
                    );
                    continue;
                }
            }
            Err(_) => {
                // Doesn't exist — create it
            }
        }

        #[cfg(unix)]
        std::os::unix::fs::symlink("AGENTS.md", &symlink_path).map_err(crate::Error::Io)?;

        crate::gitignore::ensure_agent_file_ignored(project_dir, symlink_filename)?;
    }

    Ok(())
}
```

- [ ] **Step 4: Run all agents.rs tests to verify they pass**

Run: `cargo nextest run -p ion-skill -E 'test(/agents/)'`
Expected: All tests PASS. Note: the existing `skips_existing_regular_file` test still passes because its CLAUDE.md content (`"# Existing\n"`) is NOT a pointer.

- [ ] **Step 5: Run full test suite to check for regressions**

Run: `cargo nextest run`
Expected: All existing tests PASS

- [ ] **Step 6: Commit**

```bash
git add crates/ion-skill/src/agents.rs
git commit -m "feat: ensure_agent_symlinks replaces pointer files and gitignores CLAUDE.md"
```

---

### Task 4: Add `migrate_agents_md` function to migrate command

**Files:**
- Modify: `src/commands/migrate.rs` (add new function + call it from `run`)

- [ ] **Step 1: Write the `migrate_agents_md` function**

Add at the bottom of `src/commands/migrate.rs` (before the final closing of the module, after `print_skipped`):

```rust
/// Result of AGENTS.md/CLAUDE.md migration phase.
#[derive(Debug)]
enum AgentsMdAction {
    /// Pointer file replaced or fresh symlink created
    Symlinked,
    /// CLAUDE.md renamed to AGENTS.md, backup created
    Renamed { backup: Option<String> },
    /// Both files have real content — user must resolve manually
    Skipped { reason: String },
}

impl AgentsMdAction {
    fn to_json(&self) -> serde_json::Value {
        match self {
            AgentsMdAction::Symlinked => serde_json::json!({"action": "symlinked"}),
            AgentsMdAction::Renamed { backup } => serde_json::json!({
                "action": "renamed",
                "from": "CLAUDE.md",
                "backup": backup,
            }),
            AgentsMdAction::Skipped { reason } => serde_json::json!({
                "action": "skipped",
                "reason": reason,
            }),
        }
    }
}

fn migrate_agents_md(
    project_dir: &std::path::Path,
    targets: &std::collections::BTreeMap<String, String>,
    p: &Paint,
    json: bool,
    yes: bool,
) -> anyhow::Result<Option<AgentsMdAction>> {
    // Only handle CLAUDE.md if claude is a configured target
    if !targets.contains_key("claude") {
        return Ok(None);
    }

    let agents_path = project_dir.join("AGENTS.md");
    let claude_path = project_dir.join("CLAUDE.md");

    let agents_exists = agents_path.exists();

    // Check CLAUDE.md — skip symlinks (already managed)
    let claude_meta = std::fs::symlink_metadata(&claude_path).ok();
    let claude_is_symlink = claude_meta.as_ref().is_some_and(|m| m.is_symlink());
    let claude_is_file = claude_meta.as_ref().is_some_and(|m| m.is_file());

    if claude_is_symlink || !claude_is_file {
        // No CLAUDE.md regular file to migrate — let ensure_agent_symlinks handle it
        return Ok(None);
    }

    // CLAUDE.md is a regular file — read content and classify
    let claude_content = std::fs::read_to_string(&claude_path)?;
    let is_pointer = ion_skill::agents::is_agents_pointer(&claude_content);

    match (agents_exists, is_pointer) {
        // AGENTS.md exists + CLAUDE.md is a pointer → delete and symlink
        (true, true) => {
            if !json {
                println!(
                    "  {} is a pointer to AGENTS.md — replacing with symlink",
                    p.dim("CLAUDE.md")
                );
            }
            std::fs::remove_file(&claude_path)?;
            #[cfg(unix)]
            std::os::unix::fs::symlink("AGENTS.md", &claude_path)?;
            ion_skill::gitignore::ensure_agent_file_ignored(project_dir, "CLAUDE.md")?;
            Ok(Some(AgentsMdAction::Symlinked))
        }

        // AGENTS.md exists + CLAUDE.md has real content → conflict
        (true, false) => {
            if yes || json {
                let reason =
                    "Both AGENTS.md and CLAUDE.md have content — run without --yes to choose which to keep".to_string();
                if !json {
                    println!("  {}", p.dim(&reason));
                }
                return Ok(Some(AgentsMdAction::Skipped { reason }));
            }

            println!();
            println!("Both AGENTS.md and CLAUDE.md exist with content.");
            println!("  (1) Keep AGENTS.md (backup CLAUDE.md to CLAUDE.md.bak)");
            println!("  (2) Keep CLAUDE.md as AGENTS.md (backup AGENTS.md to AGENTS.md.bak)");
            println!("  (3) Skip — I'll handle this manually");
            print!("> ");
            io::stdout().flush()?;

            let mut answer = String::new();
            io::stdin().read_line(&mut answer)?;
            let choice = answer.trim();

            match choice {
                "1" => {
                    std::fs::rename(&claude_path, project_dir.join("CLAUDE.md.bak"))?;
                    #[cfg(unix)]
                    std::os::unix::fs::symlink("AGENTS.md", &claude_path)?;
                    ion_skill::gitignore::ensure_agent_file_ignored(project_dir, "CLAUDE.md")?;
                    if !json {
                        println!(
                            "  Kept AGENTS.md, backed up CLAUDE.md to {}",
                            p.dim("CLAUDE.md.bak")
                        );
                    }
                    Ok(Some(AgentsMdAction::Renamed {
                        backup: Some("CLAUDE.md.bak".to_string()),
                    }))
                }
                "2" => {
                    std::fs::rename(&agents_path, project_dir.join("AGENTS.md.bak"))?;
                    std::fs::rename(&claude_path, &agents_path)?;
                    #[cfg(unix)]
                    std::os::unix::fs::symlink("AGENTS.md", &claude_path)?;
                    ion_skill::gitignore::ensure_agent_file_ignored(project_dir, "CLAUDE.md")?;
                    if !json {
                        println!(
                            "  Renamed CLAUDE.md to AGENTS.md, backed up old AGENTS.md to {}",
                            p.dim("AGENTS.md.bak")
                        );
                    }
                    Ok(Some(AgentsMdAction::Renamed {
                        backup: Some("AGENTS.md.bak".to_string()),
                    }))
                }
                _ => {
                    if !json {
                        println!("  Skipping AGENTS.md/CLAUDE.md migration.");
                    }
                    Ok(Some(AgentsMdAction::Skipped {
                        reason: "user chose to skip".to_string(),
                    }))
                }
            }
        }

        // No AGENTS.md + CLAUDE.md is a pointer → warn
        (false, true) => {
            if !json {
                eprintln!(
                    "Warning: CLAUDE.md references @AGENTS.md but AGENTS.md does not exist."
                );
            }
            Ok(Some(AgentsMdAction::Skipped {
                reason: "pointer to nonexistent AGENTS.md".to_string(),
            }))
        }

        // No AGENTS.md + CLAUDE.md has real content → rename
        (false, false) => {
            if yes || json {
                let reason =
                    "CLAUDE.md has content but no AGENTS.md — run without --yes to confirm rename"
                        .to_string();
                if !json {
                    println!("  {}", p.dim(&reason));
                }
                return Ok(Some(AgentsMdAction::Skipped { reason }));
            }

            println!();
            println!("Found CLAUDE.md but no AGENTS.md.");
            print!("  Rename CLAUDE.md to AGENTS.md and create symlink? [Y/n] ");
            io::stdout().flush()?;

            let mut answer = String::new();
            io::stdin().read_line(&mut answer)?;
            let answer = answer.trim();

            if answer.is_empty()
                || answer.eq_ignore_ascii_case("y")
                || answer.eq_ignore_ascii_case("yes")
            {
                std::fs::rename(&claude_path, &agents_path)?;
                #[cfg(unix)]
                std::os::unix::fs::symlink("AGENTS.md", &claude_path)?;
                ion_skill::gitignore::ensure_agent_file_ignored(project_dir, "CLAUDE.md")?;
                if !json {
                    println!("  Renamed CLAUDE.md to AGENTS.md, created symlink.");
                }
                Ok(Some(AgentsMdAction::Renamed { backup: None }))
            } else {
                if !json {
                    println!("  Skipping AGENTS.md/CLAUDE.md migration.");
                }
                Ok(Some(AgentsMdAction::Skipped {
                    reason: "user chose to skip".to_string(),
                }))
            }
        }
    }
}
```

- [ ] **Step 2: Wire `migrate_agents_md` into the `run` function**

In `src/commands/migrate.rs`, insert the new phase call right after `let merged_options = ...` (line 21) and before the lockfile_path binding (line 23). Add a new block:

```rust
    // ── Phase 0: AGENTS.md / CLAUDE.md conversion ────────────────────────
    let agents_md_action = if !dry_run {
        migrate_agents_md(project_dir, &merged_options.targets, &p, json, yes)?
    } else {
        None
    };
```

Then update the JSON output block near line 382 to include the new field. Add `"agents_md"` to the `serde_json::json!` macro:

```rust
    "agents_md": agents_md_action.as_ref().map(|a| a.to_json()),
```

Also update the `create_migration_commit` candidates array to include `"AGENTS.md"` and `"CLAUDE.md.bak"` and `"AGENTS.md.bak"`:

```rust
    let candidates = ["Ion.toml", "Ion.lock", ".gitignore", ".agents/", "AGENTS.md", "CLAUDE.md"];
```

- [ ] **Step 3: Verify it compiles**

Run: `cargo build`
Expected: Compiles with no errors

- [ ] **Step 4: Run the existing migrate tests to check for regressions**

Run: `cargo nextest run -E 'test(/migrate/)'`
Expected: All existing tests PASS (the new phase is a no-op when there's no CLAUDE.md regular file)

- [ ] **Step 5: Commit**

```bash
git add src/commands/migrate.rs
git commit -m "feat: add migrate_agents_md phase to ion migrate command"
```

---

### Task 5: Integration tests for AGENTS.md/CLAUDE.md migration

**Files:**
- Modify: `tests/migrate_integration.rs`

- [ ] **Step 1: Add test — pointer CLAUDE.md replaced with symlink**

Append to `tests/migrate_integration.rs`:

```rust
#[test]
fn migrate_replaces_pointer_claude_md_with_symlink() {
    let project = tempfile::tempdir().unwrap();

    // Create AGENTS.md and a pointer CLAUDE.md
    std::fs::write(project.path().join("AGENTS.md"), "# My Project\n").unwrap();
    std::fs::write(
        project.path().join("CLAUDE.md"),
        "treat @AGENTS.md the same as this file",
    )
    .unwrap();

    // Create Ion.toml with claude target
    std::fs::write(
        project.path().join("Ion.toml"),
        "[options.targets]\nclaude = \".claude/skills\"\n",
    )
    .unwrap();

    let output = ion_cmd()
        .args(["migrate", "--yes"])
        .current_dir(project.path())
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "migrate pointer failed: stdout={stdout}\nstderr={stderr}"
    );

    // CLAUDE.md should now be a symlink
    let meta = std::fs::symlink_metadata(project.path().join("CLAUDE.md")).unwrap();
    assert!(meta.is_symlink(), "CLAUDE.md should be a symlink");

    // .gitignore should contain CLAUDE.md
    let gitignore = std::fs::read_to_string(project.path().join(".gitignore")).unwrap();
    assert!(gitignore.contains("CLAUDE.md"));
}
```

- [ ] **Step 2: Run test to verify it passes**

Run: `cargo nextest run -E 'test(migrate_replaces_pointer_claude_md_with_symlink)'`
Expected: PASS

- [ ] **Step 3: Add test — skipped when claude not in targets**

Append to `tests/migrate_integration.rs`:

```rust
#[test]
fn migrate_skips_claude_md_when_not_target() {
    let project = tempfile::tempdir().unwrap();

    // Create AGENTS.md and a pointer CLAUDE.md
    std::fs::write(project.path().join("AGENTS.md"), "# My Project\n").unwrap();
    std::fs::write(project.path().join("CLAUDE.md"), "@AGENTS.md").unwrap();

    // Create Ion.toml with cursor target only (no claude)
    std::fs::write(
        project.path().join("Ion.toml"),
        "[options.targets]\ncursor = \".cursor/skills\"\n",
    )
    .unwrap();

    let output = ion_cmd()
        .args(["migrate", "--yes"])
        .current_dir(project.path())
        .output()
        .unwrap();

    assert!(output.status.success());

    // CLAUDE.md should still be a regular file (untouched)
    let meta = std::fs::symlink_metadata(project.path().join("CLAUDE.md")).unwrap();
    assert!(!meta.is_symlink(), "CLAUDE.md should not be touched");
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo nextest run -E 'test(migrate_skips_claude_md_when_not_target)'`
Expected: PASS

- [ ] **Step 5: Add test — yes skips real content conflict**

Append to `tests/migrate_integration.rs`:

```rust
#[test]
fn migrate_yes_skips_real_content_conflict() {
    let project = tempfile::tempdir().unwrap();

    // Both files have real content
    std::fs::write(project.path().join("AGENTS.md"), "# Agents Instructions\n").unwrap();
    std::fs::write(
        project.path().join("CLAUDE.md"),
        "# Claude-specific rules\n\nUse TypeScript.\n",
    )
    .unwrap();

    // Create Ion.toml with claude target
    std::fs::write(
        project.path().join("Ion.toml"),
        "[options.targets]\nclaude = \".claude/skills\"\n",
    )
    .unwrap();

    let output = ion_cmd()
        .args(["migrate", "--yes"])
        .current_dir(project.path())
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "migrate conflict failed: stdout={stdout}\nstderr={stderr}"
    );

    // CLAUDE.md should still be a regular file (not touched because --yes skips conflicts)
    let meta = std::fs::symlink_metadata(project.path().join("CLAUDE.md")).unwrap();
    assert!(!meta.is_symlink(), "real content CLAUDE.md should not be auto-replaced");

    // Content should be preserved
    let content = std::fs::read_to_string(project.path().join("CLAUDE.md")).unwrap();
    assert!(content.contains("Claude-specific rules"));
}
```

- [ ] **Step 6: Run test to verify it passes**

Run: `cargo nextest run -E 'test(migrate_yes_skips_real_content_conflict)'`
Expected: PASS

- [ ] **Step 7: Add test — JSON mode reports agents_md action**

Append to `tests/migrate_integration.rs`:

```rust
#[test]
fn migrate_json_reports_agents_md_pointer() {
    let project = tempfile::tempdir().unwrap();

    std::fs::write(project.path().join("AGENTS.md"), "# Project\n").unwrap();
    std::fs::write(project.path().join("CLAUDE.md"), "@AGENTS.md\n").unwrap();

    std::fs::write(
        project.path().join("Ion.toml"),
        "[options.targets]\nclaude = \".claude/skills\"\n",
    )
    .unwrap();

    let output = ion_cmd()
        .args(["--json", "migrate", "--yes"])
        .current_dir(project.path())
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "json migrate pointer failed: stdout={stdout}\nstderr={stderr}"
    );

    let parsed: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(parsed["data"]["agents_md"]["action"], "symlinked");
}
```

- [ ] **Step 8: Run test to verify it passes**

Run: `cargo nextest run -E 'test(migrate_json_reports_agents_md_pointer)'`
Expected: PASS

- [ ] **Step 9: Run full test suite**

Run: `cargo nextest run`
Expected: All tests PASS

- [ ] **Step 10: Commit**

```bash
git add tests/migrate_integration.rs
git commit -m "test: add integration tests for AGENTS.md/CLAUDE.md migration"
```

---

### Task 6: Lint, format, final verification

**Files:** All modified files

- [ ] **Step 1: Format**

Run: `cargo fmt --all`

- [ ] **Step 2: Lint**

Run: `cargo clippy --all-targets --all-features -- -D warnings`
Expected: No warnings or errors

- [ ] **Step 3: Full test suite**

Run: `cargo nextest run`
Expected: All tests PASS

- [ ] **Step 4: Fix any issues found in steps 1-3, then commit if needed**

```bash
git add -A
git commit -m "style: format and lint fixes"
```

(Only if there were changes to commit.)
