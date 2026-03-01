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
}
