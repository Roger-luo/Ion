//! Git CLI wrappers.
//!
//! Use [`require()`] to verify `git` is installed, then call methods on the
//! returned [`Git`] handle:
//!
//! ```ignore
//! let git = git::require()?;
//! let repo = git.repo(project_dir);
//! repo.stage_files(&["Ion.toml", "Ion.lock"])?;
//! if repo.has_staged_changes()? {
//!     let sha = repo.create_commit("chore: update manifest")?;
//! }
//! ```

use std::path::Path;

use crate::{Cli, CliError, Result};

/// The `git` CLI descriptor.
pub const CLI: Cli = Cli {
    name: "git",
    hint: "Install from https://git-scm.com",
};

/// Verify `git` is installed and return a handle to run commands.
pub fn require() -> Result<Git> {
    CLI.require()?;
    Ok(Git)
}

/// A validated handle proving the `git` CLI is available.
///
/// Obtained via [`require()`]. Context constructors and standalone
/// operations live here.
pub struct Git;

impl Git {
    /// Create a [`Repo`] context bound to the given working directory.
    pub fn repo<'a>(&self, path: &'a Path) -> Repo<'a> {
        repo(path)
    }

    /// Clone a git repository, or fetch updates if it already exists.
    pub fn clone_or_fetch(&self, url: &str, target: &Path) -> Result<()> {
        clone_or_fetch(url, target)
    }

    /// Initialize a new git repository.
    pub fn init(&self, path: &Path) -> Result<()> {
        init(path)
    }
}

/// Clone a git repository to a target directory. If it already exists, fetch updates.
pub fn clone_or_fetch(url: &str, target: &Path) -> Result<()> {
    if target.join(".git").exists() {
        CLI.run_status(CLI.command().args(["fetch", "--all"]).current_dir(target))
    } else {
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent).map_err(|e| CliError::Spawn {
                cli: CLI.name.to_string(),
                source: e,
            })?;
        }

        CLI.run_status(
            CLI.command()
                .args(["clone", url, &target.display().to_string()]),
        )
    }
}

/// Checkout a specific ref (branch, tag, or commit SHA).
pub fn checkout(repo: &Path, rev: &str) -> Result<()> {
    CLI.run_status(CLI.command().args(["checkout", rev]).current_dir(repo))
}

/// Get the current HEAD commit SHA.
pub fn head_commit(repo: &Path) -> Result<String> {
    CLI.run_command(CLI.command().args(["rev-parse", "HEAD"]).current_dir(repo))
}

/// Get the default branch name for a repo by checking `origin/HEAD` or falling back
/// to `symbolic-ref HEAD`.
pub fn default_branch(repo: &Path) -> Result<String> {
    // Try origin/HEAD first (works for cloned repos)
    let origin_result = CLI.run_command(
        CLI.command()
            .args(["symbolic-ref", "refs/remotes/origin/HEAD"])
            .current_dir(repo),
    );

    if let Ok(full_ref) = origin_result
        && let Some(branch) = full_ref.strip_prefix("refs/remotes/origin/")
    {
        return Ok(branch.to_string());
    }

    // Fallback: local HEAD's branch name
    let local_result = CLI.run_command(
        CLI.command()
            .args(["symbolic-ref", "--short", "HEAD"])
            .current_dir(repo),
    );

    match local_result {
        Ok(branch) => Ok(branch),
        Err(_) => Err(CliError::Failed {
            cli: CLI.name.to_string(),
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

    CLI.run_status(
        CLI.command()
            .args(["reset", "--hard", &remote_ref])
            .current_dir(repo),
    )
}

/// Stage files in a git repository.
pub fn stage_files(repo: &Path, files: &[&str]) -> Result<()> {
    let mut cmd = CLI.command();
    cmd.arg("add")
        .current_dir(repo)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null());
    for file in files {
        cmd.arg(file);
    }
    CLI.run_status(&mut cmd)
}

/// Check if there are staged changes in a git repository.
///
/// Returns `true` if there are staged changes, `false` if there are none.
/// `git diff --cached --quiet` exits with code 1 when there are changes.
pub fn has_staged_changes(repo: &Path) -> Result<bool> {
    let result = CLI.run_status(
        CLI.command()
            .args(["diff", "--cached", "--quiet"])
            .current_dir(repo)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null()),
    );

    match result {
        Ok(()) => Ok(false),
        Err(CliError::Failed { code: 1, .. }) => Ok(true),
        Err(e) => Err(e),
    }
}

/// Create a commit with the given message and return the new HEAD commit SHA.
pub fn create_commit(repo: &Path, message: &str) -> Result<String> {
    CLI.run_status(
        CLI.command()
            .args(["commit", "-m", message])
            .current_dir(repo)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null()),
    )?;

    head_commit(repo)
}

/// Initialize a new git repository at the given path.
pub fn init(path: &Path) -> Result<()> {
    CLI.run_status(CLI.command().args(["init", &path.display().to_string()]))
}

// ---------------------------------------------------------------------------
// Repo context
// ---------------------------------------------------------------------------

/// Create a [`Repo`] context bound to the given working directory.
pub fn repo(path: &Path) -> Repo<'_> {
    Repo { path }
}

/// A git repository context that binds a working directory, so you can call
/// multiple operations without repeating the path.
pub struct Repo<'a> {
    path: &'a Path,
}

impl<'a> Repo<'a> {
    /// Checkout a specific ref (branch, tag, or commit SHA).
    pub fn checkout(&self, rev: &str) -> Result<()> {
        checkout(self.path, rev)
    }

    /// Get the current HEAD commit SHA.
    pub fn head_commit(&self) -> Result<String> {
        head_commit(self.path)
    }

    /// Get the default branch name.
    pub fn default_branch(&self) -> Result<String> {
        default_branch(self.path)
    }

    /// Reset the working tree to the remote's default branch HEAD.
    pub fn reset_to_remote_head(&self) -> Result<()> {
        reset_to_remote_head(self.path)
    }

    /// Stage files.
    pub fn stage_files(&self, files: &[&str]) -> Result<()> {
        stage_files(self.path, files)
    }

    /// Check if there are staged changes.
    pub fn has_staged_changes(&self) -> Result<bool> {
        has_staged_changes(self.path)
    }

    /// Create a commit with the given message and return the new HEAD SHA.
    pub fn create_commit(&self, message: &str) -> Result<String> {
        create_commit(self.path, message)
    }

    /// Fetch updates from all remotes (requires an existing clone).
    pub fn fetch_all(&self) -> Result<()> {
        CLI.run_status(
            CLI.command()
                .args(["fetch", "--all"])
                .current_dir(self.path),
        )
    }
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
