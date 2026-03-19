pub mod binary;
pub mod config;
pub mod error;
pub mod git;
pub mod gitignore;
pub mod installer;
pub mod lockfile;
pub mod manifest;
pub mod manifest_writer;
pub mod migrate;
pub mod registry;
pub mod search;
pub mod skill;
pub mod source;
pub mod update;
pub mod validate;

// Re-export ionem's self_update for backward compatibility.
// New code should depend on `ionem` directly.
pub use ionem::self_update;

pub use error::Error;

pub type Result<T> = std::result::Result<T, Error>;

/// Load a TOML file and deserialize it. Returns `T::default()` if the file doesn't exist.
pub fn load_toml_or_default<T: serde::de::DeserializeOwned + Default>(
    path: &std::path::Path,
) -> Result<T> {
    match std::fs::read_to_string(path) {
        Ok(content) => toml::from_str(&content).map_err(Error::TomlParse),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(T::default()),
        Err(e) => Err(Error::Io(e)),
    }
}
