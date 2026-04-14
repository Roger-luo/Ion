use std::path::PathBuf;
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

    /// `expect_not()` found a pattern that should NOT have appeared.
    #[error("expect_not found unexpected pattern {pattern:?} in output:\n{buffer}")]
    UnexpectedPattern { pattern: String, buffer: String },

    /// Invalid regex passed to `expect_regex()`.
    #[error("invalid regex: {0}")]
    Regex(#[from] regex::Error),

    /// `spawn()` requires `Terminal::Pty`; it cannot be used with `Terminal::Piped`.
    #[error(
        "spawn() requires Terminal::Pty; use Terminal::pty(cols, rows) instead of Terminal::Piped"
    )]
    SpawnRequiresPty,

    /// Template directory does not exist.
    #[error("template not found: {}", path.display())]
    TemplateNotFound { path: PathBuf },

    /// Failed to parse template.toml manifest.
    #[error("failed to parse {}: {source}", path.display())]
    ManifestParse {
        path: PathBuf,
        source: toml::de::Error,
    },

    /// Required template variables were not provided.
    #[error("missing required template variable(s): {}", names.join(", "))]
    MissingVariable { names: Vec<String> },

    /// An unknown variable was set via `.var()`.
    #[error("unknown template variable: {name}")]
    UnknownVariable { name: String },

    /// Minijinja failed to render a template file.
    #[error("template render error in {file}: {source}")]
    TemplateRender {
        file: String,
        source: minijinja::Error,
    },

    /// A symlink target does not exist after rendering.
    #[error("symlink target does not exist: {}", path.display())]
    SymlinkTarget { path: PathBuf },

    /// Post-build project setup failed.
    #[error("project setup failed during {step}: {source}")]
    ProjectSetup {
        step: String,
        #[source]
        source: ProjectSetupError,
    },
}

fn format_duration(d: &Duration) -> String {
    let secs = d.as_secs_f64();
    if secs < 1.0 {
        format!("{:.0}ms", secs * 1000.0)
    } else {
        format!("{secs:.1}s")
    }
}

/// Human-readable error details for a project setup step.
#[derive(Debug)]
pub struct ProjectSetupError(String);

impl ProjectSetupError {
    pub fn new(message: impl Into<String>) -> Self {
        Self(message.into())
    }
}

impl std::fmt::Display for ProjectSetupError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for ProjectSetupError {}
