pub mod components;
pub mod badge;
pub mod file;
pub mod blueprint;
pub mod template;
pub mod context;
pub mod utils;

pub use blueprint::{Blueprint, RenderResult};
pub use template::Template;
pub use context::*;
pub use file::TemplateFile;
pub use badge::{Badge, Badgeable};
pub use utils::*;
