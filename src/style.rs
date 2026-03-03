use std::io::IsTerminal;

use crossterm::style::Stylize;
use ion_skill::config::GlobalConfig;

/// Determines whether to use color in output, respecting config and terminal.
pub fn use_color(config: &GlobalConfig) -> bool {
    match config.ui.color {
        Some(true) => true,
        Some(false) => false,
        None => std::io::stdout().is_terminal(),
    }
}

/// Styled output helper. Call methods to get colored or plain strings.
pub struct Paint {
    pub color: bool,
}

impl Paint {
    pub fn new(config: &GlobalConfig) -> Self {
        Self {
            color: use_color(config),
        }
    }

    /// Bold white — for headings and skill names.
    pub fn bold(&self, text: &str) -> String {
        if self.color {
            text.white().bold().to_string()
        } else {
            text.to_string()
        }
    }

    /// Green — for success messages.
    pub fn success(&self, text: &str) -> String {
        if self.color {
            text.green().to_string()
        } else {
            text.to_string()
        }
    }

    /// Cyan — for paths and targets.
    pub fn info(&self, text: &str) -> String {
        if self.color {
            text.cyan().to_string()
        } else {
            text.to_string()
        }
    }

    /// Yellow — for warnings and hints.
    pub fn warn(&self, text: &str) -> String {
        if self.color {
            text.yellow().to_string()
        } else {
            text.to_string()
        }
    }

    /// Dim/grey — for secondary info.
    pub fn dim(&self, text: &str) -> String {
        if self.color {
            text.grey().to_string()
        } else {
            text.to_string()
        }
    }
}
