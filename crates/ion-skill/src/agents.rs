//! AGENTS.md template management — fetch, render, and keep agent context files up to date.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::installer;
use crate::source::SkillSource;
use crate::{Error, Result, git};

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
                    // Already correct — ensure gitignored
                    crate::gitignore::ensure_agent_file_ignored(project_dir, symlink_filename)?;
                    continue;
                }
                eprintln!(
                    "Warning: {} already exists as a symlink pointing elsewhere, skipping",
                    symlink_filename
                );
                continue;
            }
            Ok(_) => {
                // Regular file — check if it's a pointer
                let content = std::fs::read_to_string(&symlink_path).unwrap_or_default();
                if is_agents_pointer(&content) {
                    std::fs::remove_file(&symlink_path).map_err(crate::Error::Io)?;
                    // Fall through to create symlink below
                } else {
                    eprintln!(
                        "Warning: {} already exists as a file, skipping symlink \
                         (remove it manually or run `ion migrate` to convert)",
                        symlink_filename
                    );
                    continue;
                }
            }
            Err(_) => {
                // Doesn't exist — create it
            }
        }

        #[cfg(unix)]
        std::os::unix::fs::symlink("AGENTS.md", &symlink_path).map_err(crate::Error::Io)?;

        crate::gitignore::ensure_agent_file_ignored(project_dir, symlink_filename)?;
    }

    Ok(())
}

/// Check whether a file's content is just a pointer to AGENTS.md.
///
/// Returns `true` if every non-blank line's only purpose is to reference
/// `@AGENTS.md` — e.g. the bare string `@AGENTS.md` or prose like
/// "treat @AGENTS.md the same as this file".
///
/// Returns `false` if the file contains additional instructions beyond
/// the reference, has no `@AGENTS.md` at all, or is empty.
pub fn is_agents_pointer(content: &str) -> bool {
    let non_blank: Vec<&str> = content.lines().filter(|l| !l.trim().is_empty()).collect();

    if non_blank.is_empty() {
        return false;
    }

    // Must contain @AGENTS.md somewhere
    if !non_blank.iter().any(|l| l.contains("@AGENTS.md")) {
        return false;
    }

    // Every non-blank line must mention @AGENTS.md
    non_blank.iter().all(|l| l.contains("@AGENTS.md"))
}

/// SHA-256 checksum of raw content, formatted as "sha256:{hex}".
fn checksum_content(content: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    let hash = Sha256::new().chain_update(content).finalize();
    let bytes: &[u8] = hash.as_ref();
    format!(
        "sha256:{}",
        bytes
            .iter()
            .map(|b| format!("{:02x}", b))
            .collect::<String>()
    )
}

/// Current UTC time as ISO 8601 string (e.g. "2026-03-27T12:00:00Z").
pub fn now_iso8601() -> String {
    use std::time::SystemTime;
    let dur = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default();
    let secs = dur.as_secs();
    let days = secs / 86400;
    let time_secs = secs % 86400;
    let hours = time_secs / 3600;
    let minutes = (time_secs % 3600) / 60;
    let seconds = time_secs % 60;
    let (year, month, day) = epoch_days_to_ymd(days);
    format!("{year:04}-{month:02}-{day:02}T{hours:02}:{minutes:02}:{seconds:02}Z")
}

fn epoch_days_to_ymd(days: u64) -> (u64, u64, u64) {
    // Howard Hinnant's algorithm
    let z = days + 719468;
    let era = z / 146097;
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    (y, m, d)
}

/// Result of fetching an AGENTS.md template
pub struct FetchedTemplate {
    pub content: String,
    pub rev: Option<String>,
    pub checksum: String,
}

/// Return a `FetchedTemplate` from an embedded built-in template.
pub fn fetch_builtin_template(name: &str) -> Result<FetchedTemplate> {
    let content = crate::templates::get(name)
        .ok_or_else(|| Error::Other(format!("Unknown built-in template: {name}")))?;
    let checksum = checksum_content(content.as_bytes());
    Ok(FetchedTemplate {
        content: content.to_string(),
        rev: None,
        checksum,
    })
}

/// Fetch an AGENTS.md template from a source.
///
/// Resolves the source using SkillSource::infer, fetches the repo/path,
/// and extracts the AGENTS.md file at the specified path (default: root).
/// Built-in templates (`builtin:rust`, etc.) are resolved from the binary
/// without any network access.
pub fn fetch_template(
    source_str: &str,
    rev: Option<&str>,
    file_path: Option<&str>,
    _project_dir: &Path,
) -> Result<FetchedTemplate> {
    // Built-in templates are resolved from the binary directly.
    if let Some(name) = crate::templates::parse_builtin_name(source_str) {
        return fetch_builtin_template(name);
    }

    let mut source = SkillSource::infer(source_str)?;
    if let Some(r) = rev {
        source.rev = Some(r.to_string());
    }

    let agents_md_path = file_path.unwrap_or("AGENTS.md");

    let base_path = fetch_source_base(&source)?;

    // If the source resolves to a file, use it directly;
    // otherwise look for AGENTS.md within the directory
    let template_file = if base_path.is_file() {
        base_path.clone()
    } else {
        base_path.join(agents_md_path)
    };

    if !template_file.exists() {
        return Err(Error::Other(format!(
            "AGENTS.md not found in {} at path '{}'",
            source_str, agents_md_path
        )));
    }

    let content = std::fs::read_to_string(&template_file).map_err(Error::Io)?;
    let checksum = checksum_content(content.as_bytes());

    let resolved_rev = if source.is_git_based() {
        let repo_dir = if base_path.is_file() {
            base_path.parent().unwrap_or(&base_path)
        } else {
            &base_path
        };
        git::head_commit(repo_dir).ok()
    } else {
        None
    };

    Ok(FetchedTemplate {
        content,
        rev: resolved_rev,
        checksum,
    })
}

/// Fetch source base directory — reuses installer's git clone/cache logic.
fn fetch_source_base(source: &SkillSource) -> Result<PathBuf> {
    if source.is_git_based() {
        let url = source.git_url()?;
        let repo_hash = format!("{:x}", installer::hash_simple(&url));
        let repo_dir = installer::data_dir().join(&repo_hash);
        git::clone_or_fetch(&url, &repo_dir)?;
        if let Some(ref rev) = source.rev {
            git::checkout(&repo_dir, rev)?;
        }
        Ok(repo_dir)
    } else if source.is_path() {
        let path = PathBuf::from(&source.source);
        if !path.exists() {
            return Err(Error::Source(format!(
                "Local path does not exist: {}",
                source.source
            )));
        }
        Ok(path)
    } else {
        Err(Error::Source(format!(
            "Source kind {:?} is not supported for AGENTS.md templates",
            source.kind
        )))
    }
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

    #[test]
    fn fetch_template_from_local_path() {
        let template_dir = tempfile::tempdir().unwrap();
        std::fs::write(
            template_dir.path().join("AGENTS.md"),
            "# Template Agents\n\nStandard workflows.\n",
        )
        .unwrap();

        let project = tempfile::tempdir().unwrap();

        let result = fetch_template(
            template_dir.path().to_str().unwrap(),
            None,
            None,
            project.path(),
        )
        .unwrap();

        assert_eq!(result.content, "# Template Agents\n\nStandard workflows.\n");
    }

    #[test]
    fn fetch_template_with_custom_path() {
        let template_dir = tempfile::tempdir().unwrap();
        let subdir = template_dir.path().join("templates");
        std::fs::create_dir(&subdir).unwrap();
        std::fs::write(subdir.join("AGENTS.md"), "# Custom Path\n").unwrap();

        let project = tempfile::tempdir().unwrap();

        let result = fetch_template(
            template_dir.path().to_str().unwrap(),
            None,
            Some("templates/AGENTS.md"),
            project.path(),
        )
        .unwrap();

        assert_eq!(result.content, "# Custom Path\n");
    }

    #[test]
    fn fetch_template_missing_file_errors() {
        let template_dir = tempfile::tempdir().unwrap();
        let project = tempfile::tempdir().unwrap();

        let result = fetch_template(
            template_dir.path().to_str().unwrap(),
            None,
            None,
            project.path(),
        );

        assert!(result.is_err());
    }

    #[test]
    fn fetch_template_from_direct_file_path() {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("my-template.md");
        std::fs::write(&file_path, "# Direct File Template\n").unwrap();

        let project = tempfile::tempdir().unwrap();

        let result =
            fetch_template(file_path.to_str().unwrap(), None, None, project.path()).unwrap();

        assert_eq!(result.content, "# Direct File Template\n");
    }

    #[test]
    fn pointer_bare_reference() {
        assert!(is_agents_pointer("@AGENTS.md"));
    }

    #[test]
    fn pointer_with_prose() {
        assert!(is_agents_pointer("treat @AGENTS.md the same as this file"));
    }

    #[test]
    fn pointer_with_whitespace() {
        assert!(is_agents_pointer("\n  @AGENTS.md  \n\n"));
    }

    #[test]
    fn pointer_multiline_prose() {
        assert!(is_agents_pointer(
            "Contents of @AGENTS.md\n\ntreat @AGENTS.md the same as this file"
        ));
    }

    #[test]
    fn not_pointer_no_reference() {
        assert!(!is_agents_pointer("# My Project\n\nSome instructions.\n"));
    }

    #[test]
    fn not_pointer_has_extra_content() {
        assert!(!is_agents_pointer(
            "treat @AGENTS.md the same as this file\n\n# Extra Rules\n\nAlways use TypeScript.\n"
        ));
    }

    #[test]
    fn not_pointer_empty() {
        assert!(!is_agents_pointer(""));
    }

    #[test]
    fn not_pointer_only_whitespace() {
        assert!(!is_agents_pointer("   \n\n  "));
    }

    #[test]
    fn replaces_pointer_file_with_symlink() {
        let project = tempfile::tempdir().unwrap();
        std::fs::write(project.path().join("AGENTS.md"), "# Agents\n").unwrap();
        std::fs::write(project.path().join("CLAUDE.md"), "@AGENTS.md").unwrap();

        let mut targets = BTreeMap::new();
        targets.insert("claude".to_string(), ".claude/skills".to_string());

        ensure_agent_symlinks(project.path(), &targets).unwrap();

        let meta = std::fs::symlink_metadata(project.path().join("CLAUDE.md")).unwrap();
        assert!(
            meta.is_symlink(),
            "pointer file should be replaced with symlink"
        );
    }

    #[test]
    fn skips_real_content_file() {
        let project = tempfile::tempdir().unwrap();
        std::fs::write(project.path().join("AGENTS.md"), "# Agents\n").unwrap();
        std::fs::write(
            project.path().join("CLAUDE.md"),
            "# My Project Rules\n\nAlways use Rust.\n",
        )
        .unwrap();

        let mut targets = BTreeMap::new();
        targets.insert("claude".to_string(), ".claude/skills".to_string());

        ensure_agent_symlinks(project.path(), &targets).unwrap();

        let meta = std::fs::symlink_metadata(project.path().join("CLAUDE.md")).unwrap();
        assert!(
            !meta.is_symlink(),
            "real content file should NOT be replaced"
        );
    }

    #[test]
    fn symlink_creation_gitignores_claude_md() {
        let project = tempfile::tempdir().unwrap();
        std::fs::write(project.path().join("AGENTS.md"), "# Agents\n").unwrap();

        let mut targets = BTreeMap::new();
        targets.insert("claude".to_string(), ".claude/skills".to_string());

        ensure_agent_symlinks(project.path(), &targets).unwrap();

        let gitignore = std::fs::read_to_string(project.path().join(".gitignore")).unwrap();
        assert!(gitignore.contains("CLAUDE.md"));
    }

    #[test]
    fn existing_correct_symlink_gitignores_claude_md() {
        let project = tempfile::tempdir().unwrap();
        std::fs::write(project.path().join("AGENTS.md"), "# Agents\n").unwrap();

        #[cfg(unix)]
        std::os::unix::fs::symlink("AGENTS.md", project.path().join("CLAUDE.md")).unwrap();

        let mut targets = BTreeMap::new();
        targets.insert("claude".to_string(), ".claude/skills".to_string());

        ensure_agent_symlinks(project.path(), &targets).unwrap();

        let gitignore = std::fs::read_to_string(project.path().join(".gitignore")).unwrap();
        assert!(gitignore.contains("CLAUDE.md"));
    }
}
