//! Shell command wrappers.

use crate::{Result, run_command};

/// Run a shell command via `sh -c`. Returns stdout.
pub fn run_sh(command: &str) -> Result<String> {
    log::debug!("shell: executing: {command}");
    run_command(
        std::process::Command::new("sh").arg("-c").arg(command),
        "sh",
    )
}
