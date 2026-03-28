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

    /// Create a `Paint` instance for styled output.
    pub fn paint(&self) -> crate::style::Paint {
        crate::style::Paint::new(&self.global_config)
    }

    /// Resolved skills directory path (absolute).
    #[allow(dead_code)]
    pub fn skills_dir(&self, manifest: &Manifest) -> std::path::PathBuf {
        let options = self.merged_options(manifest);
        self.project_dir.join(options.skills_dir_or_default())
    }

    /// Absolute path to a specific skill's directory.
    #[allow(dead_code)]
    pub fn skill_path(&self, manifest: &Manifest, name: &str) -> std::path::PathBuf {
        self.skills_dir(manifest).join(name)
    }

    /// Create a `SkillInstaller` for this project.
    pub fn installer<'a>(
        &'a self,
        options: &'a ManifestOptions,
    ) -> ion_skill::installer::SkillInstaller<'a> {
        ion_skill::installer::SkillInstaller::new(&self.project_dir, options)
    }

    /// Ensure the built-in ion-cli skill is deployed, logging a warning on failure.
    pub fn ensure_builtin_skill(&self, merged_options: &ManifestOptions) {
        if let Err(e) = crate::builtin_skill::ensure_installed(
            &self.project_dir,
            &self.manifest_path,
            merged_options,
        ) {
            log::warn!("Failed to install built-in ion-cli skill: {e}");
        }
    }
}
