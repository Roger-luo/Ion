use std::path::{Path, PathBuf};

use crate::{Error, Result};

/// Clone a git repository to a target directory. If it already exists, fetch updates.
pub fn clone_or_fetch(url: &str, target: &Path) -> Result<()> {
    Ok(ion_cli::git::clone_or_fetch(url, target)?)
}

/// Checkout a specific ref (branch, tag, or commit SHA).
pub fn checkout(repo_path: &Path, rev: &str) -> Result<()> {
    Ok(ion_cli::git::checkout(repo_path, rev)?)
}

/// Get the current HEAD commit SHA.
pub fn head_commit(repo_path: &Path) -> Result<String> {
    Ok(ion_cli::git::head_commit(repo_path)?)
}

/// Compute a SHA-256 checksum of a directory's contents (all files, sorted).
pub fn checksum_dir(dir: &Path) -> Result<String> {
    use sha2::{Digest, Sha256};

    let mut hasher = Sha256::new();
    let mut files: Vec<PathBuf> = Vec::new();

    collect_files(dir, &mut files)?;
    files.sort();

    for file in &files {
        let relative = file.strip_prefix(dir).unwrap_or(file);
        hasher.update(relative.to_string_lossy().as_bytes());
        let content = std::fs::read(file).map_err(Error::Io)?;
        hasher.update(&content);
    }

    let hash = hasher.finalize();
    Ok(format!("sha256:{:x}", hash))
}

fn collect_files(dir: &Path, files: &mut Vec<PathBuf>) -> Result<()> {
    for entry in std::fs::read_dir(dir).map_err(Error::Io)? {
        let entry = entry.map_err(Error::Io)?;
        let path = entry.path();
        if path.file_name().is_some_and(|n| n == ".git") {
            continue;
        }
        if path.is_dir() {
            collect_files(&path, files)?;
        } else {
            files.push(path);
        }
    }
    Ok(())
}

/// Get the default branch name for a repo by checking `origin/HEAD` or falling back
/// to `symbolic-ref HEAD`.
pub fn default_branch(repo_path: &Path) -> Result<String> {
    Ok(ion_cli::git::default_branch(repo_path)?)
}

/// Reset the working tree to the remote's default branch HEAD.
/// Call this after `clone_or_fetch()` to advance to the latest commit.
pub fn reset_to_remote_head(repo_path: &Path) -> Result<()> {
    Ok(ion_cli::git::reset_to_remote_head(repo_path)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn checksum_dir_deterministic() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("a.txt"), "hello").unwrap();
        std::fs::write(dir.path().join("b.txt"), "world").unwrap();

        let sum1 = checksum_dir(dir.path()).unwrap();
        let sum2 = checksum_dir(dir.path()).unwrap();
        assert_eq!(sum1, sum2);
        assert!(sum1.starts_with("sha256:"));
    }

    #[test]
    fn default_branch_of_fresh_repo() {
        let tmp = tempfile::tempdir().unwrap();
        let repo = tmp.path();
        std::process::Command::new("git")
            .args(["init"])
            .current_dir(repo)
            .output()
            .unwrap();
        std::process::Command::new("git")
            .args(["commit", "--allow-empty", "-m", "init"])
            .current_dir(repo)
            .output()
            .unwrap();
        let branch = default_branch(repo).unwrap();
        assert!(branch == "main" || branch == "master", "got: {branch}");
    }

    #[test]
    fn checksum_dir_changes_with_content() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("a.txt"), "hello").unwrap();
        let sum1 = checksum_dir(dir.path()).unwrap();

        std::fs::write(dir.path().join("a.txt"), "changed").unwrap();
        let sum2 = checksum_dir(dir.path()).unwrap();
        assert_ne!(sum1, sum2);
    }

    #[test]
    fn reset_to_remote_head_after_clone() {
        let tmp = tempfile::tempdir().unwrap();

        let upstream = tmp.path().join("upstream");
        std::fs::create_dir(&upstream).unwrap();
        std::process::Command::new("git")
            .args(["init"])
            .current_dir(&upstream)
            .output()
            .unwrap();
        std::process::Command::new("git")
            .args(["commit", "--allow-empty", "-m", "first"])
            .current_dir(&upstream)
            .output()
            .unwrap();

        let clone_dir = tmp.path().join("clone");
        clone_or_fetch(&upstream.display().to_string(), &clone_dir).unwrap();
        let commit1 = head_commit(&clone_dir).unwrap();

        std::process::Command::new("git")
            .args(["commit", "--allow-empty", "-m", "second"])
            .current_dir(&upstream)
            .output()
            .unwrap();

        clone_or_fetch(&upstream.display().to_string(), &clone_dir).unwrap();
        reset_to_remote_head(&clone_dir).unwrap();
        let commit2 = head_commit(&clone_dir).unwrap();

        assert_ne!(commit1, commit2, "HEAD should have advanced");
    }
}
