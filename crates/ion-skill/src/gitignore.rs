use std::path::Path;

use crate::{Error, Result};

/// Check which directories from the given list are missing from .gitignore.
/// Returns the list of directories that are NOT in .gitignore.
pub fn find_missing_gitignore_entries(project_dir: &Path, dirs: &[&str]) -> Result<Vec<String>> {
    let gitignore_path = project_dir.join(".gitignore");
    let content = std::fs::read_to_string(&gitignore_path).unwrap_or_default();

    let existing: Vec<&str> = content.lines().map(|l| l.trim()).collect();

    Ok(dirs
        .iter()
        .filter(|d| !existing.contains(d))
        .map(|d| d.to_string())
        .collect())
}

/// Append entries to .gitignore, creating it if it doesn't exist.
pub fn append_to_gitignore(project_dir: &Path, entries: &[&str]) -> Result<()> {
    let gitignore_path = project_dir.join(".gitignore");
    let mut content = std::fs::read_to_string(&gitignore_path).unwrap_or_default();

    // Ensure there's a newline before our additions
    if !content.is_empty() && !content.ends_with('\n') {
        content.push('\n');
    }

    // Add a section comment
    content.push_str("\n# Managed by ion\n");
    for entry in entries {
        content.push_str(entry);
        content.push('\n');
    }

    std::fs::write(&gitignore_path, &content).map_err(Error::Io)?;
    Ok(())
}

/// Add per-skill gitignore entries for a remotely installed skill.
/// Creates entries for `<skills_dir>/<name>` and `<target>/<name>` for each target.
/// Idempotent — won't duplicate existing entries.
pub fn add_skill_entries(
    project_dir: &Path,
    skill_name: &str,
    target_paths: &[&str],
    skills_dir: &str,
) -> Result<()> {
    let gitignore_path = project_dir.join(".gitignore");
    let mut content = std::fs::read_to_string(&gitignore_path).unwrap_or_default();

    let mut entries_to_add = vec![format!("{skills_dir}/{skill_name}")];
    for target in target_paths {
        entries_to_add.push(format!("{target}/{skill_name}"));
    }

    // Filter out entries that already exist
    let existing_lines: Vec<&str> = content.lines().map(|l| l.trim()).collect();
    let new_entries: Vec<&String> = entries_to_add
        .iter()
        .filter(|e| !existing_lines.contains(&e.as_str()))
        .collect();

    if new_entries.is_empty() {
        return Ok(());
    }

    // Ensure trailing newline
    if !content.is_empty() && !content.ends_with('\n') {
        content.push('\n');
    }

    // Add managed section header if not present
    if !content.contains("# Managed by ion") {
        content.push_str("\n# Managed by ion\n");
    }

    for entry in new_entries {
        content.push_str(entry);
        content.push('\n');
    }

    std::fs::write(&gitignore_path, &content).map_err(Error::Io)?;
    Ok(())
}

/// Remove all gitignore entries for a specific skill.
/// Removes any line ending with `/<name>` under the managed section.
/// Cleans up the "# Managed by ion" header if no managed entries remain.
pub fn remove_skill_entries(project_dir: &Path, skill_name: &str) -> Result<()> {
    let gitignore_path = project_dir.join(".gitignore");
    let content = match std::fs::read_to_string(&gitignore_path) {
        Ok(c) => c,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(e) => return Err(Error::Io(e)),
    };

    let skill_suffix = format!("/{skill_name}");
    let filtered: Vec<&str> = content
        .lines()
        .filter(|line| {
            let trimmed = line.trim();
            !trimmed.ends_with(&skill_suffix)
        })
        .collect();

    // Clean up empty managed section
    let mut result: Vec<&str> = Vec::new();
    for (i, line) in filtered.iter().enumerate() {
        if line.trim() == "# Managed by ion" {
            // Check if there are any non-empty lines after this before the next section/end
            let has_entries = filtered[i + 1..]
                .iter()
                .take_while(|l| !l.starts_with('#') || l.trim().is_empty())
                .any(|l| !l.trim().is_empty());
            if !has_entries {
                // Skip this header and any trailing blank line before it
                while result.last().is_some_and(|l: &&str| l.trim().is_empty()) {
                    result.pop();
                }
                continue;
            }
        }
        result.push(line);
    }

    let mut output = result.join("\n");
    if !output.is_empty() && !output.ends_with('\n') {
        output.push('\n');
    }

    std::fs::write(&gitignore_path, &output).map_err(Error::Io)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_missing_entries() {
        let project = tempfile::tempdir().unwrap();
        std::fs::write(project.path().join(".gitignore"), ".agents/\n").unwrap();

        let missing =
            find_missing_gitignore_entries(project.path(), &[".agents/", ".claude/"]).unwrap();

        assert_eq!(missing, vec![".claude/"]);
    }

    #[test]
    fn no_gitignore_means_all_missing() {
        let project = tempfile::tempdir().unwrap();

        let missing =
            find_missing_gitignore_entries(project.path(), &[".agents/", ".claude/"]).unwrap();

        assert_eq!(missing, vec![".agents/", ".claude/"]);
    }

    #[test]
    fn append_creates_gitignore() {
        let project = tempfile::tempdir().unwrap();

        append_to_gitignore(project.path(), &[".agents/", ".claude/"]).unwrap();

        let content = std::fs::read_to_string(project.path().join(".gitignore")).unwrap();
        assert!(content.contains(".agents/"));
        assert!(content.contains(".claude/"));
    }

    #[test]
    fn append_adds_to_existing_gitignore() {
        let project = tempfile::tempdir().unwrap();
        std::fs::write(project.path().join(".gitignore"), "node_modules/\n").unwrap();

        append_to_gitignore(project.path(), &[".agents/"]).unwrap();

        let content = std::fs::read_to_string(project.path().join(".gitignore")).unwrap();
        assert!(content.contains("node_modules/"));
        assert!(content.contains(".agents/"));
    }

    #[test]
    fn add_skill_gitignore_entries_creates_section() {
        let project = tempfile::tempdir().unwrap();

        add_skill_entries(
            project.path(),
            "brainstorming",
            &[".claude/skills"],
            ".agents/skills",
        )
        .unwrap();

        let content = std::fs::read_to_string(project.path().join(".gitignore")).unwrap();
        assert!(content.contains("# Managed by ion"));
        assert!(content.contains(".agents/skills/brainstorming"));
        assert!(content.contains(".claude/skills/brainstorming"));
    }

    #[test]
    fn add_skill_gitignore_entries_is_idempotent() {
        let project = tempfile::tempdir().unwrap();

        add_skill_entries(
            project.path(),
            "brainstorming",
            &[".claude/skills"],
            ".agents/skills",
        )
        .unwrap();
        add_skill_entries(
            project.path(),
            "brainstorming",
            &[".claude/skills"],
            ".agents/skills",
        )
        .unwrap();

        let content = std::fs::read_to_string(project.path().join(".gitignore")).unwrap();
        let count = content.matches(".agents/skills/brainstorming").count();
        assert_eq!(count, 1, "should not duplicate entries");
    }

    #[test]
    fn add_skill_gitignore_preserves_existing_content() {
        let project = tempfile::tempdir().unwrap();
        std::fs::write(project.path().join(".gitignore"), "node_modules/\n").unwrap();

        add_skill_entries(
            project.path(),
            "brainstorming",
            &[".claude/skills"],
            ".agents/skills",
        )
        .unwrap();

        let content = std::fs::read_to_string(project.path().join(".gitignore")).unwrap();
        assert!(content.contains("node_modules/"));
        assert!(content.contains(".agents/skills/brainstorming"));
    }

    #[test]
    fn remove_skill_gitignore_entries_removes_all() {
        let project = tempfile::tempdir().unwrap();
        add_skill_entries(
            project.path(),
            "brainstorming",
            &[".claude/skills"],
            ".agents/skills",
        )
        .unwrap();
        add_skill_entries(
            project.path(),
            "writing-plans",
            &[".claude/skills"],
            ".agents/skills",
        )
        .unwrap();

        remove_skill_entries(project.path(), "brainstorming").unwrap();

        let content = std::fs::read_to_string(project.path().join(".gitignore")).unwrap();
        assert!(!content.contains("brainstorming"));
        assert!(content.contains("writing-plans"));
    }

    #[test]
    fn remove_skill_gitignore_entries_noop_if_not_present() {
        let project = tempfile::tempdir().unwrap();
        std::fs::write(project.path().join(".gitignore"), "node_modules/\n").unwrap();

        // Should not error
        remove_skill_entries(project.path(), "brainstorming").unwrap();

        let content = std::fs::read_to_string(project.path().join(".gitignore")).unwrap();
        assert_eq!(content, "node_modules/\n");
    }

    #[test]
    fn remove_skill_gitignore_cleans_empty_managed_section() {
        let project = tempfile::tempdir().unwrap();
        add_skill_entries(
            project.path(),
            "brainstorming",
            &[".claude/skills"],
            ".agents/skills",
        )
        .unwrap();

        remove_skill_entries(project.path(), "brainstorming").unwrap();

        let content = std::fs::read_to_string(project.path().join(".gitignore")).unwrap();
        // Should not leave behind a dangling "# Managed by ion" with no entries
        assert!(!content.contains("# Managed by ion"));
    }
}
