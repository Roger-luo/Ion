use crate::binary;
use crate::lockfile::LockedSkill;
use crate::source::SkillSource;

use super::{UpdateContext, UpdateInfo, Updater};

/// Updater for binary skills installed from GitHub Releases.
pub struct BinaryUpdater;

impl Updater for BinaryUpdater {
    fn check(&self, skill: &LockedSkill, source: &SkillSource) -> crate::Result<Option<UpdateInfo>> {
        let release = binary::fetch_github_release(&source.source, source.rev.as_deref())?;
        let latest_version = binary::parse_version_from_tag(&release.tag_name).to_string();

        let current_version = skill
            .binary_version
            .as_deref()
            .unwrap_or("unknown")
            .to_string();

        if current_version == latest_version {
            return Ok(None);
        }

        Ok(Some(UpdateInfo {
            old_version: current_version,
            new_version: latest_version,
        }))
    }

    fn apply(
        &self,
        skill: &LockedSkill,
        source: &SkillSource,
        ctx: &UpdateContext,
    ) -> crate::Result<LockedSkill> {
        let binary_name = source.binary.as_deref().unwrap_or(&skill.name);
        let skill_dir = ctx
            .project_dir
            .join(".agents")
            .join("skills")
            .join(&skill.name);

        let result = binary::install_binary_from_github(
            &source.source,
            binary_name,
            source.rev.as_deref(),
            &skill_dir,
        )?;

        // Clean up old version if different
        if let Some(ref old_version) = skill.binary_version {
            if *old_version != result.version {
                let _ = binary::remove_binary_version(binary_name, old_version);
            }
        }

        // Build updated lock entry, preserving non-binary fields
        let mut updated = skill.clone();
        updated.binary_version = Some(result.version);
        updated.binary_checksum = Some(result.binary_checksum);

        Ok(updated)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn binary_updater_implements_trait() {
        let _updater = BinaryUpdater;
    }
}
