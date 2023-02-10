use anyhow::{format_err, Result};
use std::fmt::{Debug, Display};
use std::process::{Command, Output};

use crate::config::Config;

use super::julia_version;

pub struct JuliaCommand {
    cmd: Command,
    script: String,
}

impl Debug for JuliaCommand {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.cmd)?;
        write!(f, "{}", self.script)
    }
}

impl JuliaCommand {
    pub fn arg(&mut self, arg: &str) -> &mut Self {
        self.cmd.arg(arg);
        self
    }

    pub fn project(&mut self, project: &str) -> &mut Self {
        self.cmd.arg(format!("--project={project}"));
        self
    }

    pub fn compile(&mut self, option: &str) -> &mut Self {
        self.cmd.arg(format!("--compile={option}"));
        self
    }

    pub fn no_startup_file(&mut self) -> &mut Self {
        self.cmd.arg("--startup-file=no");
        self
    }

    pub fn color(&mut self) -> &mut Self {
        self.cmd.arg("--color=yes");
        self
    }

    pub fn output(&mut self) -> Result<Output, std::io::Error> {
        self.cmd.arg(format!("-e {}", self.script)).output()
    }

    pub fn status(&mut self) -> Result<std::process::ExitStatus, std::io::Error> {
        self.cmd.arg(format!("-e {}", self.script)).status()
    }
}

pub trait Julia {
    fn as_julia_command(&self, config: &Config) -> JuliaCommand;

    fn julia_exec_cmd(&self, config: &Config, project: impl AsRef<str>) -> JuliaCommand {
        let mut cmd = self.as_julia_command(config);
        cmd.project(project.as_ref())
            .no_startup_file()
            .color()
            .compile("min");
        cmd
    }

    fn julia_exec_project_quiet(&self, config: &Config, project: &str) -> Result<()> {
        let p = self.julia_exec_cmd(config, project).output()?;

        if p.status.success() {
            Ok(())
        } else {
            Err(format_err!("Julia command failed"))
        }
    }

    fn julia_exec_project(&self, config: &Config, project: &str) -> Result<()> {
        let p = self.julia_exec_cmd(config, project).status()?;

        if p.success() {
            Ok(())
        } else {
            Err(format_err!("Julia command failed"))
        }
    }

    fn julia_exec(&self, config: &Config, global: bool) -> Result<()> {
        let mut cmd = self.as_julia_command(config);
        if !global {
            cmd.project("@.");
        }
        cmd.no_startup_file().color().compile("min");
        let p = cmd.status()?;
        if p.success() {
            Ok(())
        } else {
            Err(format_err!("Julia command failed"))
        }
    }

    fn julia_exec_quiet(&self, config: &Config) -> Result<()> {
        self.julia_exec_project_quiet(config, "@.")
    }
}

impl<T: Display> Julia for T {
    fn as_julia_command(&self, config: &Config) -> JuliaCommand {
        let script = self.to_string();
        let cmd = Command::new(config.julia().exe);
        JuliaCommand { cmd, script }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use std::ffi::OsStr;

    #[test]
    fn test_julia_command() {
        let config = Config::default();
        let cmd = "using Pkg; Pkg.add(\"Foo\")".as_julia_command(&config);
        assert_eq!(cmd.cmd.get_program(), "julia");
        assert!(cmd.cmd.get_args().next().is_none());
        assert_eq!(cmd.script, "using Pkg; Pkg.add(\"Foo\")");

        let mut cmd = "using Pkg; Pkg.add(\"Foo\")".as_julia_command(&config);
        cmd.project("Foo").arg("Bar").arg("Baz");
        let args: Vec<&OsStr> = cmd.cmd.get_args().collect();
        assert_eq!(args.len(), 3);
        assert_eq!(args[0], "--project=Foo");
        assert_eq!(args[1], "Bar");
        assert_eq!(args[2], "Baz");
    }
}

pub fn assert_julia_version(config: &Config, version_spec: impl AsRef<str>) -> Result<()> {
    let range = node_semver::Range::parse(version_spec.as_ref()).expect("Invalid version spec");
    let version = julia_version(config)?;
    range.satisfies(&version).then(|| ()).ok_or_else(|| {
        format_err!(
            "Invalid Julia version: Julia version {version} does not satisfy version range {range}",
        )
    })
}
