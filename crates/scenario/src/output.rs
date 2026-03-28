use std::fmt;

/// Captured output from a completed scenario.
///
/// In piped mode, stdout and stderr are captured separately.
/// In PTY mode, stdout contains the combined terminal output and stderr is empty
/// (the PTY merges both streams).
#[derive(Debug, Clone)]
pub struct Output {
    success: bool,
    exit_code: u32,
    stdout: String,
    stderr: String,
    stdout_raw: Vec<u8>,
    stderr_raw: Vec<u8>,
}

impl Output {
    /// Whether the process exited successfully (exit code 0).
    pub fn success(&self) -> bool {
        self.success
    }

    /// The process exit code.
    pub fn exit_code(&self) -> u32 {
        self.exit_code
    }

    /// Stdout as a string with ANSI escape codes stripped.
    ///
    /// In PTY mode, this contains the combined terminal output (stdout + stderr).
    pub fn stdout(&self) -> &str {
        &self.stdout
    }

    /// Stderr as a string with ANSI escape codes stripped.
    ///
    /// In PTY mode, this is empty — stderr is merged into stdout.
    pub fn stderr(&self) -> &str {
        &self.stderr
    }

    /// Raw stdout bytes including ANSI escape codes.
    pub fn stdout_raw(&self) -> &[u8] {
        &self.stdout_raw
    }

    /// Raw stderr bytes including ANSI escape codes.
    pub fn stderr_raw(&self) -> &[u8] {
        &self.stderr_raw
    }

    pub(crate) fn from_piped(output: std::process::Output) -> Self {
        let exit_code = output.status.code().unwrap_or(1) as u32;
        let stdout = strip_ansi_escapes::strip_str(String::from_utf8_lossy(&output.stdout));
        let stderr = strip_ansi_escapes::strip_str(String::from_utf8_lossy(&output.stderr));
        Output {
            success: output.status.success(),
            exit_code,
            stdout,
            stderr,
            stdout_raw: output.stdout,
            stderr_raw: output.stderr,
        }
    }

    pub(crate) fn from_pty(raw: Vec<u8>, status: portable_pty::ExitStatus) -> Self {
        let stdout = strip_ansi_escapes::strip_str(String::from_utf8_lossy(&raw));
        Output {
            success: status.success(),
            exit_code: status.exit_code(),
            stdout,
            stderr: String::new(),
            stdout_raw: raw,
            stderr_raw: Vec::new(),
        }
    }
}

impl fmt::Display for Output {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "success: {}", self.success)?;
        writeln!(f, "exit_code: {}", self.exit_code)?;
        writeln!(f, "----- stdout -----")?;
        write!(f, "{}", self.stdout)?;
        if !self.stdout.ends_with('\n') && !self.stdout.is_empty() {
            writeln!(f)?;
        }
        if !self.stderr.is_empty() {
            writeln!(f, "----- stderr -----")?;
            write!(f, "{}", self.stderr)?;
            if !self.stderr.ends_with('\n') {
                writeln!(f)?;
            }
        }
        Ok(())
    }
}
