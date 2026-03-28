pub mod cargo;
pub mod gh;
pub mod git;

use std::process::{Command, Stdio};

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

// ---------------------------------------------------------------------------
// Cli — root type for each CLI tool
// ---------------------------------------------------------------------------

/// Descriptor for a CLI tool — binary name and install hint.
///
/// Each module exposes a `CLI` constant of this type. The hint is attached to
/// every `NotFound` error so the user always knows how to install the tool.
///
/// ```ignore
/// // Upfront check
/// gh::CLI.require()?;
///
/// // Or just run — hint is attached automatically on NotFound
/// gh::CLI.run_command(&mut gh::CLI.command().args(["repo", "view"]))?;
/// ```
pub struct Cli {
    /// The binary name (e.g., `"git"`, `"gh"`, `"cargo"`).
    pub name: &'static str,
    /// Hint shown when the binary is not found (e.g., install URL).
    pub hint: &'static str,
}

impl Cli {
    /// Check if this CLI is available on PATH.
    pub fn available(&self) -> bool {
        Command::new(self.name)
            .arg("--version")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .is_ok_and(|s| s.success())
    }

    /// Check availability, returning `Err(NotFound)` with the hint if missing.
    pub fn require(&self) -> Result<()> {
        if self.available() {
            Ok(())
        } else {
            Err(CliError::NotFound {
                cli: self.name.to_string(),
                hint: self.hint.to_string(),
            })
        }
    }

    /// Create a new [`Command`] for this CLI.
    pub fn command(&self) -> Command {
        Command::new(self.name)
    }

    /// Run a command and return its stdout as a String.
    pub(crate) fn run_command(&self, cmd: &mut Command) -> Result<String> {
        let output = cmd.output().map_err(|e| self.spawn_error(e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
            return Err(CliError::Failed {
                cli: self.name.to_string(),
                code: output.status.code().unwrap_or(-1),
                stderr,
            });
        }

        String::from_utf8(output.stdout)
            .map(|s| s.trim_end().to_string())
            .map_err(|_| CliError::InvalidUtf8 {
                cli: self.name.to_string(),
            })
    }

    /// Run a command and return only the exit status.
    pub(crate) fn run_status(&self, cmd: &mut Command) -> Result<()> {
        let status = cmd.status().map_err(|e| self.spawn_error(e))?;

        if !status.success() {
            return Err(CliError::Failed {
                cli: self.name.to_string(),
                code: status.code().unwrap_or(-1),
                stderr: String::new(),
            });
        }

        Ok(())
    }

    fn spawn_error(&self, e: std::io::Error) -> CliError {
        if e.kind() == std::io::ErrorKind::NotFound {
            CliError::NotFound {
                cli: self.name.to_string(),
                hint: self.hint.to_string(),
            }
        } else {
            CliError::Spawn {
                cli: self.name.to_string(),
                source: e,
            }
        }
    }
}
