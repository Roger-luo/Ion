//! GitHub CLI wrappers.

use crate::{Result, is_available, run_command, run_status};

/// Check if `gh` CLI is installed.
pub fn available() -> bool {
    is_available("gh")
}

/// Run an arbitrary `gh` command with the given args. Returns stdout.
pub fn run(args: &[&str]) -> Result<String> {
    log::debug!("gh: running gh {}", args.join(" "));
    let output = run_command(std::process::Command::new("gh").args(args), "gh")?;
    log::debug!("gh: returned {} bytes", output.len());
    Ok(output)
}

/// Star a GitHub repository.
pub fn star_repo(repo: &str) -> Result<()> {
    run_status(
        std::process::Command::new("gh")
            .args(["repo", "star", repo])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null()),
        "gh",
    )
}
