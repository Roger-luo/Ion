use anyhow::Result;
use assert_cmd::assert::Assert;
use assert_cmd::cargo::cargo_bin;
use assert_cmd::prelude::OutputAssertExt;
use rexpect::process::wait::WaitStatus;
use rexpect::session::PtySession;
use std::env;
use std::ffi::OsStr;
use std::io;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

#[derive(Debug)]
pub struct Ion {
    cmd: Command,
}

impl Ion {
    pub fn new() -> Self {
        let program = cargo_bin("ion");
        let cmd = Command::new(program);
        Self { cmd }
    }

    pub fn arg<S: AsRef<OsStr>>(&mut self, arg: S) -> &mut Self {
        self.cmd.arg(arg);
        self
    }

    pub fn env<K, V>(&mut self, key: K, val: V) -> &mut Self
    where
        K: AsRef<OsStr>,
        V: AsRef<OsStr>,
    {
        self.cmd.env(key, val);
        self
    }

    pub fn output(&mut self) -> io::Result<Output> {
        self.cmd.output()
    }

    pub fn assert(&mut self) -> Assert {
        OutputAssertExt::assert(self)
    }

    pub fn spawn(&self, timeout_ms: Option<u64>) -> Result<PtySession> {
        let mut command = Command::new(cargo_bin("ion"));
        for arg in self.cmd.get_args() {
            command.arg(arg);
        }
        for (key, val) in self.cmd.get_envs() {
            if let Some(val) = val {
                command.env(key, val);
            }
        }

        if let Some(cwd) = self.cmd.get_current_dir() {
            command.current_dir(cwd);
        }
        Ok(rexpect::session::spawn_command(command, timeout_ms)?)
    }

    pub fn current_dir<P: AsRef<Path>>(&mut self, dir: P) -> &mut Self {
        self.cmd.current_dir(dir);
        self
    }

    pub fn packages_dir<P: AsRef<Path>>(&mut self, dir: P) -> &mut Self {
        self.current_dir(packages_dir().join(dir));
        self
    }

    pub fn scratch_dir<P: AsRef<Path>>(&mut self, dir: P) -> &mut Self {
        self.current_dir(scratch_dir().join(dir));
        self
    }

    pub fn packages(&mut self) -> &mut Self {
        self.current_dir(packages_dir());
        self
    }

    pub fn scratch(&mut self) -> &mut Self {
        self.current_dir(scratch_dir());
        self
    }
}

impl Default for Ion {
    fn default() -> Self {
        Self::new()
    }
}

impl<'c> OutputAssertExt for &'c mut Ion {
    fn assert(self) -> Assert {
        let output = match self.output() {
            Ok(output) => output,
            Err(err) => {
                panic!("Failed to spawn {self:?}: {err}");
            }
        };
        Assert::new(output).append_context("command", format!("{:?}", self.cmd))
    }
}

pub trait AssertSuccess {
    fn success(&mut self) -> Result<()>;
}

impl AssertSuccess for PtySession {
    fn success(&mut self) -> Result<()> {
        if let WaitStatus::Exited(_, 0) = self.process.wait()? {
            Ok(())
        } else {
            Err(anyhow::anyhow!("Process exited with non-zero status"))
        }
    }
}

pub fn packages_dir() -> PathBuf {
    let root = env::var("CARGO_MANIFEST_DIR").unwrap();
    PathBuf::from(root).join("tests").join("packages")
}

pub fn scratch_dir() -> PathBuf {
    packages_dir().join("scratch")
}
