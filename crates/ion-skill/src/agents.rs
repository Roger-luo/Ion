use std::collections::BTreeMap;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::Result;

/// Configuration for AGENTS.md template management.
/// Parsed from [agents] in Ion.toml.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct AgentsConfig {
    /// Template source (GitHub shorthand, Git URL, HTTP, or local path)
    #[serde(default)]
    pub template: Option<String>,
    /// Pin to a specific git revision
    #[serde(default)]
    pub rev: Option<String>,
    /// Path to AGENTS.md within the source repo (default: "AGENTS.md" at root)
    #[serde(default)]
    pub path: Option<String>,
}

/// Lock entry for the AGENTS.md template.
/// Tracks the last-synced state in Ion.lock.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct AgentsLockEntry {
    pub template: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rev: Option<String>,
    pub checksum: String,
    pub updated_at: String, // ISO 8601, stored as plain string
}

/// Mapping of target names to the agent instructions filename that needs a symlink.
/// Only targets whose tools don't read AGENTS.md natively need an entry here.
const AGENT_FILE_SYMLINKS: &[(&str, &str)] = &[("claude", "CLAUDE.md")];

/// For each configured target that has an entry in AGENT_FILE_SYMLINKS,
/// create a symlink (e.g. CLAUDE.md -> AGENTS.md) if AGENTS.md exists
/// and the symlink doesn't.
///
/// Symlinks are only created for targets configured in [options.targets].
/// If a target filename already exists as a regular file or a symlink
/// pointing elsewhere, a warning is printed and it is skipped.
pub fn ensure_agent_symlinks(project_dir: &Path, targets: &BTreeMap<String, String>) -> Result<()> {
    let agents_md = project_dir.join("AGENTS.md");
    if !agents_md.exists() {
        return Ok(());
    }

    for (target_name, symlink_filename) in AGENT_FILE_SYMLINKS {
        if !targets.contains_key(*target_name) {
            continue;
        }

        let symlink_path = project_dir.join(symlink_filename);

        match std::fs::symlink_metadata(&symlink_path) {
            Ok(meta) if meta.is_symlink() => {
                if let Ok(target) = std::fs::read_link(&symlink_path)
                    && target == std::path::Path::new("AGENTS.md")
                {
                    continue; // Already correct
                }
                eprintln!(
                    "Warning: {} already exists as a symlink pointing elsewhere, skipping",
                    symlink_filename
                );
                continue;
            }
            Ok(_) => {
                eprintln!(
                    "Warning: {} already exists as a file, skipping symlink \
                     (remove it manually if you want ion to manage it)",
                    symlink_filename
                );
                continue;
            }
            Err(_) => {
                // Doesn't exist — create it
            }
        }

        #[cfg(unix)]
        std::os::unix::fs::symlink("AGENTS.md", &symlink_path).map_err(crate::Error::Io)?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    #[test]
    fn creates_claude_symlink_when_agents_md_exists() {
        let project = tempfile::tempdir().unwrap();
        std::fs::write(project.path().join("AGENTS.md"), "# My Agents\n").unwrap();

        let mut targets = BTreeMap::new();
        targets.insert("claude".to_string(), ".claude/skills".to_string());

        ensure_agent_symlinks(project.path(), &targets).unwrap();

        let symlink = project.path().join("CLAUDE.md");
        assert!(symlink.exists(), "CLAUDE.md symlink should exist");
        assert!(symlink.symlink_metadata().unwrap().is_symlink());
    }

    #[test]
    fn no_symlink_when_agents_md_missing() {
        let project = tempfile::tempdir().unwrap();

        let mut targets = BTreeMap::new();
        targets.insert("claude".to_string(), ".claude/skills".to_string());

        ensure_agent_symlinks(project.path(), &targets).unwrap();

        assert!(!project.path().join("CLAUDE.md").exists());
    }

    #[test]
    fn no_symlink_for_non_claude_target() {
        let project = tempfile::tempdir().unwrap();
        std::fs::write(project.path().join("AGENTS.md"), "# Agents\n").unwrap();

        let mut targets = BTreeMap::new();
        targets.insert("cursor".to_string(), ".cursor/skills".to_string());

        ensure_agent_symlinks(project.path(), &targets).unwrap();

        assert!(!project.path().join("CLAUDE.md").exists());
    }

    #[test]
    fn skips_existing_regular_file() {
        let project = tempfile::tempdir().unwrap();
        std::fs::write(project.path().join("AGENTS.md"), "# Agents\n").unwrap();
        std::fs::write(project.path().join("CLAUDE.md"), "# Existing\n").unwrap();

        let mut targets = BTreeMap::new();
        targets.insert("claude".to_string(), ".claude/skills".to_string());

        ensure_agent_symlinks(project.path(), &targets).unwrap();

        let meta = std::fs::symlink_metadata(project.path().join("CLAUDE.md")).unwrap();
        assert!(!meta.is_symlink());
    }

    #[test]
    fn skips_existing_symlink_pointing_elsewhere() {
        let project = tempfile::tempdir().unwrap();
        std::fs::write(project.path().join("AGENTS.md"), "# Agents\n").unwrap();
        std::fs::write(project.path().join("OTHER.md"), "# Other\n").unwrap();

        #[cfg(unix)]
        std::os::unix::fs::symlink("OTHER.md", project.path().join("CLAUDE.md")).unwrap();

        let mut targets = BTreeMap::new();
        targets.insert("claude".to_string(), ".claude/skills".to_string());

        ensure_agent_symlinks(project.path(), &targets).unwrap();

        let target = std::fs::read_link(project.path().join("CLAUDE.md")).unwrap();
        assert_eq!(target, std::path::Path::new("OTHER.md"));
    }

    #[test]
    fn idempotent_symlink_creation() {
        let project = tempfile::tempdir().unwrap();
        std::fs::write(project.path().join("AGENTS.md"), "# Agents\n").unwrap();

        let mut targets = BTreeMap::new();
        targets.insert("claude".to_string(), ".claude/skills".to_string());

        ensure_agent_symlinks(project.path(), &targets).unwrap();
        ensure_agent_symlinks(project.path(), &targets).unwrap();

        let symlink = project.path().join("CLAUDE.md");
        assert!(symlink.symlink_metadata().unwrap().is_symlink());
    }
}
