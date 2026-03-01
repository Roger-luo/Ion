use std::path::{Path, PathBuf};
use std::process::Command;

use crate::{Error, Result};

/// Clone a git repository to a target directory. If it already exists, fetch updates.
pub fn clone_or_fetch(url: &str, target: &Path) -> Result<()> {
    if target.join(".git").exists() {
        let output = Command::new("git")
            .args(["fetch", "--all"])
            .current_dir(target)
            .output()
            .map_err(|e| Error::Git(format!("Failed to run git fetch: {e}")))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(Error::Git(format!("git fetch failed: {stderr}")));
        }
    } else {
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent).map_err(Error::Io)?;
        }

        let output = Command::new("git")
            .args(["clone", url, &target.display().to_string()])
            .output()
            .map_err(|e| Error::Git(format!("Failed to run git clone: {e}")))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(Error::Git(format!("git clone failed: {stderr}")));
        }
    }
    Ok(())
}

/// Checkout a specific ref (branch, tag, or commit SHA).
pub fn checkout(repo_path: &Path, rev: &str) -> Result<()> {
    let output = Command::new("git")
        .args(["checkout", rev])
        .current_dir(repo_path)
        .output()
        .map_err(|e| Error::Git(format!("Failed to run git checkout: {e}")))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(Error::Git(format!("git checkout {rev} failed: {stderr}")));
    }
    Ok(())
}

/// Get the current HEAD commit SHA.
pub fn head_commit(repo_path: &Path) -> Result<String> {
    let output = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(repo_path)
        .output()
        .map_err(|e| Error::Git(format!("Failed to run git rev-parse: {e}")))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(Error::Git(format!("git rev-parse failed: {stderr}")));
    }

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
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
    fn checksum_dir_changes_with_content() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("a.txt"), "hello").unwrap();
        let sum1 = checksum_dir(dir.path()).unwrap();

        std::fs::write(dir.path().join("a.txt"), "changed").unwrap();
        let sum2 = checksum_dir(dir.path()).unwrap();
        assert_ne!(sum1, sum2);
    }
}
