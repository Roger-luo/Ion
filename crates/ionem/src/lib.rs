//! Library for building Ion binary skills.
//!
//! Provides the standard self-management infrastructure that all Ion binary skills
//! are expected to implement: `self skill`, `self info`, `self check`, `self update`.
//!
//! # Quick start
//!
//! In `build.rs`:
//!
//! ```rust,ignore
//! fn main() {
//!     ionem::build::emit_target();
//!     ionem::build::copy_skill_md();  // or render_skill_md_vars / render_skill_md
//! }
//! ```
//!
//! In `src/main.rs`:
//!
//! ```rust,ignore
//! use ionem::self_update::SelfManager;
//!
//! const SKILL_MD: &str = include_str!(concat!(env!("OUT_DIR"), "/SKILL.md"));
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
//! See [`build`] for SKILL.md preparation and [`self_update::SelfManager`] for the runtime API.

pub mod build;
pub mod error;
pub mod release;
pub mod self_update;

pub use error::{Error, Result};
