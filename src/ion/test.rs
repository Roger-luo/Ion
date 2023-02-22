use crate::{
    config::Config,
    utils::{normalize_path, Julia},
};
use anyhow::Result;
use tokio::runtime::Builder;
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
                    write!(f, "  {}", report)?;
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
                    let stdout = String::from_utf8_lossy(&output.stdout);
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    let stdout = stdout.trim();
                    let stderr = stderr.trim();
                    let stdout = stdout.split('\n').collect::<Vec<_>>().iter()
                        .map(|s| format!("  |{}", s))
                        .collect::<Vec<_>>()
                        .join("\n");

                    writeln!(f, "================= Output ================")?;
                    writeln!(f, "{}", stdout)?;
                    if !stderr.is_empty() {
                        writeln!(f, "================= Error ================")?;
                        let stderr = stderr.split('\n').collect::<Vec<_>>().iter()
                        .map(|s| format!("  |{}", s))
                        .collect::<Vec<_>>()
                        .join("\n");
                        writeln!(f, "{}", stderr)?;
                    }
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
                log::debug!("running test: {:?}", cmd);

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
                let runtime = Builder::new_multi_thread()
                    .worker_threads(2)
                    .build()?;

                let mut handles = Vec::with_capacity(tests.len());
                for test in tests {
                    let config = config.clone();
                    let args = args.clone();
                    let test = test.clone();
                    handles.push(runtime.spawn(async move {
                        test.run(&config, &args)
                    }));
                }

                let mut reports = Vec::with_capacity(tests.len());
                for handle in handles {
                    reports.push(runtime.block_on(handle)??);
                }

                Ok(TestReport::Group {
                    name: name.to_string(),
                    reports,
                })
            }
        } // match
    }
}

#[derive(Debug, Clone)]
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
