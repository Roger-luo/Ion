use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("TOML parse error: {0}")]
    TomlParse(#[from] toml::de::Error),

    #[error("TOML edit error: {0}")]
    TomlEdit(#[from] toml_edit::TomlError),

    #[error("YAML parse error: {0}")]
    YamlParse(#[from] serde_yaml::Error),

    #[error("Invalid skill: {0}")]
    InvalidSkill(String),

    #[error("Source error: {0}")]
    Source(String),

    #[error("Git error: {0}")]
    Git(String),

    #[error("Manifest error: {0}")]
    Manifest(String),

    #[error("Search error: {0}")]
    Search(String),

    #[error("HTTP error: {0}")]
    Http(String),

    #[error("{0}")]
    Other(String),

    #[error(
        "Validation failed: {error_count} error(s), {warning_count} warning(s), {info_count} info finding(s)"
    )]
    ValidationFailed {
        report: crate::validate::ValidationReport,
        error_count: usize,
        warning_count: usize,
        info_count: usize,
    },

    #[error(
        "Validation warnings require confirmation: {warning_count} warning(s), {info_count} info finding(s)"
    )]
    ValidationWarning {
        report: crate::validate::ValidationReport,
        warning_count: usize,
        info_count: usize,
    },
}

impl Error {
    /// Create a ValidationFailed error from a report.
    pub fn validation_failed(report: crate::validate::ValidationReport) -> Self {
        Self::ValidationFailed {
            error_count: report.error_count,
            warning_count: report.warning_count,
            info_count: report.info_count,
            report,
        }
    }

    /// Create a ValidationWarning error from a report.
    pub fn validation_warning(report: crate::validate::ValidationReport) -> Self {
        Self::ValidationWarning {
            warning_count: report.warning_count,
            info_count: report.info_count,
            report,
        }
    }
}
