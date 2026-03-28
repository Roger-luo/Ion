use std::time::Duration;

/// Errors that can occur when running scenarios.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// I/O error from subprocess or PTY operations.
    #[error("{0}")]
    Io(#[from] std::io::Error),

    /// Error from the PTY system (wraps `anyhow::Error` from `portable-pty`).
    #[error("pty: {0}")]
    Pty(String),

    /// The process did not exit within the configured timeout.
    #[error("process timed out after {}", format_duration(.0))]
    Timeout(Duration),

    /// `expect()` did not find the pattern within the timeout.
    #[error("expect timed out after {} waiting for {pattern:?}\nbuffer content:\n{buffer}", format_duration(.timeout))]
    ExpectTimeout {
        pattern: String,
        timeout: Duration,
        buffer: String,
    },

    /// Invalid regex passed to `expect_regex()`.
    #[error("invalid regex: {0}")]
    Regex(#[from] regex::Error),

    /// `spawn()` requires `Terminal::Pty`; it cannot be used with `Terminal::Piped`.
    #[error(
        "spawn() requires Terminal::Pty; use Terminal::pty(cols, rows) instead of Terminal::Piped"
    )]
    SpawnRequiresPty,
}

fn format_duration(d: &Duration) -> String {
    let secs = d.as_secs_f64();
    if secs < 1.0 {
        format!("{:.0}ms", secs * 1000.0)
    } else {
        format!("{secs:.1}s")
    }
}
