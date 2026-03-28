//! Git CLI wrappers.

use std::path::Path;
use std::process::Command;

use crate::{CliError, Result, run_command, run_status};

/// Clone a git repository to a target directory. If it already exists, fetch updates.
pub fn clone_or_fetch(url: &str, target: &Path) -> Result<()> {
    if target.join(".git").exists() {
        run_status(
            Command::new("git")
                .args(["fetch", "--all"])
                .current_dir(target),
            "git",
        )
    } else {
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent).map_err(|e| CliError::Spawn {
                cli: "git".to_string(),
                source: e,
            })?;
        }

        run_status(
            Command::new("git").args(["clone", url, &target.display().to_string()]),
            "git",
        )
    }
}

/// Checkout a specific ref (branch, tag, or commit SHA).
pub fn checkout(repo: &Path, rev: &str) -> Result<()> {
    run_status(
        Command::new("git")
            .args(["checkout", rev])
            .current_dir(repo),
        "git",
    )
}

/// Get the current HEAD commit SHA.
pub fn head_commit(repo: &Path) -> Result<String> {
    run_command(
        Command::new("git")
            .args(["rev-parse", "HEAD"])
            .current_dir(repo),
        "git",
    )
}

/// Get the default branch name for a repo by checking `origin/HEAD` or falling back
/// to `symbolic-ref HEAD`.
pub fn default_branch(repo: &Path) -> Result<String> {
    // Try origin/HEAD first (works for cloned repos)
    let origin_result = run_command(
        Command::new("git")
            .args(["symbolic-ref", "refs/remotes/origin/HEAD"])
            .current_dir(repo),
        "git",
    );

    if let Ok(full_ref) = origin_result
        && let Some(branch) = full_ref.strip_prefix("refs/remotes/origin/")
    {
        return Ok(branch.to_string());
    }

    // Fallback: local HEAD's branch name
    let local_result = run_command(
        Command::new("git")
            .args(["symbolic-ref", "--short", "HEAD"])
            .current_dir(repo),
        "git",
    );

    match local_result {
        Ok(branch) => Ok(branch),
        Err(_) => Err(CliError::Failed {
            cli: "git".to_string(),
            code: 1,
            stderr: "Could not determine default branch".to_string(),
        }),
    }
}

/// Reset the working tree to the remote's default branch HEAD.
/// Call this after `clone_or_fetch()` to advance to the latest commit.
pub fn reset_to_remote_head(repo: &Path) -> Result<()> {
    let branch = default_branch(repo)?;
    let remote_ref = format!("origin/{branch}");

    run_status(
        Command::new("git")
            .args(["reset", "--hard", &remote_ref])
            .current_dir(repo),
        "git",
    )
}

/// Stage files in a git repository.
pub fn stage_files(repo: &Path, files: &[&str]) -> Result<()> {
    let mut cmd = Command::new("git");
    cmd.arg("add")
        .current_dir(repo)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null());
    for file in files {
        cmd.arg(file);
    }
    run_status(&mut cmd, "git")
}

/// Check if there are staged changes in a git repository.
///
/// Returns `true` if there are staged changes, `false` if there are none.
/// `git diff --cached --quiet` exits with code 1 when there are changes.
pub fn has_staged_changes(repo: &Path) -> Result<bool> {
    let result = run_status(
        Command::new("git")
            .args(["diff", "--cached", "--quiet"])
            .current_dir(repo)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null()),
        "git",
    );

    match result {
        Ok(()) => Ok(false),
        Err(CliError::Failed { code: 1, .. }) => Ok(true),
        Err(e) => Err(e),
    }
}

/// Create a commit with the given message and return the new HEAD commit SHA.
pub fn create_commit(repo: &Path, message: &str) -> Result<String> {
    run_status(
        Command::new("git")
            .args(["commit", "-m", message])
            .current_dir(repo)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null()),
        "git",
    )?;

    head_commit(repo)
}

/// Initialize a new git repository at the given path.
pub fn init(path: &Path) -> Result<()> {
    run_status(
        Command::new("git").args(["init", &path.display().to_string()]),
        "git",
    )
}

#[cfg(test)]
mod tests {
    use super::*;

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

    #[test]
    fn head_commit_returns_sha() {
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

        let sha = head_commit(repo).unwrap();
        assert_eq!(sha.len(), 40, "SHA should be 40 hex chars, got: {sha}");
        assert!(sha.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn checkout_switches_branch() {
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

        // Create a new branch
        std::process::Command::new("git")
            .args(["branch", "test-branch"])
            .current_dir(repo)
            .output()
            .unwrap();

        checkout(repo, "test-branch").unwrap();

        let output = std::process::Command::new("git")
            .args(["symbolic-ref", "--short", "HEAD"])
            .current_dir(repo)
            .output()
            .unwrap();
        let current = String::from_utf8_lossy(&output.stdout).trim().to_string();
        assert_eq!(current, "test-branch");
    }

    #[test]
    fn init_creates_repo() {
        let tmp = tempfile::tempdir().unwrap();
        let repo = tmp.path().join("new-repo");

        init(&repo).unwrap();

        assert!(repo.join(".git").exists(), ".git directory should exist");
    }

    #[test]
    fn stage_files_and_has_staged_changes() {
        let tmp = tempfile::tempdir().unwrap();
        let repo = tmp.path();
        std::process::Command::new("git")
            .args(["init"])
            .current_dir(repo)
            .output()
            .unwrap();

        // No staged changes initially
        // (need at least one commit for diff --cached to work reliably)
        std::process::Command::new("git")
            .args(["commit", "--allow-empty", "-m", "init"])
            .current_dir(repo)
            .output()
            .unwrap();

        assert!(!has_staged_changes(repo).unwrap());

        // Create a file and stage it
        std::fs::write(repo.join("hello.txt"), "hello").unwrap();
        stage_files(repo, &["hello.txt"]).unwrap();

        assert!(has_staged_changes(repo).unwrap());
    }

    #[test]
    fn create_commit_returns_sha() {
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

        let sha1 = head_commit(repo).unwrap();

        std::fs::write(repo.join("file.txt"), "content").unwrap();
        stage_files(repo, &["file.txt"]).unwrap();
        let sha2 = create_commit(repo, "add file").unwrap();

        assert_ne!(sha1, sha2, "commit should create a new SHA");
        assert_eq!(sha2.len(), 40);
        assert!(sha2.chars().all(|c| c.is_ascii_hexdigit()));
    }
}
