pub mod error;
pub mod skill;
pub mod source;

pub use error::Error;

pub type Result<T> = std::result::Result<T, Error>;
