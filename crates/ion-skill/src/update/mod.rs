pub mod binary;
pub mod git;

use crate::installer::SkillInstaller;
use crate::lockfile::LockedSkill;
use crate::source::SkillSource;

/// Information about an available update.
#[derive(Debug)]
pub struct UpdateInfo {
    /// Human-readable description of the old version (e.g., commit SHA prefix or version string).
    pub old_version: String,
    /// Human-readable description of the new version.
    pub new_version: String,
}

/// Trait for source-type-specific update logic.
pub trait Updater {
    /// Check if an update is available. Returns `Some(UpdateInfo)` if yes, `None` if up to date.
    fn check(&self, skill: &LockedSkill, source: &SkillSource)
    -> crate::Result<Option<UpdateInfo>>;

    /// Apply the update: fetch new version, validate, deploy, return updated lock entry.
    fn apply(
        &self,
        skill: &LockedSkill,
        source: &SkillSource,
        installer: &SkillInstaller,
    ) -> crate::Result<LockedSkill>;
}
