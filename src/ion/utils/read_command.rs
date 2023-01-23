use anyhow::{format_err, Error};
use std::process::{Command, Output};

use super::JuliaCommand;

trait CommandMarker {
    fn output(&mut self) -> Result<Output, std::io::Error>;
}
impl CommandMarker for Command {
    fn output(&mut self) -> Result<Output, std::io::Error> {
        self.output()
    }
}
impl CommandMarker for JuliaCommand {
    fn output(&mut self) -> Result<Output, std::io::Error> {
        self.output()
    }
}

pub trait ReadCommand {
    fn read_command(&mut self) -> Result<String, Error>;
}

impl<T: CommandMarker> ReadCommand for T {
    fn read_command(&mut self) -> Result<String, Error> {
        let output = self.output()?;
        if output.status.success() {
            let raw = String::from_utf8(output.stdout)?.trim().to_string();
            Ok(raw)
        } else {
            Err(format_err!("Failed to read command"))
        }
    }
}
