use std::path::{Path, PathBuf};

use ion_skill::config::GlobalConfig;
use ion_skill::manifest::ManifestOptions;
use ion_skill::workspace::Project;

/// Scope determines which projects a command operates on.
#[derive(Debug, Clone)]
pub enum Scope {
    /// Operate on all projects (root + members). Used when CWD is workspace root.
    All,
    /// Operate on specific projects by index (0 = root, 1+ = members).
    Projects(Vec<usize>),
}

/// Workspace-first context that replaces `ProjectContext`.
///
/// Every invocation works through this type. A project without `[workspace]` in
/// its `Ion.toml` is a "workspace-of-one": `projects` has a single entry and
/// `scope` is `Scope::Projects(vec![0])`, so `single_project()` always succeeds.
pub struct WorkspaceContext {
    pub projects: Vec<Project>,
    pub scope: Scope,
    pub global_config: GlobalConfig,
    /// Whether the root Ion.toml has a `[workspace]` section.
    has_workspace_section: bool,
}

impl WorkspaceContext {
    /// Load the workspace context.
    ///
    /// `project_flags` corresponds to future `--project` flag values. For now all
    /// callers pass `&[]`.
    ///
    /// Discovery logic:
    /// 1. Walk up from CWD looking for `Ion.toml`.
    /// 2. If one has `[workspace]`, that is the root -- load members.
    /// 3. If CWD is inside a member, scope to that member.
    /// 4. If no `[workspace]`, nearest `Ion.toml` is workspace-of-one,
    ///    scope = `Projects(vec![0])`.
    /// 5. If no `Ion.toml` found, use CWD as root (for init command).
    pub fn load(project_flags: &[String]) -> anyhow::Result<Self> {
        let cwd = std::env::current_dir()?;
        let global_config = GlobalConfig::load()?;

        // Walk up from CWD to find Ion.toml files
        let (root_dir, has_workspace) = find_workspace_root(&cwd);

        let mut projects = vec![Project::new(root_dir.clone())];

        if has_workspace {
            // Load workspace members from the root manifest
            if let Ok(manifest) = projects[0].manifest()
                && let Some(ref ws) = manifest.workspace
            {
                for member_path in &ws.members {
                    let member_dir = root_dir.join(member_path);
                    projects.push(Project::new(member_dir));
                }
            }
        }

        // Determine scope
        let scope = if !project_flags.is_empty() {
            // --project flags override everything
            let indices = resolve_project_flags(&projects, &root_dir, project_flags)?;
            Scope::Projects(indices)
        } else if has_workspace {
            // Check if CWD is inside a member
            if let Some(idx) = find_member_index(&projects, &cwd) {
                Scope::Projects(vec![idx])
            } else if cwd == root_dir {
                Scope::All
            } else {
                // CWD is under root but not in a member -- scope to root
                Scope::Projects(vec![0])
            }
        } else {
            // Workspace-of-one
            Scope::Projects(vec![0])
        };

        Ok(Self {
            projects,
            scope,
            global_config,
            has_workspace_section: has_workspace,
        })
    }

    /// Projects that are in scope for the current command.
    pub fn scoped_projects(&self) -> Vec<&Project> {
        match &self.scope {
            Scope::All => self.projects.iter().collect(),
            Scope::Projects(indices) => indices
                .iter()
                .filter_map(|&i| self.projects.get(i))
                .collect(),
        }
    }

    /// The single project in scope. Errors if multiple projects are in scope.
    pub fn single_project(&self) -> anyhow::Result<&Project> {
        let scoped = self.scoped_projects();
        if scoped.len() != 1 {
            anyhow::bail!(
                "This command requires a single project but {} are in scope. \
                 Use --project to select one.",
                scoped.len()
            );
        }
        Ok(scoped[0])
    }

    /// The root project (always index 0).
    pub fn root_project(&self) -> &Project {
        &self.projects[0]
    }

    /// The root directory of the workspace.
    pub fn root_dir(&self) -> &Path {
        &self.projects[0].dir
    }

    /// Whether the root project declares a `[workspace]` section.
    ///
    /// Returns `true` even for a workspace with zero members (e.g. before
    /// any members have been registered). This differs from checking
    /// `projects.len() > 1`, which only tells you if members have been loaded.
    pub fn is_workspace(&self) -> bool {
        self.has_workspace_section
    }

    /// Merge global config + root options + local options for a project.
    ///
    /// If the project IS the root, just merge global + root options.
    /// If it is a member, inherit from root first, then override with local.
    pub fn merged_options_for(&self, project: &Project) -> anyhow::Result<ManifestOptions> {
        let root_manifest = self.projects[0].manifest_or_empty()?;
        let root_options = &root_manifest.options;

        let is_root =
            std::ptr::eq(project, &self.projects[0]) || project.dir == self.projects[0].dir;

        let effective = if is_root {
            root_options.clone()
        } else {
            project.effective_options(root_options)?
        };

        // Merge with global targets
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

    /// Ensure the built-in ion-cli skill is deployed for a project.
    pub fn ensure_builtin_skill(&self, project: &Project, merged_options: &ManifestOptions) {
        if let Err(e) = crate::builtin_skill::ensure_installed(
            &project.dir,
            &project.manifest_path,
            merged_options,
        ) {
            log::warn!("Failed to install built-in ion-cli skill: {e}");
        }
    }
}

/// Walk up from `start` looking for an `Ion.toml`.
/// Returns `(root_dir, has_workspace)`.
/// If no `Ion.toml` is found, returns `(start, false)`.
fn find_workspace_root(start: &Path) -> (PathBuf, bool) {
    // First pass: find the nearest Ion.toml
    let mut nearest: Option<PathBuf> = None;
    let mut dir = start.to_path_buf();
    loop {
        let candidate = dir.join("Ion.toml");
        if candidate.exists() {
            if nearest.is_none() {
                nearest = Some(dir.clone());
            }
            // Check if this one has [workspace]
            if has_workspace_section(&candidate) {
                return (dir, true);
            }
        }
        if !dir.pop() {
            break;
        }
    }

    // No [workspace] found. Use the nearest Ion.toml, or CWD if none.
    match nearest {
        Some(d) => (d, false),
        None => (start.to_path_buf(), false),
    }
}

/// Quick check if an Ion.toml file contains a [workspace] section.
fn has_workspace_section(path: &Path) -> bool {
    if let Ok(content) = std::fs::read_to_string(path)
        && let Ok(manifest) = ion_skill::manifest::Manifest::parse(&content)
    {
        return manifest.workspace.is_some();
    }
    false
}

/// Find the member index if CWD is inside one of the member directories.
fn find_member_index(projects: &[Project], cwd: &Path) -> Option<usize> {
    // Start from index 1 (members), check if CWD starts with any member dir
    for (i, project) in projects.iter().enumerate().skip(1) {
        if cwd.starts_with(&project.dir) {
            return Some(i);
        }
    }
    None
}

/// Resolve --project flags to project indices.
fn resolve_project_flags(
    projects: &[Project],
    root_dir: &Path,
    flags: &[String],
) -> anyhow::Result<Vec<usize>> {
    let mut indices = Vec::new();
    for flag in flags {
        if flag == "." {
            // Resolve "." to the project at CWD, not always root
            let cwd = std::env::current_dir()?;
            let idx = projects.iter().position(|p| p.dir == cwd).unwrap_or(0); // fall back to root if CWD isn't a registered project
            indices.push(idx);
        } else {
            let target = root_dir.join(flag);
            let found = projects
                .iter()
                .position(|p| p.dir == target)
                .ok_or_else(|| {
                    anyhow::anyhow!("No project found at '{}' in this workspace", flag)
                })?;
            indices.push(found);
        }
    }
    if indices.is_empty() {
        anyhow::bail!("No projects matched the given --project flags");
    }
    Ok(indices)
}

/// Type alias for backward compatibility during migration.
#[allow(dead_code)]
pub type ProjectContext = WorkspaceContext;
