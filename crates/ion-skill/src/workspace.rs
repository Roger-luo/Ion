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
