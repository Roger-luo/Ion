//! Library for building Ion binary skills.
//!
//! Provides the standard self-management infrastructure that all Ion binary skills
//! are expected to implement: `self skill`, `self info`, `self check`, `self update`.
//!
//! # Quick start
//!
//! ```rust,ignore
//! use ionlib::self_update::SelfManager;
//!
//! let manager = SelfManager::new(
//!     "owner/my-tool",
//!     "my-tool",
//!     "v",
//!     env!("CARGO_PKG_VERSION"),
//!     env!("TARGET"),
//! );
//! ```
//!
//! See [`self_update::SelfManager`] for the full API.

pub mod error;
pub mod release;
pub mod self_update;

pub use error::{Error, Result};
