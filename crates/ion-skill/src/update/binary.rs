use crate::binary;
use crate::installer::SkillInstaller;
use crate::lockfile::LockedSkill;
use crate::source::SkillSource;

use super::{UpdateInfo, Updater};

/// Updater for binary skills installed from GitHub Releases.
pub struct BinaryUpdater;

impl Updater for BinaryUpdater {
    fn check(
        &self,
        skill: &LockedSkill,
        source: &SkillSource,
    ) -> crate::Result<Option<UpdateInfo>> {
        if source.source.starts_with("http://") || source.source.starts_with("https://") {
            // URL-based binary sources don't support automatic update checking
            return Ok(None);
        }

        let release = binary::fetch_github_release(&source.source, source.rev.as_deref())?;
        let latest_version = binary::parse_version_from_tag(&release.tag_name).to_string();

        let current_version = skill.binary_version().unwrap_or("unknown").to_string();

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
        installer: &SkillInstaller,
    ) -> crate::Result<LockedSkill> {
        if source.source.starts_with("http://") || source.source.starts_with("https://") {
            // URL-based binary sources don't support automatic updates
            return Ok(skill.clone());
        }

        let (binary_name_owned, asset_pattern) = match &source.kind {
            crate::source::SkillSourceKind::Binary {
                binary_name,
                asset_pattern,
                ..
            } => (binary_name.clone(), asset_pattern.clone()),
            _ => (String::new(), None),
        };
        let binary_name = if binary_name_owned.is_empty() {
            &skill.name
        } else {
            &binary_name_owned
        };
        let skill_dir = installer.skill_dir(&skill.name);

        let result = binary::install_binary_from_github(
            &source.source,
            binary_name,
            source.rev.as_deref(),
            &skill_dir,
            asset_pattern.as_deref(),
        )?;

        // Clean up old version if different
        if let Some(old_version) = skill.binary_version()
            && old_version != result.version
        {
            let _ = binary::remove_binary_version(binary_name, old_version);
        }

        // Build updated lock entry, preserving non-binary fields
        let mut locked = LockedSkill::binary(
            skill.name.clone(),
            skill.source.clone(),
            binary_name,
            Some(result.version),
            Some(result.binary_checksum),
        );
        if let Some(path) = skill.path.clone() {
            locked = locked.with_path(path);
        }
        if let Some(version) = skill.version.clone() {
            locked = locked.with_version(version);
        }

        Ok(locked)
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
