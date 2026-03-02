use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::installer::SkillInstaller;
use crate::lockfile::{LockedSkill, Lockfile};
use crate::manifest::ManifestOptions;
use crate::manifest_writer;
use crate::skill::SkillMetadata;
use crate::source::SkillSource;
use crate::{Error, Result};

// ---------------------------------------------------------------------------
// skills-lock.json types
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
pub struct SkillsLockFile {
    #[allow(dead_code)]
    pub version: u32,
    pub skills: BTreeMap<String, SkillsLockEntry>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillsLockEntry {
    pub source: String,
    pub source_type: String,
    #[allow(dead_code)]
    pub computed_hash: String,
}

// ---------------------------------------------------------------------------
// Discovered skill
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiscoveryOrigin {
    LockFile,
    AgentsDir,
    ClaudeDir,
}

#[derive(Debug, Clone)]
pub struct DiscoveredSkill {
    pub name: String,
    pub source: Option<SkillSource>,
    pub version: Option<String>,
    pub installed_path: PathBuf,
    pub origin: DiscoveryOrigin,
}

// ---------------------------------------------------------------------------
// Discovery
// ---------------------------------------------------------------------------

/// Parse a skills-lock.json file and return discovered skills.
pub fn discover_from_lockfile(lockfile_path: &Path) -> Result<Vec<DiscoveredSkill>> {
    let content = std::fs::read_to_string(lockfile_path).map_err(Error::Io)?;
    let lock: SkillsLockFile =
        serde_json::from_str(&content).map_err(|e| Error::Manifest(format!("Invalid skills-lock.json: {e}")))?;

    let mut skills = Vec::new();

    for (name, entry) in &lock.skills {
        let source = match entry.source_type.as_str() {
            "github" => {
                // source is "owner/repo", skill name is the key
                // Full shorthand: owner/repo/skill-name
                let shorthand = format!("{}/{}", entry.source, name);
                SkillSource::infer(&shorthand).ok()
            }
            "git" => SkillSource::infer(&entry.source).ok(),
            _ => None,
        };

        // Try to read version from installed SKILL.md if it exists
        let installed_path = PathBuf::from(".agents").join("skills").join(name);
        let version = installed_path
            .join("SKILL.md")
            .exists()
            .then(|| {
                SkillMetadata::from_file(&installed_path.join("SKILL.md"))
                    .ok()
                    .and_then(|(meta, _)| meta.version().map(|v| v.to_string()))
            })
            .flatten();

        skills.push(DiscoveredSkill {
            name: name.clone(),
            source,
            version,
            installed_path,
            origin: DiscoveryOrigin::LockFile,
        });
    }

    skills.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(skills)
}

/// Scan .agents/skills/ and .claude/skills/ for installed skills.
pub fn discover_from_directories(project_dir: &Path) -> Result<Vec<DiscoveredSkill>> {
    let mut skills = BTreeMap::new();

    for (dir, origin) in [
        (project_dir.join(".agents").join("skills"), DiscoveryOrigin::AgentsDir),
        (project_dir.join(".claude").join("skills"), DiscoveryOrigin::ClaudeDir),
    ] {
        if !dir.exists() {
            continue;
        }

        let entries = std::fs::read_dir(&dir).map_err(Error::Io)?;
        for entry in entries {
            let entry = entry.map_err(Error::Io)?;
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }

            let skill_md = path.join("SKILL.md");
            if !skill_md.exists() {
                continue;
            }

            let name = entry
                .file_name()
                .to_string_lossy()
                .to_string();

            // Don't overwrite if already found from .agents/skills/
            if skills.contains_key(&name) {
                continue;
            }

            let version = SkillMetadata::from_file(&skill_md)
                .ok()
                .and_then(|(meta, _)| meta.version().map(|v| v.to_string()));

            skills.insert(
                name.clone(),
                DiscoveredSkill {
                    name,
                    source: None,
                    version,
                    installed_path: path,
                    origin,
                },
            );
        }
    }

    Ok(skills.into_values().collect())
}

// ---------------------------------------------------------------------------
// Migration options
// ---------------------------------------------------------------------------

/// Per-skill resolution provided by the CLI layer after prompting the user.
pub struct ResolvedSkill {
    pub name: String,
    pub source: SkillSource,
    pub rev: Option<String>,
}

pub struct MigrateOptions {
    pub dry_run: bool,
    pub manifest_options: ManifestOptions,
}

// ---------------------------------------------------------------------------
// Migration execution
// ---------------------------------------------------------------------------

/// Execute migration for a list of resolved skills.
/// Returns (migrated count, list of locked skills).
pub fn migrate(
    project_dir: &Path,
    resolved: &[ResolvedSkill],
    options: &MigrateOptions,
) -> Result<Vec<LockedSkill>> {
    if options.dry_run {
        return Ok(Vec::new());
    }

    let manifest_path = project_dir.join("ion.toml");
    let lockfile_path = project_dir.join("ion.lock");

    let mut lockfile = Lockfile::from_file(&lockfile_path)?;
    let mut locked_skills = Vec::new();

    let installer = SkillInstaller::new(project_dir, &options.manifest_options);
    for skill in resolved {
        let mut source = skill.source.clone();
        if let Some(ref rev) = skill.rev {
            source.rev = Some(rev.clone());
        }

        let locked = installer.install(&skill.name, &source)?;
        manifest_writer::add_skill(&manifest_path, &skill.name, &source)?;
        lockfile.upsert(locked.clone());
        locked_skills.push(locked);
    }

    lockfile.write_to(&lockfile_path)?;

    Ok(locked_skills)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_skills_lock_json() {
        let json = r#"{
            "version": 1,
            "skills": {
                "brainstorming": {
                    "source": "obra/superpowers",
                    "sourceType": "github",
                    "computedHash": "abc123"
                },
                "dispatching-parallel-agents": {
                    "source": "obra/superpowers",
                    "sourceType": "github",
                    "computedHash": "def456"
                }
            }
        }"#;

        let lock: SkillsLockFile = serde_json::from_str(json).unwrap();
        assert_eq!(lock.version, 1);
        assert_eq!(lock.skills.len(), 2);
        assert_eq!(lock.skills["brainstorming"].source, "obra/superpowers");
        assert_eq!(lock.skills["brainstorming"].source_type, "github");
    }

    #[test]
    fn discover_from_lockfile_builds_source() {
        let dir = tempfile::tempdir().unwrap();
        let lock_path = dir.path().join("skills-lock.json");
        std::fs::write(
            &lock_path,
            r#"{
                "version": 1,
                "skills": {
                    "brainstorming": {
                        "source": "obra/superpowers",
                        "sourceType": "github",
                        "computedHash": "abc"
                    }
                }
            }"#,
        )
        .unwrap();

        let skills = discover_from_lockfile(&lock_path).unwrap();
        assert_eq!(skills.len(), 1);
        assert_eq!(skills[0].name, "brainstorming");
        assert_eq!(skills[0].origin, DiscoveryOrigin::LockFile);

        let source = skills[0].source.as_ref().unwrap();
        assert_eq!(source.source, "obra/superpowers");
        assert_eq!(source.path.as_deref(), Some("brainstorming"));
    }

    #[test]
    fn discover_from_directories_finds_skills() {
        let dir = tempfile::tempdir().unwrap();

        // Create .agents/skills/my-skill/SKILL.md
        let skill_dir = dir.path().join(".agents").join("skills").join("my-skill");
        std::fs::create_dir_all(&skill_dir).unwrap();
        std::fs::write(
            skill_dir.join("SKILL.md"),
            "---\nname: my-skill\ndescription: A test skill.\n---\n\nBody.\n",
        )
        .unwrap();

        let skills = discover_from_directories(dir.path()).unwrap();
        assert_eq!(skills.len(), 1);
        assert_eq!(skills[0].name, "my-skill");
        assert!(skills[0].source.is_none());
        assert_eq!(skills[0].origin, DiscoveryOrigin::AgentsDir);
    }

    #[test]
    fn discover_from_directories_reads_version() {
        let dir = tempfile::tempdir().unwrap();
        let skill_dir = dir.path().join(".agents").join("skills").join("versioned");
        std::fs::create_dir_all(&skill_dir).unwrap();
        std::fs::write(
            skill_dir.join("SKILL.md"),
            "---\nname: versioned\ndescription: Has version.\nmetadata:\n  version: \"2.0\"\n---\n\nBody.\n",
        )
        .unwrap();

        let skills = discover_from_directories(dir.path()).unwrap();
        assert_eq!(skills[0].version.as_deref(), Some("2.0"));
    }

    #[test]
    fn discover_from_directories_scans_claude_dir() {
        let dir = tempfile::tempdir().unwrap();

        let skill_dir = dir.path().join(".claude").join("skills").join("claude-skill");
        std::fs::create_dir_all(&skill_dir).unwrap();
        std::fs::write(
            skill_dir.join("SKILL.md"),
            "---\nname: claude-skill\ndescription: Claude dir skill.\n---\n\nBody.\n",
        )
        .unwrap();

        let skills = discover_from_directories(dir.path()).unwrap();
        assert_eq!(skills.len(), 1);
        assert_eq!(skills[0].name, "claude-skill");
        assert_eq!(skills[0].origin, DiscoveryOrigin::ClaudeDir);
    }

    #[test]
    fn discover_from_directories_skips_without_skill_md() {
        let dir = tempfile::tempdir().unwrap();

        let skill_dir = dir.path().join(".agents").join("skills").join("no-manifest");
        std::fs::create_dir_all(&skill_dir).unwrap();
        std::fs::write(skill_dir.join("README.md"), "not a skill").unwrap();

        let skills = discover_from_directories(dir.path()).unwrap();
        assert!(skills.is_empty());
    }

    #[test]
    fn discover_from_directories_empty_when_no_dirs() {
        let dir = tempfile::tempdir().unwrap();
        let skills = discover_from_directories(dir.path()).unwrap();
        assert!(skills.is_empty());
    }
}
