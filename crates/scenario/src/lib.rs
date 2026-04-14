//! # scenario
//!
//! Define and test CLI behavior scenarios under controlled terminal conditions.
//!
//! `scenario` provides infrastructure for running CLI applications across
//! different terminal environments — with or without a TTY, at various
//! terminal widths, with or without color support.
//!
//! ## Quick Start
//!
//! ```no_run
//! use scenario::{Scenario, Terminal};
//!
//! // Run a command with piped stdio (no TTY)
//! let output = Scenario::new("echo")
//!     .arg("hello")
//!     .run()
//!     .unwrap();
//! assert!(output.stdout().contains("hello"));
//!
//! // Run in a real PTY with specific dimensions
//! let output = Scenario::new("my-cli")
//!     .args(["--help"])
//!     .terminal(Terminal::pty(80, 24))
//!     .run()
//!     .unwrap();
//!
//! // Interactive session
//! let mut session = Scenario::new("my-cli")
//!     .args(["init"])
//!     .terminal(Terminal::pty(80, 24))
//!     .spawn()
//!     .unwrap();
//! session.expect("Choose:").unwrap();
//! session.send_line("default").unwrap();
//! let output = session.wait().unwrap();
//! ```

mod error;
mod key;
pub mod manifest;
mod output;
mod project;
mod scenario;
pub mod screen;
mod session;

pub use error::Error;
pub use key::Key;
pub use output::Output;
pub use project::{Project, ProjectBuilder};
pub use scenario::{Scenario, SessionConfig, Terminal};
pub use screen::ScreenBuffer;
pub use session::Session;
