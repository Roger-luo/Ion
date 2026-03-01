use std::path::{Path, PathBuf};

use crate::lockfile::LockedSkill;
use crate::manifest::ManifestOptions;
use crate::skill::SkillMetadata;
use crate::source::{SkillSource, SourceType};
use crate::{Error, Result, git};

/// Where ion caches cloned repositories.
fn cache_dir() -> PathBuf {
    dirs::cache_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("ion")
        .join("repos")
}

/// Install a single skill from a resolved source into a project directory.
pub fn install_skill(
    project_dir: &Path,
    name: &str,
    source: &SkillSource,
    options: &ManifestOptions,
) -> Result<LockedSkill> {
    let skill_dir = fetch_skill(source)?;

    // Validate SKILL.md exists and is valid
    let skill_md = skill_dir.join("SKILL.md");
    if !skill_md.exists() {
        return Err(Error::InvalidSkill(format!(
            "No SKILL.md found at {}",
            skill_md.display()
        )));
    }

    let (meta, _body) = SkillMetadata::from_file(&skill_md)?;

    // Version check
    if let Some(ref required_version) = source.version {
        let actual_version = meta.version().unwrap_or("(none)");
        if actual_version != required_version {
            return Err(Error::InvalidSkill(format!(
                "Version mismatch: expected {required_version}, found {actual_version}"
            )));
        }
    }

    // Copy to .agents/skills/<name>/
    let agents_target = project_dir.join(".agents").join("skills").join(name);
    copy_skill_dir(&skill_dir, &agents_target)?;

    // Create symlinks for each configured target
    let canonical = project_dir.join(".agents").join("skills").join(name);
    for target_path in options.targets.values() {
        let target_skill_dir = project_dir.join(target_path).join(name);
        create_skill_symlink(&canonical, &target_skill_dir)?;
    }

    // Build locked entry
    let (commit, checksum) = match source.source_type {
        SourceType::Github | SourceType::Git => {
            let repo_dir = find_repo_root(&skill_dir);
            let commit = git::head_commit(&repo_dir).ok();
            let checksum = git::checksum_dir(&skill_dir).ok();
            (commit, checksum)
        }
        SourceType::Path | SourceType::Http => {
            let checksum = git::checksum_dir(&skill_dir).ok();
            (None, checksum)
        }
    };

    let git_url = source.git_url().ok().unwrap_or_else(|| source.source.clone());

    Ok(LockedSkill {
        name: name.to_string(),
        source: git_url,
        path: source.path.clone(),
        version: meta.version().map(|s| s.to_string()),
        commit,
        checksum,
    })
}

/// Fetch a skill source to a local directory. Returns the path to the skill directory.
fn fetch_skill(source: &SkillSource) -> Result<PathBuf> {
    match source.source_type {
        SourceType::Github | SourceType::Git => {
            let url = source.git_url()?;
            let repo_hash = format!("{:x}", hash_simple(&url));
            let repo_dir = cache_dir().join(&repo_hash);

            git::clone_or_fetch(&url, &repo_dir)?;

            if let Some(ref rev) = source.rev {
                git::checkout(&repo_dir, rev)?;
            }

            match &source.path {
                Some(path) => {
                    let skill_dir = repo_dir.join(path);
                    if skill_dir.exists() {
                        return Ok(skill_dir);
                    }
                    // Fallback: try skills/<path> (common convention)
                    let fallback_dir = repo_dir.join("skills").join(path);
                    if fallback_dir.exists() {
                        return Ok(fallback_dir);
                    }
                    Err(Error::Source(format!(
                        "Skill path '{path}' not found in repository (also tried 'skills/{path}')"
                    )))
                }
                None => Ok(repo_dir),
            }
        }
        SourceType::Path => {
            let path = PathBuf::from(&source.source);
            if !path.exists() {
                return Err(Error::Source(format!(
                    "Local path does not exist: {}", source.source
                )));
            }
            Ok(path)
        }
        SourceType::Http => {
            Err(Error::Source("HTTP source not yet implemented".to_string()))
        }
    }
}

/// Copy a skill directory to a target location (overwriting if it exists).
fn copy_skill_dir(src: &Path, dst: &Path) -> Result<()> {
    if dst.exists() {
        std::fs::remove_dir_all(dst).map_err(Error::Io)?;
    }
    if let Some(parent) = dst.parent() {
        std::fs::create_dir_all(parent).map_err(Error::Io)?;
    }
    copy_dir_recursive(src, dst)
}

fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<()> {
    std::fs::create_dir_all(dst).map_err(Error::Io)?;
    for entry in std::fs::read_dir(src).map_err(Error::Io)? {
        let entry = entry.map_err(Error::Io)?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());
        if src_path.file_name().is_some_and(|n| n == ".git") {
            continue;
        }
        if src_path.is_dir() {
            copy_dir_recursive(&src_path, &dst_path)?;
        } else {
            std::fs::copy(&src_path, &dst_path).map_err(Error::Io)?;
        }
    }
    Ok(())
}

/// Create a relative symlink from `link` pointing to `original`.
fn create_skill_symlink(original: &Path, link: &Path) -> Result<()> {
    // Remove existing file/dir/symlink at the link location
    if link.is_symlink() {
        std::fs::remove_file(link).map_err(Error::Io)?;
    } else if link.exists() {
        std::fs::remove_dir_all(link).map_err(Error::Io)?;
    }

    // Ensure parent directory exists
    if let Some(parent) = link.parent() {
        std::fs::create_dir_all(parent).map_err(Error::Io)?;
    }

    // Compute relative path from link's parent to the original
    let link_parent = link.parent().unwrap();
    let relative = pathdiff::diff_paths(original, link_parent)
        .ok_or_else(|| Error::Io(std::io::Error::other(
            format!("Cannot compute relative path from {} to {}", link_parent.display(), original.display()),
        )))?;

    #[cfg(unix)]
    std::os::unix::fs::symlink(&relative, link).map_err(Error::Io)?;

    #[cfg(windows)]
    std::os::windows::fs::symlink_dir(&relative, link).map_err(Error::Io)?;

    Ok(())
}

/// Remove an installed skill from the project directory.
pub fn uninstall_skill(project_dir: &Path, name: &str, options: &ManifestOptions) -> Result<()> {
    // Remove canonical copy
    let agents_dir = project_dir.join(".agents").join("skills").join(name);
    if agents_dir.exists() {
        std::fs::remove_dir_all(&agents_dir).map_err(Error::Io)?;
    }

    // Remove symlinks from all targets
    for target_path in options.targets.values() {
        let target_dir = project_dir.join(target_path).join(name);
        if target_dir.is_symlink() {
            std::fs::remove_file(&target_dir).map_err(Error::Io)?;
        } else if target_dir.exists() {
            std::fs::remove_dir_all(&target_dir).map_err(Error::Io)?;
        }
    }

    Ok(())
}

fn find_repo_root(path: &Path) -> PathBuf {
    let mut current = path.to_path_buf();
    loop {
        if current.join(".git").exists() {
            return current;
        }
        if !current.pop() {
            return path.to_path_buf();
        }
    }
}

fn hash_simple(s: &str) -> u64 {
    use std::hash::{DefaultHasher, Hash, Hasher};
    let mut hasher = DefaultHasher::new();
    s.hash(&mut hasher);
    hasher.finish()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn copy_skill_dir_works() {
        let src = tempfile::tempdir().unwrap();
        std::fs::write(src.path().join("SKILL.md"), "---\nname: test\ndescription: Test.\n---\nBody").unwrap();
        std::fs::create_dir(src.path().join("scripts")).unwrap();
        std::fs::write(src.path().join("scripts").join("run.sh"), "#!/bin/bash").unwrap();

        let dst_dir = tempfile::tempdir().unwrap();
        let dst = dst_dir.path().join("test-skill");
        copy_skill_dir(src.path(), &dst).unwrap();

        assert!(dst.join("SKILL.md").exists());
        assert!(dst.join("scripts").join("run.sh").exists());
    }

    #[test]
    fn copy_skill_dir_skips_git() {
        let src = tempfile::tempdir().unwrap();
        std::fs::write(src.path().join("SKILL.md"), "content").unwrap();
        std::fs::create_dir(src.path().join(".git")).unwrap();
        std::fs::write(src.path().join(".git").join("HEAD"), "ref").unwrap();

        let dst_dir = tempfile::tempdir().unwrap();
        let dst = dst_dir.path().join("out");
        copy_skill_dir(src.path(), &dst).unwrap();

        assert!(dst.join("SKILL.md").exists());
        assert!(!dst.join(".git").exists());
    }

    #[test]
    fn uninstall_removes_dirs() {
        let project = tempfile::tempdir().unwrap();
        let agents = project.path().join(".agents").join("skills").join("test");
        std::fs::create_dir_all(&agents).unwrap();
        std::fs::write(agents.join("SKILL.md"), "x").unwrap();

        // Create a symlink target
        let claude = project.path().join(".claude").join("skills");
        std::fs::create_dir_all(&claude).unwrap();
        #[cfg(unix)]
        std::os::unix::fs::symlink(
            std::path::Path::new("../../../.agents/skills/test"),
            claude.join("test"),
        ).unwrap();

        let mut targets = std::collections::BTreeMap::new();
        targets.insert("claude".to_string(), ".claude/skills".to_string());
        let options = ManifestOptions { targets };
        uninstall_skill(project.path(), "test", &options).unwrap();

        assert!(!agents.exists());
        assert!(!claude.join("test").exists());
    }

    #[test]
    fn install_creates_symlinks_for_targets() {
        let skill_src = tempfile::tempdir().unwrap();
        std::fs::write(
            skill_src.path().join("SKILL.md"),
            "---\nname: sym-test\ndescription: Symlink test.\n---\n\nBody.\n",
        ).unwrap();

        let project = tempfile::tempdir().unwrap();
        let source = SkillSource {
            source_type: SourceType::Path,
            source: skill_src.path().display().to_string(),
            path: None,
            rev: None,
            version: None,
        };

        let mut targets = std::collections::BTreeMap::new();
        targets.insert("claude".to_string(), ".claude/skills".to_string());
        let options = ManifestOptions { targets };

        let _locked = install_skill(project.path(), "sym-test", &source, &options).unwrap();

        // Canonical copy is a real directory
        let canonical = project.path().join(".agents/skills/sym-test");
        assert!(canonical.exists());
        assert!(canonical.is_dir());
        assert!(!canonical.is_symlink());

        // Target is a symlink
        let target = project.path().join(".claude/skills/sym-test");
        assert!(target.exists());
        assert!(target.is_symlink());

        // Symlink resolves to the right content
        assert!(target.join("SKILL.md").exists());
    }

    #[test]
    fn install_local_skill() {
        let skill_src = tempfile::tempdir().unwrap();
        std::fs::write(
            skill_src.path().join("SKILL.md"),
            "---\nname: local-test\ndescription: A local test skill.\n---\n\nInstructions here.\n",
        ).unwrap();

        let project = tempfile::tempdir().unwrap();
        let source = SkillSource {
            source_type: SourceType::Path,
            source: skill_src.path().display().to_string(),
            path: None,
            rev: None,
            version: None,
        };
        let options = ManifestOptions { targets: std::collections::BTreeMap::new() };

        let locked = install_skill(project.path(), "local-test", &source, &options).unwrap();
        assert_eq!(locked.name, "local-test");
        assert!(project.path().join(".agents/skills/local-test/SKILL.md").exists());
    }
}
