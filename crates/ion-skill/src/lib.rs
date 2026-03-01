pub mod error;
pub mod git;
pub mod installer;
pub mod lockfile;
pub mod manifest;
pub mod manifest_writer;
pub mod skill;
pub mod source;

pub use error::Error;

pub type Result<T> = std::result::Result<T, Error>;
