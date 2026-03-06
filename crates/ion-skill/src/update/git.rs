use std::path::{Path, PathBuf};

use crate::installer::{SkillInstaller, data_dir, hash_simple};
use crate::lockfile::LockedSkill;
use crate::skill::SkillMetadata;
use crate::source::SkillSource;
use crate::{Error, git, validate};

use super::{UpdateContext, UpdateInfo, Updater};

/// Updater for Git and GitHub-sourced skills.
pub struct GitUpdater;

impl Updater for GitUpdater {
    fn check(
        &self,
        skill: &LockedSkill,
        source: &SkillSource,
    ) -> crate::Result<Option<UpdateInfo>> {
        let url = source.git_url()?;
        let repo_hash = format!("{:x}", hash_simple(&url));
        let repo_dir = data_dir().join(&repo_hash);

        git::clone_or_fetch(&url, &repo_dir)?;
        git::reset_to_remote_head(&repo_dir)?;

        let new_commit = git::head_commit(&repo_dir)?;
        let old_commit = skill.commit.clone().unwrap_or_default();

        if new_commit == old_commit {
            return Ok(None);
        }

        Ok(Some(UpdateInfo {
            old_version: short_sha(&old_commit),
            new_version: short_sha(&new_commit),
        }))
    }

    fn apply(
        &self,
        skill: &LockedSkill,
        source: &SkillSource,
        ctx: &UpdateContext,
    ) -> crate::Result<LockedSkill> {
        let url = source.git_url()?;
        let repo_hash = format!("{:x}", hash_simple(&url));
        let repo_dir = data_dir().join(&repo_hash);

        // Fetch and advance to latest
        git::clone_or_fetch(&url, &repo_dir)?;
        git::reset_to_remote_head(&repo_dir)?;

        // Resolve the skill directory within the repo
        let skill_dir = resolve_skill_dir(&repo_dir, source.path.as_deref())?;

        // Validate SKILL.md
        let skill_md = skill_dir.join("SKILL.md");
        if !skill_md.exists() {
            return Err(Error::InvalidSkill(format!(
                "No SKILL.md found at {}",
                skill_md.display()
            )));
        }
        let (meta, body) = SkillMetadata::from_file(&skill_md)?;

        let report = validate::validate_skill_dir(&skill_dir, &meta, &body);
        if report.error_count > 0 {
            return Err(Error::ValidationFailed {
                error_count: report.error_count,
                warning_count: report.warning_count,
                info_count: report.info_count,
                report,
            });
        }

        // Deploy symlinks via the installer
        let installer = SkillInstaller::new(ctx.project_dir, ctx.options);
        installer.deploy(&skill.name, &skill_dir)?;

        // Build updated lock entry
        let commit = git::head_commit(&repo_dir).ok();
        let checksum = git::checksum_dir(&skill_dir).ok();
        let git_url = source
            .git_url()
            .ok()
            .unwrap_or_else(|| source.source.clone());

        Ok(LockedSkill {
            name: skill.name.clone(),
            source: git_url,
            path: source.path.clone(),
            version: meta.version().map(|s| s.to_string()),
            commit,
            checksum,
            binary: None,
            binary_version: None,
            binary_checksum: None,
        })
    }
}

/// Resolve the skill directory within a repo, handling subdirectory skills.
fn resolve_skill_dir(repo_dir: &Path, path: Option<&str>) -> crate::Result<PathBuf> {
    match path {
        None => Ok(repo_dir.to_path_buf()),
        Some(p) => {
            let direct = repo_dir.join(p);
            if direct.exists() {
                return Ok(direct);
            }
            let fallback = repo_dir.join("skills").join(p);
            if fallback.exists() {
                return Ok(fallback);
            }
            Err(Error::Source(format!(
                "Skill path '{p}' not found in repository (also tried 'skills/{p}')"
            )))
        }
    }
}

/// Return a short (7-char) prefix of a SHA, or the full string if shorter.
fn short_sha(sha: &str) -> String {
    sha.get(..7).unwrap_or(sha).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::source::SourceType;

    fn make_git_repo(path: &std::path::Path) {
        use std::process::Command;
        std::fs::create_dir_all(path).unwrap();
        Command::new("git")
            .args(["init"])
            .current_dir(path)
            .output()
            .unwrap();
        Command::new("git")
            .args(["commit", "--allow-empty", "-m", "init"])
            .current_dir(path)
            .output()
            .unwrap();
    }

    fn add_commit(path: &std::path::Path, msg: &str) {
        use std::process::Command;
        Command::new("git")
            .args(["commit", "--allow-empty", "-m", msg])
            .current_dir(path)
            .output()
            .unwrap();
    }

    fn git_source(url: &str) -> SkillSource {
        SkillSource {
            source_type: SourceType::Git,
            source: url.to_string(),
            path: None,
            rev: None,
            version: None,
            binary: None,
            asset_pattern: None,
            forked_from: None,
        }
    }

    #[test]
    fn short_sha_truncates() {
        assert_eq!(short_sha("abcdef1234567890"), "abcdef1");
        assert_eq!(short_sha("abc"), "abc");
    }

    #[test]
    fn resolve_skill_dir_none_returns_repo() {
        let tmp = tempfile::tempdir().unwrap();
        let result = resolve_skill_dir(tmp.path(), None).unwrap();
        assert_eq!(result, tmp.path());
    }

    #[test]
    fn resolve_skill_dir_direct_path() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(tmp.path().join("my-skill")).unwrap();
        let result = resolve_skill_dir(tmp.path(), Some("my-skill")).unwrap();
        assert_eq!(result, tmp.path().join("my-skill"));
    }

    #[test]
    fn resolve_skill_dir_skills_fallback() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(tmp.path().join("skills").join("my-skill")).unwrap();
        let result = resolve_skill_dir(tmp.path(), Some("my-skill")).unwrap();
        assert_eq!(result, tmp.path().join("skills").join("my-skill"));
    }

    #[test]
    fn resolve_skill_dir_missing_returns_error() {
        let tmp = tempfile::tempdir().unwrap();
        let result = resolve_skill_dir(tmp.path(), Some("nonexistent"));
        assert!(result.is_err());
    }

    #[test]
    fn check_detects_new_commit() {
        let tmp = tempfile::tempdir().unwrap();
        let upstream = tmp.path().join("upstream");
        make_git_repo(&upstream);

        // Clone it so we have a cached copy
        let clone_dir = tmp.path().join("clone");
        git::clone_or_fetch(&upstream.display().to_string(), &clone_dir).unwrap();
        let old_commit = git::head_commit(&clone_dir).unwrap();

        // Add a new commit upstream
        add_commit(&upstream, "second commit");

        // Build a locked skill pointing to the old commit
        let locked = LockedSkill {
            name: "test-skill".to_string(),
            source: upstream.display().to_string(),
            path: None,
            version: None,
            commit: Some(old_commit),
            checksum: None,
            binary: None,
            binary_version: None,
            binary_checksum: None,
        };

        // Use the upstream path as a Git source — but we need to ensure
        // the updater uses the same repo_dir. Override data_dir by using
        // the source directly.
        let source = git_source(&upstream.display().to_string());

        let updater = GitUpdater;
        let result = updater.check(&locked, &source).unwrap();
        assert!(result.is_some(), "should detect an update");
        let info = result.unwrap();
        assert_ne!(info.old_version, info.new_version);
    }

    #[test]
    fn check_returns_none_when_up_to_date() {
        let tmp = tempfile::tempdir().unwrap();
        let upstream = tmp.path().join("upstream");
        make_git_repo(&upstream);

        let source = git_source(&upstream.display().to_string());

        // Clone via the same hashing mechanism the updater uses
        let url = source.git_url().unwrap();
        let repo_hash = format!("{:x}", hash_simple(&url));
        let repo_dir = data_dir().join(&repo_hash);
        git::clone_or_fetch(&url, &repo_dir).unwrap();
        let current_commit = git::head_commit(&repo_dir).unwrap();

        let locked = LockedSkill {
            name: "test-skill".to_string(),
            source: upstream.display().to_string(),
            path: None,
            version: None,
            commit: Some(current_commit),
            checksum: None,
            binary: None,
            binary_version: None,
            binary_checksum: None,
        };

        let updater = GitUpdater;
        let result = updater.check(&locked, &source).unwrap();
        assert!(result.is_none(), "should be up to date");
    }
}
