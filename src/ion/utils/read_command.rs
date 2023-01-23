use anyhow::Error;
use std::process::Command;

use super::JuliaCommand;

pub trait ReadCommand {
    fn read_command(&mut self) -> Result<String, Error>;
}

impl ReadCommand for Command {
    fn read_command(&mut self) -> Result<String, Error> {
        let output = self.output()?;
        let raw = String::from_utf8(output.stdout)?.trim().to_string();
        Ok(raw)
    }
}

impl ReadCommand for JuliaCommand {
    fn read_command(&mut self) -> Result<String, Error> {
        let output = self.output()?;
        let raw = String::from_utf8(output.stdout)?.trim().to_string();
        Ok(raw)
    }
}
