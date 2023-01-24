use anyhow;

pub type CliResult = Result<(), CliError>;

#[derive(Debug)]
/// The CLI error is the error type used at Cargo's CLI-layer.
///
/// All errors from the lib side of Cargo will get wrapped with this error.
/// Other errors (such as command-line argument validation) will create this
/// directly.
pub struct CliError {
    /// The error to display. This can be `None` in rare cases to exit with a
    /// code without displaying a message. For example `cargo run -q` where
    /// the resulting process exits with a nonzero code (on Windows), or an
    /// external subcommand that exits nonzero (we assume it printed its own
    /// message).
    pub error: Option<anyhow::Error>,
    /// The process exit code.
    pub exit_code: i32,
}

impl CliError {
    pub fn new(error: anyhow::Error, code: i32) -> CliError {
        CliError {
            error: Some(error),
            exit_code: code,
        }
    }

    pub fn code(code: i32) -> CliError {
        CliError {
            error: None,
            exit_code: code,
        }
    }
}

impl From<anyhow::Error> for CliError {
    fn from(err: anyhow::Error) -> CliError {
        CliError::new(err, 101)
    }
}

impl From<clap::Error> for CliError {
    fn from(err: clap::Error) -> CliError {
        let code = i32::from(err.use_stderr());
        CliError::new(err.into(), code)
    }
}

impl From<std::io::Error> for CliError {
    fn from(err: std::io::Error) -> CliError {
        CliError::new(err.into(), 1)
    }
}

impl From<node_semver::SemverError> for CliError {
    fn from(err: node_semver::SemverError) -> CliError {
        CliError::new(err.into(), 1)
    }
}

impl From<url::ParseError> for CliError {
    fn from(err: url::ParseError) -> CliError {
        CliError::new(err.into(), 1)
    }
}

impl From<octocrab::Error> for CliError {
    fn from(err: octocrab::Error) -> CliError {
        CliError::new(err.into(), 1)
    }
}

impl From<keyring::Error> for CliError {
    fn from(err: keyring::Error) -> CliError {
        CliError::new(err.into(), 1)
    }
}
