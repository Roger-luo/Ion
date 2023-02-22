use crate::{
    config::Config,
    utils::{normalize_path, Julia},
};
use anyhow::Result;
use std::fmt::Display;
use std::path::{Path, PathBuf};
use std::process::Output;

pub struct TestOptions {
    pub color: bool,
    pub coverage: bool,
}

pub struct TestOutput {
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
}

impl TestOutput {
    pub fn from(output: &Output) -> Self {
        Self {
            stdout: output.stdout.clone(),
            stderr: output.stderr.clone(),
        }
    }
}

pub enum TestReport {
    Group {
        name: String,
        reports: Vec<TestReport>,
    },
    Test {
        name: String,
        passed: bool,
        output: TestOutput,
    },
}

impl Display for TestReport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Group { name, reports } => {
                writeln!(f, "{}", name)?;
                for report in reports {
                    writeln!(f, "  {}", report)?;
                }
                Ok(())
            }
            Self::Test {
                name,
                passed,
                output,
            } => {
                if *passed {
                    writeln!(f, "{}: passed", name)?;
                } else {
                    writeln!(f, "{}: failed", name)?;
                    writeln!(f, "  stdout:")?;
                    writeln!(f, "  {}", String::from_utf8_lossy(&output.stdout))?;
                    writeln!(f, "  stderr:")?;
                    writeln!(f, "  {}", String::from_utf8_lossy(&output.stderr))?;
                }
                Ok(())
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum JuliaTest {
    Test { name: String, entry: PathBuf },
    Group { name: String, tests: Vec<JuliaTest> },
}

impl JuliaTest {
    pub fn new(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let name = path
            .file_name()
            .ok_or_else(|| anyhow::anyhow!("invalid test path: {}", path.display()))?;
        let name = name
            .to_str()
            .ok_or_else(|| anyhow::anyhow!("invalid test path: {}", path.display()))?;
        let name = name.trim_end_matches(".jl");
        let name = name.trim_start_matches("test_").to_string();

        if path.is_file() {
            Ok(Self::Test {
                name,
                entry: normalize_path(path),
            })
        } else if path.is_dir() {
            let mut tests: Vec<JuliaTest> = Vec::new();
            for entry in path.read_dir()? {
                let entry = entry?;
                let path = entry.path();

                if Self::is_test_path(&path) {
                    tests.push(JuliaTest::new(path)?);
                }
            }
            Ok(Self::Group { name, tests })
        } else {
            anyhow::bail!("invalid test path: {}", path.display());
        }
    }

    fn is_test_path(path: impl AsRef<Path>) -> bool {
        let path = path.as_ref();
        path.file_name()
            .map(|s| s.to_str().map(|s| s.starts_with("test_")).unwrap_or(false))
            .unwrap_or(false)
    }

    pub fn run(&self, config: &Config, args: &Vec<String>) -> Result<TestReport> {
        match self {
            Self::Test { name, entry } => {
                let mut cmd = format!(
                    "using TestEnv; TestEnv.activate(); include(\"{entry}\")",
                    entry = entry.display()
                )
                .as_julia_command(config);

                for arg in args {
                    cmd.arg(arg);
                }
                let output = cmd.output()?;
                Ok(TestReport::Test {
                    name: name.to_string(),
                    passed: output.status.success(),
                    output: TestOutput::from(&output),
                })
            }
            Self::Group { name, tests } => {
                let mut reports: Vec<TestReport> = Vec::new();
                for test in tests {
                    reports.push(test.run(config, args)?);
                }
                Ok(TestReport::Group {
                    name: name.to_string(),
                    reports,
                })
            }
        } // match
    }
}

pub struct JuliaTestRunner {
    pub(crate) args: Vec<String>, // compiler args
    pub(crate) test: JuliaTest,
}

impl JuliaTestRunner {
    pub fn new(path: impl AsRef<Path>) -> Result<Self> {
        let test = JuliaTest::new(path)?;
        Ok(Self {
            args: vec!["--project", "--startup-file=no", "--history-file=no"]
                .iter()
                .map(|s| s.to_string())
                .collect::<Vec<_>>(),
            test,
        })
    }

    pub fn arg(&mut self, arg: impl AsRef<str>) {
        self.args.push(arg.as_ref().to_string());
    }

    pub fn run(&self, config: &Config) -> Result<TestReport> {
        self.test.run(config, &self.args)
    }
}
