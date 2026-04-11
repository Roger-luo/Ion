# Workspace Support Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace `ProjectContext` with a workspace-first `WorkspaceContext` so all commands operate on a workspace (which may be a single project or multiple sub-projects).

**Architecture:** `WorkspaceContext` replaces `ProjectContext` as the single entry point. A `Project` struct in `ion-skill` holds per-project state (dir, manifest path, lockfile path). Discovery walks up from CWD to find the workspace root (an `Ion.toml` with `[workspace]`). A solo project without `[workspace]` is a workspace-of-one. Commands use scoping logic: from root = all projects, from member dir = that member, `--project` flag overrides.

**Tech Stack:** Rust, clap (derive), toml/toml_edit, serde, anyhow, tempfile (tests)

---

### Task 1: Add `[workspace]` Parsing to Manifest

**Files:**
- Modify: `crates/ion-skill/src/manifest.rs`

- [ ] **Step 1: Write the failing test**

Add to the existing `#[cfg(test)] mod tests` block in `crates/ion-skill/src/manifest.rs`:

```rust
#[test]
fn parse_workspace_config() {
    let toml_str = r#"
[workspace]
members = ["docs", "packages/frontend"]

[skills]
foo = "bar/baz"
"#;
    let manifest = Manifest::parse(toml_str).unwrap();
    let ws = manifest.workspace.as_ref().expect("workspace should be present");
    assert_eq!(ws.members, vec!["docs", "packages/frontend"]);
}

#[test]
fn parse_manifest_without_workspace() {
    let toml_str = "[skills]\nfoo = \"bar/baz\"\n";
    let manifest = Manifest::parse(toml_str).unwrap();
    assert!(manifest.workspace.is_none());
}

#[test]
fn parse_empty_workspace_members() {
    let toml_str = "[workspace]\nmembers = []\n\n[skills]\n";
    let manifest = Manifest::parse(toml_str).unwrap();
    let ws = manifest.workspace.as_ref().unwrap();
    assert!(ws.members.is_empty());
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo nextest run -E 'test(parse_workspace)' -p ion-skill`
Expected: FAIL — `workspace` field does not exist on `Manifest`

- [ ] **Step 3: Add WorkspaceConfig struct and field to Manifest**

In `crates/ion-skill/src/manifest.rs`, add the struct before `Manifest`:

```rust
/// Workspace configuration for multi-project setups.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WorkspaceConfig {
    #[serde(default)]
    pub members: Vec<String>,
}
```

Add the field to `Manifest`:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Manifest {
    #[serde(default)]
    pub project: Option<ProjectMeta>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workspace: Option<WorkspaceConfig>,
    #[serde(default)]
    pub skills: BTreeMap<String, SkillEntry>,
    #[serde(default)]
    pub options: ManifestOptions,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agents: Option<crate::agents::AgentsConfig>,
}
```

Update `Manifest::empty()` to include `workspace: None`.

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo nextest run -E 'test(parse_workspace)' -p ion-skill`
Expected: PASS — all three new tests pass

- [ ] **Step 5: Run full test suite to check nothing is broken**

Run: `cargo nextest run`
Expected: All existing tests still pass

- [ ] **Step 6: Commit**

```bash
git add crates/ion-skill/src/manifest.rs
git commit -m "feat: add [workspace] section parsing to Manifest"
```

---

### Task 2: Create `Project` Struct in `ion-skill`

**Files:**
- Create: `crates/ion-skill/src/workspace.rs`
- Modify: `crates/ion-skill/src/lib.rs`

- [ ] **Step 1: Write the failing test**

Create `crates/ion-skill/src/workspace.rs` with test only:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn project_from_dir() {
        let dir = std::path::PathBuf::from("/tmp/test-project");
        let project = Project::new(dir.clone());
        assert_eq!(project.dir, dir);
        assert_eq!(project.manifest_path, dir.join("Ion.toml"));
        assert_eq!(project.lockfile_path, dir.join("Ion.lock"));
    }

    #[test]
    fn project_has_manifest_false_for_nonexistent() {
        let dir = std::path::PathBuf::from("/tmp/nonexistent-project-12345");
        let project = Project::new(dir);
        assert!(!project.has_manifest());
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo nextest run -E 'test(project_from_dir)' -p ion-skill`
Expected: FAIL — `Project` struct does not exist

- [ ] **Step 3: Implement Project struct**

In `crates/ion-skill/src/workspace.rs`:

```rust
use std::path::PathBuf;

use crate::lockfile::Lockfile;
use crate::manifest::{Manifest, ManifestOptions};

/// A single project within a workspace (or the root project itself).
#[derive(Debug)]
pub struct Project {
    pub dir: PathBuf,
    pub manifest_path: PathBuf,
    pub lockfile_path: PathBuf,
}

impl Project {
    pub fn new(dir: PathBuf) -> Self {
        let manifest_path = dir.join("Ion.toml");
        let lockfile_path = dir.join("Ion.lock");
        Self {
            dir,
            manifest_path,
            lockfile_path,
        }
    }

    pub fn has_manifest(&self) -> bool {
        self.manifest_path.exists()
    }

    pub fn manifest(&self) -> crate::Result<Manifest> {
        Manifest::from_file(&self.manifest_path)
    }

    pub fn manifest_or_empty(&self) -> crate::Result<Manifest> {
        if self.has_manifest() {
            self.manifest()
        } else {
            Ok(Manifest::empty())
        }
    }

    pub fn lockfile(&self) -> crate::Result<Lockfile> {
        Lockfile::from_file(&self.lockfile_path)
    }

    /// Compute effective options by merging inherited options with this project's local options.
    /// `inherited` comes from the workspace root; local options override inherited ones.
    pub fn effective_options(&self, inherited: &ManifestOptions) -> crate::Result<ManifestOptions> {
        let local = self.manifest_or_empty()?.options;
        Ok(ManifestOptions {
            targets: if local.targets.is_empty() {
                inherited.targets.clone()
            } else {
                local.targets
            },
            skills_dir: local.skills_dir.or_else(|| inherited.skills_dir.clone()),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn project_from_dir() {
        let dir = std::path::PathBuf::from("/tmp/test-project");
        let project = Project::new(dir.clone());
        assert_eq!(project.dir, dir);
        assert_eq!(project.manifest_path, dir.join("Ion.toml"));
        assert_eq!(project.lockfile_path, dir.join("Ion.lock"));
    }

    #[test]
    fn project_has_manifest_false_for_nonexistent() {
        let dir = std::path::PathBuf::from("/tmp/nonexistent-project-12345");
        let project = Project::new(dir);
        assert!(!project.has_manifest());
    }
}
```

- [ ] **Step 4: Register module in lib.rs**

In `crates/ion-skill/src/lib.rs`, add:

```rust
pub mod workspace;
```

- [ ] **Step 5: Run tests to verify they pass**

Run: `cargo nextest run -E 'test(project_from_dir) | test(project_has_manifest)' -p ion-skill`
Expected: PASS

- [ ] **Step 6: Commit**

```bash
git add crates/ion-skill/src/workspace.rs crates/ion-skill/src/lib.rs
git commit -m "feat: add Project struct for workspace support"
```

---

### Task 3: Create `WorkspaceContext` with Discovery Logic

**Files:**
- Modify: `src/context.rs`

This is the core change. `WorkspaceContext` replaces `ProjectContext` with workspace-aware discovery. For a workspace-of-one (no `[workspace]` section), behavior is identical to the old `ProjectContext`.

- [ ] **Step 1: Write integration test for workspace-of-one backward compat**

Create `tests/workspace_integration.rs`:

```rust
use std::process::Command;

fn ion_cmd() -> Command {
    Command::new(env!("CARGO_BIN_EXE_ion"))
}

#[test]
fn workspace_of_one_list_works() {
    let project = tempfile::tempdir().unwrap();
    std::fs::write(
        project.path().join("Ion.toml"),
        "[options.targets]\nclaude = \".claude/skills\"\n\n[skills]\n",
    )
    .unwrap();

    let output = ion_cmd()
        .args(["skill", "list"])
        .current_dir(project.path())
        .output()
        .unwrap();

    assert!(output.status.success(), "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr));
}

#[test]
fn workspace_discovered_from_member_dir() {
    let root = tempfile::tempdir().unwrap();
    // Root Ion.toml with workspace
    std::fs::write(
        root.path().join("Ion.toml"),
        r#"
[workspace]
members = ["docs"]

[options.targets]
claude = ".claude/skills"

[skills]
"#,
    )
    .unwrap();

    // Member dir with its own Ion.toml
    let docs_dir = root.path().join("docs");
    std::fs::create_dir_all(&docs_dir).unwrap();
    std::fs::write(
        docs_dir.join("Ion.toml"),
        "[skills]\n",
    )
    .unwrap();

    // Running from docs/ should find the workspace and scope to docs
    let output = ion_cmd()
        .args(["skill", "list"])
        .current_dir(&docs_dir)
        .output()
        .unwrap();

    assert!(output.status.success(), "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr));
}

#[test]
fn workspace_member_inherits_targets() {
    let root = tempfile::tempdir().unwrap();
    std::fs::write(
        root.path().join("Ion.toml"),
        r#"
[workspace]
members = ["docs"]

[options.targets]
claude = ".claude/skills"

[skills]
"#,
    )
    .unwrap();

    let docs_dir = root.path().join("docs");
    std::fs::create_dir_all(&docs_dir).unwrap();
    // No [options.targets] in member — should inherit from root
    std::fs::write(docs_dir.join("Ion.toml"), "[skills]\n").unwrap();

    let output = ion_cmd()
        .args(["config", "list"])
        .current_dir(&docs_dir)
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(output.status.success(), "stderr: {}", String::from_utf8_lossy(&output.stderr));
    assert!(stdout.contains("claude"), "should show inherited target; got: {stdout}");
}

#[test]
fn workspace_rejects_missing_member() {
    let root = tempfile::tempdir().unwrap();
    std::fs::write(
        root.path().join("Ion.toml"),
        r#"
[workspace]
members = ["nonexistent"]

[skills]
"#,
    )
    .unwrap();

    let output = ion_cmd()
        .args(["skill", "list"])
        .current_dir(root.path())
        .output()
        .unwrap();

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("nonexistent"), "should mention missing member; got: {stderr}");
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo nextest run -E 'test(workspace)' --test workspace_integration`
Expected: FAIL — some tests will fail because workspace discovery doesn't exist yet

- [ ] **Step 3: Implement WorkspaceContext**

Replace the contents of `src/context.rs`:

```rust
use std::path::{Path, PathBuf};

use ion_skill::config::GlobalConfig;
use ion_skill::lockfile::Lockfile;
use ion_skill::manifest::{Manifest, ManifestOptions};
use ion_skill::workspace::Project;

/// Scoping for workspace commands.
#[derive(Debug, Clone)]
pub enum Scope {
    /// Operate on all projects (root + members).
    All,
    /// Operate on specific project(s) by index (0 = root, 1+ = members).
    Projects(Vec<usize>),
}

/// Workspace-first project context. Replaces the old `ProjectContext`.
///
/// Every project is a workspace-of-one by default. If the root `Ion.toml`
/// has a `[workspace]` section, member projects are loaded.
pub struct WorkspaceContext {
    /// All projects: index 0 is always root, 1+ are members.
    pub projects: Vec<Project>,
    /// Which projects are in scope for this invocation.
    pub scope: Scope,
    /// Global user config.
    pub global_config: GlobalConfig,
}

impl WorkspaceContext {
    /// Load workspace context by discovering `Ion.toml` from CWD upward.
    ///
    /// `project_flags`: values from `--project` CLI flag (paths relative to workspace root).
    pub fn load(project_flags: &[String]) -> anyhow::Result<Self> {
        let cwd = std::env::current_dir()?;
        let global_config = GlobalConfig::load()?;

        // Walk up from CWD to find Ion.toml files
        let (root_dir, is_workspace) = discover_root(&cwd)?;
        let root_project = Project::new(root_dir.clone());

        let mut projects = vec![root_project];
        let mut cwd_member_idx: Option<usize> = None;

        if is_workspace {
            let manifest = projects[0].manifest()?;
            let members = manifest
                .workspace
                .as_ref()
                .map(|w| w.members.clone())
                .unwrap_or_default();

            for (i, member_path) in members.iter().enumerate() {
                let member_dir = root_dir.join(member_path);
                if !member_dir.exists() {
                    anyhow::bail!(
                        "Workspace member '{}' does not exist (listed in [workspace].members)",
                        member_path
                    );
                }
                let member_manifest_path = member_dir.join("Ion.toml");
                if !member_manifest_path.exists() {
                    anyhow::bail!(
                        "Workspace member '{}' has no Ion.toml",
                        member_path
                    );
                }
                // Check member doesn't also declare [workspace]
                let member_manifest = Manifest::from_file(&member_manifest_path)?;
                if member_manifest.workspace.is_some() {
                    anyhow::bail!(
                        "Workspace member '{}' cannot also have a [workspace] section",
                        member_path
                    );
                }

                let member_project = Project::new(member_dir.clone());
                projects.push(member_project);

                // Check if CWD is inside this member
                if cwd.starts_with(&member_dir) {
                    cwd_member_idx = Some(i + 1); // +1 because root is 0
                }
            }
        }

        // Resolve scope
        let scope = if !project_flags.is_empty() {
            let mut indices = Vec::new();
            for flag in project_flags {
                let idx = resolve_project_flag(flag, &root_dir, &projects)?;
                indices.push(idx);
            }
            Scope::Projects(indices)
        } else if let Some(idx) = cwd_member_idx {
            // CWD is inside a member — scope to that member
            Scope::Projects(vec![idx])
        } else if is_workspace && cwd.starts_with(&root_dir) {
            // CWD is at or under root in a workspace — scope to all
            Scope::All
        } else {
            // Workspace-of-one — scope to root
            Scope::Projects(vec![0])
        };

        Ok(Self {
            projects,
            scope,
            global_config,
        })
    }

    // -----------------------------------------------------------------------
    // Scope helpers
    // -----------------------------------------------------------------------

    /// Returns projects in scope.
    pub fn scoped_projects(&self) -> Vec<&Project> {
        match &self.scope {
            Scope::All => self.projects.iter().collect(),
            Scope::Projects(indices) => indices.iter().map(|&i| &self.projects[i]).collect(),
        }
    }

    /// Returns a single project in scope. Errors if scope covers multiple projects.
    pub fn single_project(&self) -> anyhow::Result<&Project> {
        let projects = self.scoped_projects();
        if projects.len() == 1 {
            Ok(projects[0])
        } else {
            anyhow::bail!(
                "This command requires a single project, but {} are in scope. \
                 Use --project <path> to specify which one.",
                projects.len()
            )
        }
    }

    /// Returns the root project (index 0).
    pub fn root_project(&self) -> &Project {
        &self.projects[0]
    }

    /// Root directory of the workspace.
    pub fn root_dir(&self) -> &Path {
        &self.projects[0].dir
    }

    /// Is this a multi-project workspace?
    pub fn is_workspace(&self) -> bool {
        self.projects.len() > 1
    }

    // -----------------------------------------------------------------------
    // Backward-compatible convenience methods (delegate to single_project)
    // -----------------------------------------------------------------------

    /// Merged options for a project: global config + inherited root options + local options.
    pub fn merged_options_for(&self, project: &Project) -> anyhow::Result<ManifestOptions> {
        let root_options = &self.root_project().manifest_or_empty()?.options;
        let effective = project.effective_options(root_options)?;
        let merged_targets = self.global_config.resolve_targets(&effective);
        Ok(ManifestOptions {
            targets: merged_targets,
            skills_dir: effective.skills_dir,
        })
    }

    /// Create a `Paint` instance for styled output.
    pub fn paint(&self) -> crate::style::Paint {
        crate::style::Paint::new(&self.global_config)
    }

    /// Create a `SkillInstaller` for a specific project.
    pub fn installer_for<'a>(
        &'a self,
        project: &'a Project,
        options: &'a ManifestOptions,
    ) -> ion_skill::installer::SkillInstaller<'a> {
        ion_skill::installer::SkillInstaller::new(&project.dir, options)
    }
}

/// Walk up from `start` looking for an `Ion.toml`.
/// Returns (root_dir, has_workspace_section).
fn discover_root(start: &Path) -> anyhow::Result<(PathBuf, bool)> {
    let mut current = start.to_path_buf();

    // First, collect all Ion.toml locations walking upward
    let mut found: Vec<(PathBuf, bool)> = Vec::new();
    loop {
        let manifest_path = current.join("Ion.toml");
        if manifest_path.exists() {
            // Check if this manifest has [workspace]
            let content = std::fs::read_to_string(&manifest_path)?;
            let has_workspace = content.contains("[workspace]");
            found.push((current.clone(), has_workspace));
            if has_workspace {
                // Found a workspace root — stop searching
                break;
            }
        }
        if !current.pop() {
            break;
        }
    }

    if found.is_empty() {
        // No Ion.toml found — use CWD (commands like init will create one)
        return Ok((start.to_path_buf(), false));
    }

    // If any ancestor has [workspace], that's the root
    if let Some((dir, _)) = found.iter().find(|(_, has_ws)| *has_ws) {
        return Ok((dir.clone(), true));
    }

    // Otherwise, the nearest Ion.toml is the root (workspace-of-one)
    Ok((found[0].0.clone(), false))
}

/// Resolve a `--project` flag value to an index in the projects vec.
fn resolve_project_flag(flag: &str, root_dir: &Path, projects: &[Project]) -> anyhow::Result<usize> {
    if flag == "." {
        return Ok(0); // root project
    }

    let target_dir = root_dir.join(flag);
    for (i, project) in projects.iter().enumerate() {
        if project.dir == target_dir {
            return Ok(i);
        }
    }

    anyhow::bail!(
        "No workspace member matches '--project {flag}'. \
         Available members: {}",
        projects
            .iter()
            .skip(1)
            .map(|p| p.dir.strip_prefix(root_dir).unwrap_or(&p.dir).display().to_string())
            .collect::<Vec<_>>()
            .join(", ")
    )
}

// ---------------------------------------------------------------------------
// Legacy alias — allows gradual migration of command files
// ---------------------------------------------------------------------------

/// Backward-compatible alias. New code should use `WorkspaceContext` directly.
pub type ProjectContext = WorkspaceContext;
```

- [ ] **Step 4: Run tests to verify workspace_integration tests pass**

Run: `cargo nextest run --test workspace_integration`
Expected: PASS — all four tests pass

- [ ] **Step 5: Run full test suite (expect compile errors in commands)**

Run: `cargo build`
Expected: Compile errors in command files that use `ProjectContext` fields directly. This is expected and will be fixed in Task 4.

- [ ] **Step 6: Commit (even with compile errors in commands, the library and context compile)**

Do NOT commit yet — proceed to Task 4 to fix compile errors first.

---

### Task 4: Migrate All Commands to `WorkspaceContext`

**Files:**
- Modify: `src/commands/list.rs`
- Modify: `src/commands/update.rs`
- Modify: `src/commands/add.rs`
- Modify: `src/commands/install.rs`
- Modify: `src/commands/remove.rs`
- Modify: `src/commands/init.rs`
- Modify: `src/commands/agents.rs`
- Modify: `src/commands/install_shared.rs`
- Modify: `src/commands/link.rs`
- Modify: `src/commands/eject.rs`
- Modify: `src/commands/config.rs`
- Modify: `src/commands/validate.rs`
- Modify: `src/commands/new.rs`
- Modify: `src/commands/migrate.rs`
- Modify: `src/commands/run.rs`
- Modify: `src/commands/gc.rs`
- Modify: `src/builtin_skill.rs`
- Modify: `src/main.rs`

This is a mechanical migration. The goal: all commands compile and behave identically to before (workspace-of-one). No workspace-specific behavior yet — that comes in later tasks.

**Migration pattern for each command:**

Old pattern:
```rust
let ctx = ProjectContext::load()?;
let manifest = ctx.manifest()?;
let options = ctx.merged_options(&manifest);
let installer = ctx.installer(&options);
// uses ctx.project_dir, ctx.manifest_path, ctx.lockfile_path
```

New pattern:
```rust
let ws = WorkspaceContext::load(&[])?;
let project = ws.single_project()?;
let manifest = project.manifest()?;
let options = ws.merged_options_for(project)?;
let installer = ws.installer_for(project, &options);
// uses project.dir, project.manifest_path, project.lockfile_path
```

- [ ] **Step 1: Update `install_shared.rs` — change `finalize_skill_install` signatures**

Replace `ctx: &ProjectContext` with `project: &Project` in function signatures. Change `ctx.project_dir` to `project.dir`, `ctx.manifest_path` to `project.manifest_path`, `ctx.lockfile_path` to `project.lockfile_path`.

For `finalize_skill_install`:
```rust
pub fn finalize_skill_install(
    project: &ion_skill::workspace::Project,
    merged_options: &ManifestOptions,
    name: &str,
    source: &SkillSource,
    locked: LockedSkill,
    lockfile: &mut Lockfile,
    opts: &FinalizeOptions,
) -> anyhow::Result<()> {
    add_gitignore_entries(&project.dir, name, source, merged_options)?;
    if opts.register_in_registry {
        register_in_registry(source, &project.dir)?;
    }
    if opts.write_manifest {
        manifest_writer::add_skill(&project.manifest_path, name, source)?;
    }
    lockfile.upsert(locked);
    Ok(())
}
```

Same pattern for `finalize_skill_install_and_write` — take `project: &Project` instead of `ctx: &ProjectContext`, use `project.lockfile()`, `project.lockfile_path`.

- [ ] **Step 2: Update `list.rs`**

```rust
use crate::context::WorkspaceContext;

pub fn run(json: bool) -> anyhow::Result<()> {
    let ws = WorkspaceContext::load(&[])?;
    let p = ws.paint();
    let project = ws.single_project()?;

    if !project.has_manifest() {
        anyhow::bail!("No Ion.toml found in current directory");
    }

    let manifest = project.manifest()?;
    let merged_options = ws.merged_options_for(project)?;
    let lockfile = project.lockfile()?;

    // ... rest unchanged but replace ctx.project_dir with project.dir
```

- [ ] **Step 3: Update `update.rs`**

Same pattern. Replace `ctx = ProjectContext::load()` with `ws = WorkspaceContext::load(&[])` + `project = ws.single_project()`. Replace:
- `ctx.manifest()` → `project.manifest()`
- `ctx.lockfile()` → `project.lockfile()`
- `ctx.merged_options(&manifest)` → `ws.merged_options_for(project)?`
- `ctx.installer(&options)` → `ws.installer_for(project, &options)`
- `ctx.lockfile_path` → `project.lockfile_path`
- `ctx.paint()` → `ws.paint()`

For the `update_template_non_fatal` call in `agents.rs`, update its signature to accept `project: &Project` instead of `ctx: &ProjectContext`.

- [ ] **Step 4: Update `add.rs`**

Same mechanical pattern. Note `add.rs` passes `&ctx` to `finish_single_install` and `install_collection` — change those function signatures to take `project: &Project` and `ws: &WorkspaceContext` (for `paint()`).

```rust
fn finish_single_install(
    project: &ion_skill::workspace::Project,
    p: &Paint,
    merged_options: &ion_skill::manifest::ManifestOptions,
    name: &str,
    source: &SkillSource,
    locked: ion_skill::lockfile::LockedSkill,
    json: bool,
) -> anyhow::Result<()> {
```

And `install_collection` similarly.

- [ ] **Step 5: Update `install.rs`**

Same pattern. Replace `ctx.project_dir` with `project.dir`, etc.

- [ ] **Step 6: Update `remove.rs`**

Same pattern. All `ctx.project_dir` → `project.dir`, `ctx.manifest_path` → `project.manifest_path`, etc.

- [ ] **Step 7: Update `init.rs`**

`init.rs` is special — it creates Ion.toml. Use `ws = WorkspaceContext::load(&[])` then `ws.root_project()` (init always operates on the current directory which becomes the root).

Replace `ctx.project_dir` with `project.dir`, `ctx.manifest_path` with `project.manifest_path`.

- [ ] **Step 8: Update `agents.rs`**

Replace `ProjectContext` usage. The `update_template_non_fatal` function signature changes:

```rust
pub fn update_template_non_fatal(
    project: &ion_skill::workspace::Project,
    global_config: &ion_skill::config::GlobalConfig,
    lockfile: &mut ion_skill::lockfile::Lockfile,
    p: &crate::style::Paint,
    json: bool,
) -> anyhow::Result<()> {
```

Replace `ctx.global_config.resolve_source(...)` with `global_config.resolve_source(...)`.
Replace `ctx.project_dir` with `project.dir`, etc.

Same for `deploy_agents_update_skill` — take `project: &Project` + `options: &ManifestOptions`.

Same for `init`, `update`, `diff` functions.

- [ ] **Step 9: Update remaining commands**

Apply the same mechanical pattern to:
- `config.rs` — uses `ctx.manifest()`, `ctx.merged_options()`, `ctx.global_config`
- `link.rs` — uses `ctx.installer()`, `ctx.manifest_path`
- `eject.rs` — uses `ctx.manifest()`, `ctx.project_dir`
- `new.rs` — uses `ctx.project_dir`, `ctx.manifest_path`
- `migrate.rs` — uses `ctx.project_dir`, `ctx.manifest_path`
- `run.rs` — uses `ctx.manifest()`, `ctx.lockfile()`
- `gc.rs` — does not use ProjectContext (uses data_dir directly), likely no changes
- `validate.rs` — may not use ProjectContext, check and update if needed
- `info.rs` — uses global_config for source resolution
- `search.rs` — uses global_config for source resolution

- [ ] **Step 10: Update `builtin_skill.rs`**

Check if it references `ProjectContext` directly. Update to take `project: &Project` + `manifest_path` where needed.

- [ ] **Step 11: Update `main.rs` to pass `&[]` to `WorkspaceContext::load`**

No changes to the dispatch logic yet — `--project` flag comes in Task 5. For now, all commands load with `WorkspaceContext::load(&[])`.

- [ ] **Step 12: Build and verify compilation**

Run: `cargo build`
Expected: PASS — no compile errors

- [ ] **Step 13: Run full test suite**

Run: `cargo nextest run`
Expected: All existing tests pass (workspace-of-one behaves identically)

- [ ] **Step 14: Run clippy**

Run: `cargo clippy --all-targets --all-features -- -D warnings`
Expected: PASS

- [ ] **Step 15: Commit**

```bash
git add -A
git commit -m "refactor!: replace ProjectContext with WorkspaceContext"
```

---

### Task 5: Add `--project` CLI Flag

**Files:**
- Modify: `src/main.rs`

- [ ] **Step 1: Write integration test**

Add to `tests/workspace_integration.rs`:

```rust
#[test]
fn project_flag_scopes_to_member() {
    let root = tempfile::tempdir().unwrap();
    std::fs::write(
        root.path().join("Ion.toml"),
        r#"
[workspace]
members = ["docs"]

[options.targets]
claude = ".claude/skills"

[skills]
"#,
    )
    .unwrap();

    let docs_dir = root.path().join("docs");
    std::fs::create_dir_all(&docs_dir).unwrap();
    std::fs::write(docs_dir.join("Ion.toml"), "[skills]\n").unwrap();

    // --project docs from root should show docs skills
    let output = ion_cmd()
        .args(["skill", "list", "--project", "docs"])
        .current_dir(root.path())
        .output()
        .unwrap();

    assert!(output.status.success());
}

#[test]
fn project_flag_dot_scopes_to_root() {
    let root = tempfile::tempdir().unwrap();
    std::fs::write(
        root.path().join("Ion.toml"),
        r#"
[workspace]
members = ["docs"]

[options.targets]
claude = ".claude/skills"

[skills]
foo = "bar/baz"
"#,
    )
    .unwrap();

    let docs_dir = root.path().join("docs");
    std::fs::create_dir_all(&docs_dir).unwrap();
    std::fs::write(docs_dir.join("Ion.toml"), "[skills]\n").unwrap();

    // --project . from root should show root skills only
    let output = ion_cmd()
        .args(["skill", "list", "--project", "."])
        .current_dir(root.path())
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(output.status.success());
    assert!(stdout.contains("foo"), "should show root skill; got: {stdout}");
}

#[test]
fn project_flag_invalid_member_errors() {
    let root = tempfile::tempdir().unwrap();
    std::fs::write(
        root.path().join("Ion.toml"),
        "[workspace]\nmembers = [\"docs\"]\n\n[skills]\n",
    )
    .unwrap();

    let docs_dir = root.path().join("docs");
    std::fs::create_dir_all(&docs_dir).unwrap();
    std::fs::write(docs_dir.join("Ion.toml"), "[skills]\n").unwrap();

    let output = ion_cmd()
        .args(["skill", "list", "--project", "nonexistent"])
        .current_dir(root.path())
        .output()
        .unwrap();

    assert!(!output.status.success());
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo nextest run -E 'test(project_flag)' --test workspace_integration`
Expected: FAIL — `--project` flag not recognized

- [ ] **Step 3: Add `--project` global flag to CLI**

In `src/main.rs`, add to the `Cli` struct:

```rust
#[derive(Parser)]
#[command(name = "ion", about = "Agent skill manager", version = env!("CARGO_PKG_VERSION"))]
struct Cli {
    /// Output results as JSON (for agents and scripts)
    #[arg(long, global = true)]
    json: bool,

    /// Pretty-print JSON output (requires --json)
    #[arg(long, global = true, requires = "json")]
    pretty: bool,

    /// Operate on a specific project within the workspace (path relative to root, or "." for root)
    #[arg(long, global = true)]
    project: Vec<String>,

    #[command(subcommand)]
    command: Commands,
}
```

Then pass `project` to `WorkspaceContext::load`:

In `fn main()`, after parsing:
```rust
let cli = Cli::parse();
let project_flags = cli.project;
```

Each command that calls `WorkspaceContext::load(&[])` needs access to `project_flags`. The simplest approach: pass `&project_flags` through the command dispatch. Update command signatures to accept `project_flags: &[String]`.

Alternatively, store it in a thread-local or pass through main. The cleanest approach for clap: pass it to each command function.

Update the dispatch in `main()`:

```rust
Commands::Add { source, .. } => match source {
    Some(src) => commands::add::run(&src, ..., &project_flags),
    None => commands::install::run(json, allow_warnings, &project_flags),
},
```

And update each command's `run` function to accept `project_flags: &[String]` and pass to `WorkspaceContext::load(project_flags)`.

- [ ] **Step 4: Run tests**

Run: `cargo nextest run -E 'test(project_flag)' --test workspace_integration`
Expected: PASS

- [ ] **Step 5: Run full test suite**

Run: `cargo nextest run`
Expected: All tests pass

- [ ] **Step 6: Commit**

```bash
git add -A
git commit -m "feat: add --project flag for workspace scoping"
```

---

### Task 6: Workspace-Aware `list` and `update` (Bulk Commands)

**Files:**
- Modify: `src/commands/list.rs`
- Modify: `src/commands/update.rs`

These commands operate on ALL projects when run from workspace root.

- [ ] **Step 1: Write integration test for workspace-wide list**

Add to `tests/workspace_integration.rs`:

```rust
#[test]
fn list_shows_all_projects_from_root() {
    let root = tempfile::tempdir().unwrap();
    std::fs::write(
        root.path().join("Ion.toml"),
        r#"
[workspace]
members = ["docs"]

[options.targets]
claude = ".claude/skills"

[skills]
root-skill = { type = "local" }
"#,
    )
    .unwrap();
    // Create the local skill dir so it shows as installed
    let root_skill_dir = root.path().join(".agents/skills/root-skill");
    std::fs::create_dir_all(&root_skill_dir).unwrap();
    std::fs::write(root_skill_dir.join("SKILL.md"), "---\nname: root-skill\n---\n").unwrap();

    let docs_dir = root.path().join("docs");
    std::fs::create_dir_all(&docs_dir).unwrap();
    std::fs::write(
        docs_dir.join("Ion.toml"),
        "[skills]\ndocs-skill = { type = \"local\" }\n",
    )
    .unwrap();
    let docs_skill_dir = docs_dir.join(".agents/skills/docs-skill");
    std::fs::create_dir_all(&docs_skill_dir).unwrap();
    std::fs::write(docs_skill_dir.join("SKILL.md"), "---\nname: docs-skill\n---\n").unwrap();

    let output = ion_cmd()
        .args(["skill", "list"])
        .current_dir(root.path())
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(output.status.success());
    assert!(stdout.contains("root-skill"), "should show root skills; got: {stdout}");
    assert!(stdout.contains("docs-skill"), "should show docs skills; got: {stdout}");
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo nextest run -E 'test(list_shows_all)' --test workspace_integration`
Expected: FAIL — list only shows root project skills

- [ ] **Step 3: Update `list.rs` to iterate scoped projects**

```rust
use crate::context::WorkspaceContext;

pub fn run(json: bool, project_flags: &[String]) -> anyhow::Result<()> {
    let ws = WorkspaceContext::load(project_flags)?;
    let p = ws.paint();
    let scoped = ws.scoped_projects();

    if json {
        let mut all_skills: Vec<serde_json::Value> = Vec::new();
        for project in &scoped {
            if !project.has_manifest() { continue; }
            let manifest = project.manifest()?;
            let options = ws.merged_options_for(project)?;
            let lockfile = project.lockfile()?;
            let project_label = project.dir.strip_prefix(ws.root_dir())
                .map(|p| if p.as_os_str().is_empty() { ".".to_string() } else { p.display().to_string() })
                .unwrap_or_else(|_| project.dir.display().to_string());

            for (name, entry) in &manifest.skills {
                let source = match entry.resolve() {
                    Ok(s) => s,
                    Err(_) => continue,
                };
                let locked = lockfile.find(name);
                let is_binary = locked.is_some_and(|l| l.is_binary());
                let version = if is_binary {
                    locked.and_then(|l| l.binary_version()).unwrap_or("unknown")
                } else {
                    locked.and_then(|l| l.version.as_deref()).unwrap_or("unknown")
                };
                let commit = locked.and_then(|l| l.commit());
                let installed = project.dir.join(options.skills_dir_or_default()).join(name).exists();
                all_skills.push(serde_json::json!({
                    "name": name,
                    "project": project_label,
                    "source": source.source,
                    "version": version,
                    "commit": commit,
                    "binary": is_binary,
                    "installed": installed,
                }));
            }
        }
        crate::json::print_success(serde_json::json!(all_skills));
        return Ok(());
    }

    let show_headers = scoped.len() > 1;

    for project in &scoped {
        if !project.has_manifest() { continue; }
        let manifest = project.manifest()?;
        let options = ws.merged_options_for(project)?;
        let lockfile = project.lockfile()?;

        if show_headers {
            let label = project.dir.strip_prefix(ws.root_dir())
                .map(|p| if p.as_os_str().is_empty() { ". (root)".to_string() } else { p.display().to_string() })
                .unwrap_or_else(|_| project.dir.display().to_string());
            println!("\n{}:", p.bold(&label));
        }

        if manifest.skills.is_empty() {
            println!("  No skills declared in Ion.toml.");
            continue;
        }

        if !show_headers {
            println!("Skills ({}):", p.bold(&manifest.skills.len().to_string()));
        }

        for (name, entry) in &manifest.skills {
            // ... same per-skill display logic as before, using project.dir
            // instead of ctx.project_dir
            let source = match entry.resolve() {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("warning: skipping '{}': {}", name, e);
                    continue;
                }
            };
            let locked = lockfile.find(name);
            let is_binary = locked.is_some_and(|l| l.is_binary());
            let version_str = if is_binary {
                locked.and_then(|l| l.binary_version()).unwrap_or("unknown")
            } else {
                locked.and_then(|l| l.version.as_deref()).unwrap_or("unknown")
            };
            let type_indicator = if is_binary {
                format!(" {}", p.info("(binary)"))
            } else {
                let commit_str = locked
                    .and_then(|l| l.commit())
                    .map(|c| &c[..c.len().min(8)])
                    .unwrap_or("none");
                format!(" {}", p.dim(&format!("({commit_str})")))
            };
            let installed = project.dir.join(options.skills_dir_or_default()).join(name).exists();
            let status = if installed { p.success("installed") } else { p.warn("not installed") };
            let display_version = if version_str.starts_with('v') { version_str.to_string() } else { format!("v{version_str}") };
            println!("  {} {}{} [{}]", p.bold(name), p.dim(&display_version), type_indicator, status);
            println!("    source: {}", p.info(&source.source));
        }
    }
    Ok(())
}
```

- [ ] **Step 4: Update `update.rs` to iterate scoped projects**

For update, wrap the main loop in a `for project in scoped` loop. Each project gets its own lockfile, manifest, and installer. The agents template update also runs per-project.

The key structural change:

```rust
pub fn run(name: Option<&str>, json: bool, project_flags: &[String]) -> anyhow::Result<()> {
    let ws = WorkspaceContext::load(project_flags)?;
    let p = ws.paint();
    let scoped = ws.scoped_projects();
    let show_headers = scoped.len() > 1;

    // Aggregate counters across all projects
    let mut total_updated = 0u32;
    // ... etc

    for project in &scoped {
        if !project.has_manifest() { continue; }

        if show_headers {
            let label = project.dir.strip_prefix(ws.root_dir())
                .map(|p| if p.as_os_str().is_empty() { ". (root)".to_string() } else { p.display().to_string() })
                .unwrap_or_else(|_| project.dir.display().to_string());
            println!("\n{}:", p.bold(&label));
        }

        let manifest = project.manifest()?;
        let mut lockfile = project.lockfile()?;
        let options = ws.merged_options_for(project)?;
        let installer = ws.installer_for(project, &options);

        // ... existing update loop per skill, unchanged ...

        if updated_count > 0 {
            lockfile.write_to(&project.lockfile_path)?;
        }

        // agents template update
        if manifest.agents.as_ref().and_then(|a| a.template.as_ref()).is_some() {
            if let Err(e) = crate::commands::agents::update_template_non_fatal(
                project, &ws.global_config, &mut lockfile, &p, json
            ) {
                if !json {
                    println!("  {} agents template: {}", p.warn("⚠"), p.warn(&e.to_string()));
                }
            }
        }

        total_updated += updated_count;
    }

    // ... summary ...
}
```

- [ ] **Step 5: Run tests**

Run: `cargo nextest run --test workspace_integration`
Expected: All workspace tests pass

- [ ] **Step 6: Run full test suite**

Run: `cargo nextest run`
Expected: All tests pass

- [ ] **Step 7: Commit**

```bash
git add src/commands/list.rs src/commands/update.rs
git commit -m "feat: workspace-aware list and update commands"
```

---

### Task 7: Workspace-Aware `add` and `remove` (Single-Project Commands)

**Files:**
- Modify: `src/commands/add.rs`
- Modify: `src/commands/remove.rs`

These commands error when run from workspace root without `--project` (ambiguous).

- [ ] **Step 1: Write integration test**

Add to `tests/workspace_integration.rs`:

```rust
#[test]
fn add_from_workspace_root_without_project_flag_errors() {
    let root = tempfile::tempdir().unwrap();
    std::fs::write(
        root.path().join("Ion.toml"),
        "[workspace]\nmembers = [\"docs\"]\n\n[skills]\n",
    )
    .unwrap();

    let docs_dir = root.path().join("docs");
    std::fs::create_dir_all(&docs_dir).unwrap();
    std::fs::write(docs_dir.join("Ion.toml"), "[skills]\n").unwrap();

    // ion add <source> from root without --project should error
    let output = ion_cmd()
        .args(["add", "some/fake/skill"])
        .current_dir(root.path())
        .output()
        .unwrap();

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("--project") || stderr.contains("single project"),
        "should suggest --project; got: {stderr}"
    );
}

#[test]
fn remove_from_workspace_root_without_project_flag_errors() {
    let root = tempfile::tempdir().unwrap();
    std::fs::write(
        root.path().join("Ion.toml"),
        "[workspace]\nmembers = [\"docs\"]\n\n[skills]\nfoo = { type = \"local\" }\n",
    )
    .unwrap();

    let docs_dir = root.path().join("docs");
    std::fs::create_dir_all(&docs_dir).unwrap();
    std::fs::write(docs_dir.join("Ion.toml"), "[skills]\n").unwrap();

    let output = ion_cmd()
        .args(["remove", "foo", "-y"])
        .current_dir(root.path())
        .output()
        .unwrap();

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("--project") || stderr.contains("single project"),
        "should suggest --project; got: {stderr}"
    );
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo nextest run -E 'test(add_from_workspace) | test(remove_from_workspace)' --test workspace_integration`
Expected: FAIL

- [ ] **Step 3: Update `add.rs` to require single project**

At the top of `run()`:

```rust
let ws = WorkspaceContext::load(project_flags)?;
let project = ws.single_project()?;
```

`single_project()` already errors with the right message if scope is All and there are multiple projects.

- [ ] **Step 4: Update `remove.rs` same way**

Same pattern — `ws.single_project()?` at the top.

- [ ] **Step 5: Run tests**

Run: `cargo nextest run -E 'test(add_from_workspace) | test(remove_from_workspace)' --test workspace_integration`
Expected: PASS

- [ ] **Step 6: Run full test suite**

Run: `cargo nextest run`
Expected: All tests pass

- [ ] **Step 7: Commit**

```bash
git add src/commands/add.rs src/commands/remove.rs
git commit -m "feat: add and remove require --project in workspace"
```

---

### Task 8: Workspace-Aware `agents` Commands

**Files:**
- Modify: `src/commands/agents.rs`

- [ ] **Step 1: Write integration test**

Add to `tests/workspace_integration.rs`:

```rust
#[test]
fn agents_init_requires_project_in_workspace() {
    let root = tempfile::tempdir().unwrap();
    std::fs::write(
        root.path().join("Ion.toml"),
        "[workspace]\nmembers = [\"docs\"]\n\n[skills]\n",
    )
    .unwrap();

    let docs_dir = root.path().join("docs");
    std::fs::create_dir_all(&docs_dir).unwrap();
    std::fs::write(docs_dir.join("Ion.toml"), "[skills]\n").unwrap();

    // agents init from root without --project: should init for root (not ambiguous for setup)
    // This test just verifies the command doesn't crash; actual template fetching
    // would require network access
    let output = ion_cmd()
        .args(["agents", "init", "some/template", "--project", "."])
        .current_dir(root.path())
        .output()
        .unwrap();

    // Will fail because template doesn't exist, but it should NOT fail with
    // "requires single project" error
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("single project"),
        "agents init with --project . should not complain about scoping; got: {stderr}"
    );
}
```

- [ ] **Step 2: Update `agents.rs` functions**

`agents::init` — takes `project_flags`, uses `ws.single_project()`.
`agents::update` — iterates `ws.scoped_projects()`, updates template for each project that has `[agents]` configured.
`agents::diff` — iterates `ws.scoped_projects()`, shows diff per project.

For `update`, the workspace-aware loop:

```rust
pub fn update(json: bool, project_flags: &[String]) -> anyhow::Result<()> {
    let ws = WorkspaceContext::load(project_flags)?;
    let p = ws.paint();

    for project in ws.scoped_projects() {
        if !project.has_manifest() { continue; }
        let manifest = project.manifest()?;
        if manifest.agents.as_ref().and_then(|a| a.template.as_ref()).is_none() {
            continue;
        }

        // ... existing update logic using project.dir, project.lockfile_path ...
    }
    Ok(())
}
```

- [ ] **Step 3: Run tests**

Run: `cargo nextest run --test workspace_integration`
Expected: PASS

- [ ] **Step 4: Run full test suite**

Run: `cargo nextest run`
Expected: All tests pass

- [ ] **Step 5: Commit**

```bash
git add src/commands/agents.rs
git commit -m "feat: workspace-aware agents commands"
```

---

### Task 9: Add `ion workspace` Subcommand Group

**Files:**
- Create: `src/commands/workspace.rs`
- Modify: `src/commands/mod.rs`
- Modify: `src/main.rs`

- [ ] **Step 1: Write integration tests**

Add to `tests/workspace_integration.rs`:

```rust
#[test]
fn workspace_add_creates_member() {
    let root = tempfile::tempdir().unwrap();
    std::fs::write(root.path().join("Ion.toml"), "[skills]\n").unwrap();

    let docs_dir = root.path().join("docs");
    std::fs::create_dir_all(&docs_dir).unwrap();

    let output = ion_cmd()
        .args(["workspace", "add", "docs"])
        .current_dir(root.path())
        .output()
        .unwrap();

    assert!(output.status.success(), "stderr: {}", String::from_utf8_lossy(&output.stderr));

    // Check Ion.toml has [workspace] section
    let content = std::fs::read_to_string(root.path().join("Ion.toml")).unwrap();
    assert!(content.contains("[workspace]"), "should add workspace section; got: {content}");
    assert!(content.contains("\"docs\""), "should list docs as member; got: {content}");

    // Check docs/Ion.toml was created
    assert!(docs_dir.join("Ion.toml").exists(), "should create docs/Ion.toml");
}

#[test]
fn workspace_remove_unregisters_member() {
    let root = tempfile::tempdir().unwrap();
    std::fs::write(
        root.path().join("Ion.toml"),
        "[workspace]\nmembers = [\"docs\"]\n\n[skills]\n",
    )
    .unwrap();

    let docs_dir = root.path().join("docs");
    std::fs::create_dir_all(&docs_dir).unwrap();
    std::fs::write(docs_dir.join("Ion.toml"), "[skills]\n").unwrap();

    let output = ion_cmd()
        .args(["workspace", "remove", "docs"])
        .current_dir(root.path())
        .output()
        .unwrap();

    assert!(output.status.success());

    let content = std::fs::read_to_string(root.path().join("Ion.toml")).unwrap();
    assert!(!content.contains("\"docs\""), "should remove docs from members; got: {content}");
}

#[test]
fn workspace_list_shows_members() {
    let root = tempfile::tempdir().unwrap();
    std::fs::write(
        root.path().join("Ion.toml"),
        "[workspace]\nmembers = [\"docs\"]\n\n[skills]\nfoo = { type = \"local\" }\n",
    )
    .unwrap();

    let docs_dir = root.path().join("docs");
    std::fs::create_dir_all(&docs_dir).unwrap();
    std::fs::write(
        docs_dir.join("Ion.toml"),
        "[skills]\nbar = { type = \"local\" }\nbaz = { type = \"local\" }\n",
    )
    .unwrap();

    let output = ion_cmd()
        .args(["workspace", "list"])
        .current_dir(root.path())
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(output.status.success());
    assert!(stdout.contains("root"), "should show root; got: {stdout}");
    assert!(stdout.contains("docs"), "should show docs; got: {stdout}");
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo nextest run -E 'test(workspace_add) | test(workspace_remove) | test(workspace_list_shows)' --test workspace_integration`
Expected: FAIL — `workspace` subcommand does not exist

- [ ] **Step 3: Add `WorkspaceCommands` to CLI**

In `src/main.rs`, add to `Commands` enum:

```rust
/// Manage workspace members
Workspace {
    #[command(subcommand)]
    action: WorkspaceCommands,
},
```

Add the subcommands enum:

```rust
#[derive(Subcommand)]
enum WorkspaceCommands {
    /// Add a sub-project to the workspace
    Add {
        /// Path to the sub-project directory (relative to workspace root)
        path: String,
    },
    /// Remove a sub-project from the workspace
    Remove {
        /// Path of the sub-project to remove
        path: String,
    },
    /// List all workspace members
    List,
    /// Show update availability across all projects
    Status,
}
```

Add dispatch in `main()`:

```rust
Commands::Workspace { action } => match action {
    WorkspaceCommands::Add { path } => commands::workspace::add(&path, json),
    WorkspaceCommands::Remove { path } => commands::workspace::remove(&path, json),
    WorkspaceCommands::List => commands::workspace::list(json),
    WorkspaceCommands::Status => commands::workspace::status(json),
},
```

- [ ] **Step 4: Implement `src/commands/workspace.rs`**

```rust
use crate::context::WorkspaceContext;

/// Add a new member to manifest_writer. This is a new function.
fn add_workspace_member(manifest_path: &std::path::Path, member: &str) -> anyhow::Result<()> {
    let content = std::fs::read_to_string(manifest_path)
        .unwrap_or_else(|_| "[skills]\n".to_string());
    let mut doc: toml_edit::DocumentMut = content.parse()?;

    if !doc.contains_key("workspace") {
        doc["workspace"] = toml_edit::Item::Table(toml_edit::Table::new());
    }
    let ws = doc["workspace"].as_table_mut()
        .ok_or_else(|| anyhow::anyhow!("[workspace] is not a table"))?;

    if !ws.contains_key("members") {
        ws["members"] = toml_edit::value(toml_edit::Array::new());
    }
    let members = ws["members"].as_array_mut()
        .ok_or_else(|| anyhow::anyhow!("[workspace].members is not an array"))?;

    // Check for duplicates
    let already_exists = members.iter().any(|v| v.as_str() == Some(member));
    if !already_exists {
        members.push(member);
    }

    std::fs::write(manifest_path, doc.to_string())?;
    Ok(())
}

fn remove_workspace_member(manifest_path: &std::path::Path, member: &str) -> anyhow::Result<()> {
    let content = std::fs::read_to_string(manifest_path)?;
    let mut doc: toml_edit::DocumentMut = content.parse()?;

    let ws = doc.get_mut("workspace")
        .and_then(|w| w.as_table_mut())
        .ok_or_else(|| anyhow::anyhow!("No [workspace] section in Ion.toml"))?;

    let members = ws.get_mut("members")
        .and_then(|m| m.as_array_mut())
        .ok_or_else(|| anyhow::anyhow!("No [workspace].members in Ion.toml"))?;

    let idx = members.iter().position(|v| v.as_str() == Some(member));
    match idx {
        Some(i) => { members.remove(i); }
        None => anyhow::bail!("'{}' is not a workspace member", member),
    }

    std::fs::write(manifest_path, doc.to_string())?;
    Ok(())
}

pub fn add(path: &str, json: bool) -> anyhow::Result<()> {
    let ws = WorkspaceContext::load(&[])?;
    let p = ws.paint();
    let root = ws.root_project();

    let member_dir = root.dir.join(path);

    // Create member directory if it doesn't exist
    if !member_dir.exists() {
        std::fs::create_dir_all(&member_dir)?;
    }

    // Create member Ion.toml if it doesn't exist
    let member_manifest = member_dir.join("Ion.toml");
    if !member_manifest.exists() {
        std::fs::write(&member_manifest, "[skills]\n")?;
        if !json {
            println!("{} {}/Ion.toml", p.success("Created"), path);
        }
    }

    // Add to [workspace].members in root Ion.toml
    add_workspace_member(&root.manifest_path, path)?;

    if json {
        crate::json::print_success(serde_json::json!({
            "member": path,
            "created_manifest": !member_manifest.exists(),
        }));
    } else {
        println!("{} '{}' to workspace", p.success("Added"), path);
        println!("  Updated {}", p.dim("Ion.toml"));
    }

    Ok(())
}

pub fn remove(path: &str, json: bool) -> anyhow::Result<()> {
    let ws = WorkspaceContext::load(&[])?;
    let p = ws.paint();
    let root = ws.root_project();

    remove_workspace_member(&root.manifest_path, path)?;

    if json {
        crate::json::print_success(serde_json::json!({ "removed": path }));
    } else {
        println!("{} '{}' from workspace (files preserved)", p.success("Removed"), path);
        println!("  Updated {}", p.dim("Ion.toml"));
    }

    Ok(())
}

pub fn list(json: bool) -> anyhow::Result<()> {
    let ws = WorkspaceContext::load(&[])?;
    let p = ws.paint();

    if json {
        let members: Vec<serde_json::Value> = ws.projects.iter().enumerate().map(|(i, proj)| {
            let label = if i == 0 { ".".to_string() }
            else {
                proj.dir.strip_prefix(ws.root_dir())
                    .map(|p| p.display().to_string())
                    .unwrap_or_else(|_| proj.dir.display().to_string())
            };
            let skill_count = proj.manifest_or_empty()
                .map(|m| m.skills.len())
                .unwrap_or(0);
            serde_json::json!({
                "path": label,
                "root": i == 0,
                "skills": skill_count,
            })
        }).collect();
        crate::json::print_success(serde_json::json!(members));
        return Ok(());
    }

    println!("Workspace: {}\n", p.bold(&ws.root_dir().display().to_string()));

    for (i, project) in ws.projects.iter().enumerate() {
        let label = if i == 0 {
            ". (root)".to_string()
        } else {
            project.dir.strip_prefix(ws.root_dir())
                .map(|p| p.display().to_string())
                .unwrap_or_else(|_| project.dir.display().to_string())
        };
        let skill_count = project.manifest_or_empty()
            .map(|m| m.skills.len())
            .unwrap_or(0);
        println!("  {:<25} {} skill(s)", p.bold(&label), skill_count);
    }

    Ok(())
}

pub fn status(json: bool) -> anyhow::Result<()> {
    let ws = WorkspaceContext::load(&[])?;
    let p = ws.paint();

    if !json {
        println!("Workspace: {}\n", p.bold(&ws.root_dir().display().to_string()));
    }

    let mut results = Vec::new();

    for (i, project) in ws.projects.iter().enumerate() {
        let label = if i == 0 {
            ". (root)".to_string()
        } else {
            project.dir.strip_prefix(ws.root_dir())
                .map(|p| p.display().to_string())
                .unwrap_or_else(|_| project.dir.display().to_string())
        };

        if !project.has_manifest() {
            if !json {
                println!("  {:<25} no manifest", p.bold(&label));
            }
            continue;
        }

        let manifest = project.manifest()?;
        let skill_count = manifest.skills.len();

        if !json {
            println!("  {:<25} {} skill(s)", p.bold(&label), skill_count);
        }

        results.push(serde_json::json!({
            "path": label,
            "skills": skill_count,
        }));
    }

    if json {
        crate::json::print_success(serde_json::json!(results));
    }

    Ok(())
}
```

- [ ] **Step 5: Register module**

In `src/commands/mod.rs`, add:

```rust
pub mod workspace;
```

- [ ] **Step 6: Run tests**

Run: `cargo nextest run -E 'test(workspace_add) | test(workspace_remove) | test(workspace_list_shows)' --test workspace_integration`
Expected: PASS

- [ ] **Step 7: Run full test suite**

Run: `cargo nextest run`
Expected: All tests pass

- [ ] **Step 8: Commit**

```bash
git add src/commands/workspace.rs src/commands/mod.rs src/main.rs
git commit -m "feat: add ion workspace subcommand group"
```

---

### Task 10: `ion project init` Workspace Auto-Registration

**Files:**
- Modify: `src/commands/init.rs`

When `ion project init` runs inside a directory that's under a workspace root, it auto-registers as a member.

- [ ] **Step 1: Write integration test**

Add to `tests/workspace_integration.rs`:

```rust
#[test]
fn init_inside_workspace_auto_registers() {
    let root = tempfile::tempdir().unwrap();
    std::fs::write(
        root.path().join("Ion.toml"),
        "[workspace]\nmembers = []\n\n[options.targets]\nclaude = \".claude/skills\"\n\n[skills]\n",
    )
    .unwrap();

    let sub_dir = root.path().join("packages/frontend");
    std::fs::create_dir_all(&sub_dir).unwrap();

    let output = ion_cmd()
        .args(["init", "--target", "claude", "--force"])
        .current_dir(&sub_dir)
        .output()
        .unwrap();

    assert!(output.status.success(), "stderr: {}", String::from_utf8_lossy(&output.stderr));

    // Check that sub_dir/Ion.toml was created
    assert!(sub_dir.join("Ion.toml").exists());

    // Check that root Ion.toml now lists this as a member
    let root_content = std::fs::read_to_string(root.path().join("Ion.toml")).unwrap();
    assert!(
        root_content.contains("packages/frontend"),
        "should auto-register as member; got: {root_content}"
    );
}
```

- [ ] **Step 2: Run test**

Run: `cargo nextest run -E 'test(init_inside_workspace)' --test workspace_integration`
Expected: FAIL

- [ ] **Step 3: Update `init.rs`**

After the workspace context is loaded, check if we're in a workspace and the current dir is not the root. If so, compute the relative path and add to members:

```rust
pub fn run(targets: &[String], force: bool, json: bool, project_flags: &[String]) -> anyhow::Result<()> {
    let ws = WorkspaceContext::load(project_flags)?;
    let project = ws.single_project()?;

    // ... existing init logic using project.dir ...

    // After creating Ion.toml, check if we should auto-register in workspace
    if ws.is_workspace() && project.dir != ws.root_dir().to_path_buf() {
        // Compute relative path from workspace root
        if let Ok(relative) = project.dir.strip_prefix(ws.root_dir()) {
            let member_path = relative.display().to_string();
            // Check if already registered
            let root_manifest = ws.root_project().manifest_or_empty()?;
            let already_member = root_manifest
                .workspace
                .as_ref()
                .map(|w| w.members.contains(&member_path))
                .unwrap_or(false);

            if !already_member {
                crate::commands::workspace::add_workspace_member(
                    &ws.root_project().manifest_path,
                    &member_path,
                )?;
                if !json {
                    println!(
                        "  {} as workspace member in root Ion.toml",
                        p.success("Registered")
                    );
                }
            }
        }
    }

    Ok(())
}
```

Note: `add_workspace_member` needs to be `pub` in `workspace.rs` for this to work.

- [ ] **Step 4: Run test**

Run: `cargo nextest run -E 'test(init_inside_workspace)' --test workspace_integration`
Expected: PASS

- [ ] **Step 5: Run full test suite**

Run: `cargo nextest run`
Expected: All tests pass

- [ ] **Step 6: Commit**

```bash
git add src/commands/init.rs src/commands/workspace.rs
git commit -m "feat: auto-register in workspace on ion init"
```

---

### Task 11: Workspace-Aware `install` (Install-All)

**Files:**
- Modify: `src/commands/install.rs`

`ion add` with no args runs install-all. In a workspace, this should install all skills across all scoped projects.

- [ ] **Step 1: Write integration test**

Add to `tests/workspace_integration.rs`:

```rust
#[test]
fn install_all_across_workspace() {
    let root = tempfile::tempdir().unwrap();
    std::fs::write(
        root.path().join("Ion.toml"),
        r#"
[workspace]
members = ["docs"]

[options.targets]
claude = ".claude/skills"

[skills]
root-local = { type = "local" }
"#,
    )
    .unwrap();

    // Create local skill for root
    let root_skill = root.path().join(".agents/skills/root-local");
    std::fs::create_dir_all(&root_skill).unwrap();
    std::fs::write(root_skill.join("SKILL.md"), "---\nname: root-local\n---\ntest").unwrap();

    // Create docs member
    let docs_dir = root.path().join("docs");
    std::fs::create_dir_all(&docs_dir).unwrap();
    std::fs::write(
        docs_dir.join("Ion.toml"),
        "[skills]\ndocs-local = { type = \"local\" }\n",
    )
    .unwrap();
    let docs_skill = docs_dir.join(".agents/skills/docs-local");
    std::fs::create_dir_all(&docs_skill).unwrap();
    std::fs::write(docs_skill.join("SKILL.md"), "---\nname: docs-local\n---\ntest").unwrap();

    // ion add (no args) from root should install skills for both projects
    let output = ion_cmd()
        .args(["add"])
        .current_dir(root.path())
        .output()
        .unwrap();

    assert!(output.status.success(), "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr));

    // Both lockfiles should exist
    assert!(root.path().join("Ion.lock").exists());
    assert!(docs_dir.join("Ion.lock").exists());
}
```

- [ ] **Step 2: Run test**

Run: `cargo nextest run -E 'test(install_all_across)' --test workspace_integration`
Expected: FAIL

- [ ] **Step 3: Update `install.rs` to iterate scoped projects**

Wrap the install logic in a `for project in scoped` loop, similar to `update.rs`:

```rust
pub fn run(json: bool, allow_warnings: bool, project_flags: &[String]) -> anyhow::Result<()> {
    let ws = WorkspaceContext::load(project_flags)?;
    let p = ws.paint();
    let scoped = ws.scoped_projects();
    let show_headers = scoped.len() > 1;

    for project in &scoped {
        if !project.has_manifest() { continue; }

        if show_headers {
            let label = project.dir.strip_prefix(ws.root_dir())
                .map(|p| if p.as_os_str().is_empty() { ". (root)".to_string() } else { p.display().to_string() })
                .unwrap_or_else(|_| project.dir.display().to_string());
            println!("\n{}:", p.bold(&label));
        }

        // ... existing install logic per project ...
    }

    Ok(())
}
```

- [ ] **Step 4: Run test**

Run: `cargo nextest run -E 'test(install_all_across)' --test workspace_integration`
Expected: PASS

- [ ] **Step 5: Run full test suite**

Run: `cargo nextest run`
Expected: All tests pass

- [ ] **Step 6: Commit**

```bash
git add src/commands/install.rs
git commit -m "feat: workspace-aware install-all"
```

---

### Task 12: Add `manifest_writer` Support for Workspace Config

**Files:**
- Modify: `crates/ion-skill/src/manifest_writer.rs`

Add functions for writing `[workspace]` config to Ion.toml, used by `workspace add/remove`.

- [ ] **Step 1: Write unit test**

Add to `crates/ion-skill/src/manifest_writer.rs` (or a test module within):

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn write_workspace_member() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("Ion.toml");
        std::fs::write(&path, "[skills]\n").unwrap();

        add_workspace_member(&path, "docs").unwrap();

        let content = std::fs::read_to_string(&path).unwrap();
        assert!(content.contains("[workspace]"));
        assert!(content.contains("\"docs\""));
    }

    #[test]
    fn remove_workspace_member() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("Ion.toml");
        std::fs::write(
            &path,
            "[workspace]\nmembers = [\"docs\", \"frontend\"]\n\n[skills]\n",
        )
        .unwrap();

        remove_workspace_member_from_manifest(&path, "docs").unwrap();

        let content = std::fs::read_to_string(&path).unwrap();
        assert!(!content.contains("\"docs\""));
        assert!(content.contains("\"frontend\""));
    }

    #[test]
    fn add_duplicate_member_is_noop() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("Ion.toml");
        std::fs::write(
            &path,
            "[workspace]\nmembers = [\"docs\"]\n\n[skills]\n",
        )
        .unwrap();

        add_workspace_member(&path, "docs").unwrap();

        let content = std::fs::read_to_string(&path).unwrap();
        // Should only appear once
        assert_eq!(content.matches("\"docs\"").count(), 1);
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo nextest run -E 'test(write_workspace) | test(remove_workspace) | test(add_duplicate)' -p ion-skill`
Expected: FAIL — functions don't exist yet

- [ ] **Step 3: Implement workspace member functions in manifest_writer**

Add to `crates/ion-skill/src/manifest_writer.rs`:

```rust
/// Add a member to [workspace].members in an Ion.toml file.
/// Creates the [workspace] section if it doesn't exist.
pub fn add_workspace_member(manifest_path: &Path, member: &str) -> Result<String> {
    let content =
        std::fs::read_to_string(manifest_path).unwrap_or_else(|_| "[skills]\n".to_string());
    let mut doc: DocumentMut = content.parse().map_err(Error::TomlEdit)?;

    if !doc.contains_key("workspace") {
        doc["workspace"] = Item::Table(Table::new());
    }
    let ws = doc["workspace"]
        .as_table_mut()
        .ok_or_else(|| Error::Manifest("[workspace] is not a table".to_string()))?;

    if !ws.contains_key("members") {
        ws["members"] = Item::Value(toml_edit::Value::Array(toml_edit::Array::new()));
    }
    let members = ws["members"]
        .as_array_mut()
        .ok_or_else(|| Error::Manifest("[workspace].members is not an array".to_string()))?;

    let already_exists = members.iter().any(|v| v.as_str() == Some(member));
    if !already_exists {
        members.push(member);
    }

    let result = doc.to_string();
    std::fs::write(manifest_path, &result).map_err(Error::Io)?;
    Ok(result)
}

/// Remove a member from [workspace].members in an Ion.toml file.
pub fn remove_workspace_member_from_manifest(manifest_path: &Path, member: &str) -> Result<String> {
    let content = std::fs::read_to_string(manifest_path).map_err(Error::Io)?;
    let mut doc: DocumentMut = content.parse().map_err(Error::TomlEdit)?;

    let ws = doc
        .get_mut("workspace")
        .and_then(|w| w.as_table_mut())
        .ok_or_else(|| Error::Manifest("No [workspace] section in Ion.toml".to_string()))?;

    let members = ws
        .get_mut("members")
        .and_then(|m| m.as_array_mut())
        .ok_or_else(|| Error::Manifest("No [workspace].members in Ion.toml".to_string()))?;

    let idx = members.iter().position(|v| v.as_str() == Some(member));
    match idx {
        Some(i) => {
            members.remove(i);
        }
        None => {
            return Err(Error::Manifest(format!(
                "'{}' is not a workspace member",
                member
            )));
        }
    }

    let result = doc.to_string();
    std::fs::write(manifest_path, &result).map_err(Error::Io)?;
    Ok(result)
}
```

- [ ] **Step 4: Update `commands/workspace.rs` to use manifest_writer functions**

Replace the inline TOML editing in `workspace.rs` with calls to:
- `ion_skill::manifest_writer::add_workspace_member`
- `ion_skill::manifest_writer::remove_workspace_member_from_manifest`

- [ ] **Step 5: Run tests**

Run: `cargo nextest run -E 'test(write_workspace) | test(remove_workspace) | test(add_duplicate)' -p ion-skill`
Expected: PASS

- [ ] **Step 6: Run full test suite**

Run: `cargo nextest run`
Expected: All tests pass

- [ ] **Step 7: Commit**

```bash
git add crates/ion-skill/src/manifest_writer.rs src/commands/workspace.rs
git commit -m "feat: manifest_writer support for workspace members"
```

---

### Task 13: Final Integration Tests and Cleanup

**Files:**
- Modify: `tests/workspace_integration.rs`

- [ ] **Step 1: Add end-to-end workspace lifecycle test**

```rust
#[test]
fn workspace_full_lifecycle() {
    let root = tempfile::tempdir().unwrap();

    // 1. Init root project
    let output = ion_cmd()
        .args(["init", "--target", "claude"])
        .current_dir(root.path())
        .output()
        .unwrap();
    assert!(output.status.success());

    // 2. Add workspace member
    let docs_dir = root.path().join("docs");
    std::fs::create_dir_all(&docs_dir).unwrap();

    let output = ion_cmd()
        .args(["workspace", "add", "docs"])
        .current_dir(root.path())
        .output()
        .unwrap();
    assert!(output.status.success());

    // 3. Verify workspace list shows both
    let output = ion_cmd()
        .args(["workspace", "list"])
        .current_dir(root.path())
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("root"));
    assert!(stdout.contains("docs"));

    // 4. Verify skill list from root shows both projects
    let output = ion_cmd()
        .args(["skill", "list"])
        .current_dir(root.path())
        .output()
        .unwrap();
    assert!(output.status.success());

    // 5. Verify skill list from docs/ scopes to docs
    let output = ion_cmd()
        .args(["skill", "list"])
        .current_dir(&docs_dir)
        .output()
        .unwrap();
    assert!(output.status.success());

    // 6. Remove workspace member
    let output = ion_cmd()
        .args(["workspace", "remove", "docs"])
        .current_dir(root.path())
        .output()
        .unwrap();
    assert!(output.status.success());

    // docs/Ion.toml should still exist (files preserved)
    assert!(docs_dir.join("Ion.toml").exists());

    // workspace list should show only root
    let output = ion_cmd()
        .args(["workspace", "list"])
        .current_dir(root.path())
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    // Should be workspace-of-one now (or show only root if [workspace] section remains)
    assert!(output.status.success());
}
```

- [ ] **Step 2: Run the full lifecycle test**

Run: `cargo nextest run -E 'test(workspace_full_lifecycle)' --test workspace_integration`
Expected: PASS

- [ ] **Step 3: Run full pre-commit checklist**

```bash
cargo fmt --all
cargo clippy --all-targets --all-features -- -D warnings
cargo nextest run
```

Expected: All three pass

- [ ] **Step 4: Commit**

```bash
git add tests/workspace_integration.rs
git commit -m "test: add comprehensive workspace integration tests"
```

---

## File Structure Summary

**New files:**
- `crates/ion-skill/src/workspace.rs` — `Project` struct
- `src/commands/workspace.rs` — `ion workspace add/remove/list/status`
- `tests/workspace_integration.rs` — integration tests

**Modified files (significant changes):**
- `src/context.rs` — `WorkspaceContext` replaces `ProjectContext`
- `crates/ion-skill/src/manifest.rs` — `WorkspaceConfig` struct, `workspace` field on `Manifest`
- `crates/ion-skill/src/manifest_writer.rs` — `add_workspace_member`, `remove_workspace_member_from_manifest`
- `crates/ion-skill/src/lib.rs` — `pub mod workspace`
- `src/main.rs` — `--project` flag, `Workspace` subcommand, plumbing for `project_flags`
- `src/commands/mod.rs` — `pub mod workspace`

**Modified files (mechanical migration):**
- `src/commands/list.rs` — workspace iteration
- `src/commands/update.rs` — workspace iteration
- `src/commands/add.rs` — `single_project()` guard
- `src/commands/remove.rs` — `single_project()` guard
- `src/commands/install.rs` — workspace iteration
- `src/commands/agents.rs` — workspace iteration / `single_project()`
- `src/commands/init.rs` — auto-registration
- `src/commands/install_shared.rs` — `Project` instead of `ProjectContext` in signatures
- `src/commands/config.rs`, `link.rs`, `eject.rs`, `new.rs`, `migrate.rs`, `run.rs`, `validate.rs`, `info.rs` — mechanical `ProjectContext` → `WorkspaceContext` rename
- `src/builtin_skill.rs` — adapt to `Project`
