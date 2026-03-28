pub mod cargo;
pub mod gh;
pub mod git;
pub mod shell;

use std::process::Command;

/// Error from a CLI invocation.
#[derive(Debug, thiserror::Error)]
pub enum CliError {
    /// The CLI binary was not found on PATH.
    #[error("{cli} not found. {hint}")]
    NotFound { cli: String, hint: String },

    /// The command ran but exited with a non-zero status.
    #[error("{cli} failed (exit {code}): {stderr}")]
    Failed {
        cli: String,
        code: i32,
        stderr: String,
    },

    /// I/O error spawning the process.
    #[error("failed to spawn {cli}: {source}")]
    Spawn { cli: String, source: std::io::Error },

    /// Output was not valid UTF-8.
    #[error("{cli} produced invalid UTF-8 output")]
    InvalidUtf8 { cli: String },
}

pub type Result<T> = std::result::Result<T, CliError>;

/// Check if a CLI is available on PATH.
pub fn is_available(name: &str) -> bool {
    Command::new(name)
        .arg("--version")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .is_ok_and(|s| s.success())
}

/// Run a command and return its stdout as a String, or a CliError.
#[allow(dead_code)]
pub(crate) fn run_command(cmd: &mut Command, cli_name: &str) -> Result<String> {
    let output = cmd.output().map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            CliError::NotFound {
                cli: cli_name.to_string(),
                hint: String::new(),
            }
        } else {
            CliError::Spawn {
                cli: cli_name.to_string(),
                source: e,
            }
        }
    })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(CliError::Failed {
            cli: cli_name.to_string(),
            code: output.status.code().unwrap_or(-1),
            stderr,
        });
    }

    String::from_utf8(output.stdout)
        .map(|s| s.trim_end().to_string())
        .map_err(|_| CliError::InvalidUtf8 {
            cli: cli_name.to_string(),
        })
}

/// Run a command and return only the exit status.
#[allow(dead_code)]
pub(crate) fn run_status(cmd: &mut Command, cli_name: &str) -> Result<()> {
    let status = cmd.status().map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            CliError::NotFound {
                cli: cli_name.to_string(),
                hint: String::new(),
            }
        } else {
            CliError::Spawn {
                cli: cli_name.to_string(),
                source: e,
            }
        }
    })?;

    if !status.success() {
        return Err(CliError::Failed {
            cli: cli_name.to_string(),
            code: status.code().unwrap_or(-1),
            stderr: String::new(),
        });
    }

    Ok(())
}
