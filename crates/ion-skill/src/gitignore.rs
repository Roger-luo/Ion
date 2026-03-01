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
}
