use std::path::PathBuf;

use ion_skill::config::GlobalConfig;
use ion_skill::lockfile::Lockfile;
use ion_skill::manifest::{Manifest, ManifestOptions};

/// Shared project context used across commands.
/// Loads global config eagerly; manifest and lockfile are loaded on demand.
pub struct ProjectContext {
    pub project_dir: PathBuf,
    pub manifest_path: PathBuf,
    pub lockfile_path: PathBuf,
    pub global_config: GlobalConfig,
}

impl ProjectContext {
    pub fn load() -> anyhow::Result<Self> {
        let project_dir = std::env::current_dir()?;
        let manifest_path = project_dir.join("Ion.toml");
        let lockfile_path = project_dir.join("Ion.lock");
        let global_config = GlobalConfig::load()?;
        Ok(Self {
            project_dir,
            manifest_path,
            lockfile_path,
            global_config,
        })
    }

    pub fn manifest(&self) -> anyhow::Result<Manifest> {
        Manifest::from_file(&self.manifest_path).map_err(Into::into)
    }

    pub fn manifest_or_empty(&self) -> anyhow::Result<Manifest> {
        if self.manifest_path.exists() {
            self.manifest()
        } else {
            Ok(Manifest::empty())
        }
    }

    pub fn lockfile(&self) -> anyhow::Result<Lockfile> {
        Lockfile::from_file(&self.lockfile_path).map_err(Into::into)
    }

    pub fn merged_options(&self, manifest: &Manifest) -> ManifestOptions {
        let merged_targets = self.global_config.resolve_targets(&manifest.options);
        ManifestOptions {
            targets: merged_targets,
            skills_dir: manifest.options.skills_dir.clone(),
        }
    }

    pub fn require_manifest(&self) -> anyhow::Result<()> {
        if !self.manifest_path.exists() {
            anyhow::bail!("No Ion.toml found in current directory");
        }
        Ok(())
    }
}
