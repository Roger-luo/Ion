use cargo::CliResult;
use url::Url;
use std::fmt::Display;
use std::path::PathBuf;
use std::process;
use clap::parser::ArgMatches;
use node_semver::Range;

pub trait JuliaCmd {
    fn julia_cmd(&self, compile_min : bool) -> CliResult;
    fn as_julia_script(&self) -> CliResult {
        self.julia_cmd(true)
    }
    fn as_julia_script_compile(&self) -> CliResult {
        self.julia_cmd(false)
    }
}

impl<T: Display> JuliaCmd for T {
    fn julia_cmd(&self, compile_min : bool) -> CliResult {
        let mut cmd = process::Command::new("julia");
        let mut cmd = cmd.arg("--color=yes")
            .arg("--startup-file=no")
            .arg("--project");

        if compile_min {
            cmd = cmd.arg("--compile=min");
        }

        cmd = cmd.arg(format!("-e {}", self));
        let output = cmd.output().expect("fail to call Julia");
        print!("{}", String::from_utf8(output.stdout).unwrap().as_str());
        eprint!("{}", String::from_utf8(output.stderr).unwrap().as_str());
        Ok(())
    }
}

#[derive(Debug)]
pub struct PackageSpec {
    name: Option<String>,
    url: Option<String>,
    path: Option<String>,
    subdir: Option<String>,
    rev: Option<String>,
    version: Option<String>,
}

impl PackageSpec {
    pub fn new(expr: &String) -> Self {
        let mut name : String = expr.to_owned();
        let mut version : Option<String> = None;
        let mut rev : Option<String> = None;

        name = if expr.contains("@") {
            let parts = expr.split('@').collect::<Vec<_>>();
            assert!(parts.len() == 2, "Invalid package name: {}", expr);
            let version_str = expr.split('@').last().unwrap();
            assert!(Range::parse(version_str).is_ok(), "Invalid version: {}", version_str);
            version = Some(version_str.to_string());
            parts[0].to_string()
        } else {
            name
        };

        name = if name.contains('#') {
            let parts = name.split('#').collect::<Vec<_>>();
            println!("{:?}", parts);
            assert!(parts.len() == 2, "Invalid package name: {}", expr);
            rev = Some(parts[1].to_string());
            parts[0].to_string()
        } else {
            name
        };

        if PathBuf::from(name.clone()).is_dir() {
            return Self {
                name: None,
                url: None,
                path: Some(name),
                subdir: None,
                rev,
                version
            }
        }

        if Url::parse(name.as_str()).is_ok() {
            return Self {
                name: None,
                url: Some(name),
                path: None,
                subdir: None,
                rev,
                version
            }
        }

        return Self {
            name: Some(name.to_owned()),
            url: None,
            path: None,
            subdir: None,
            rev,
            version
        }
    }

    pub fn from_path(path: &PathBuf) -> PackageSpec {
        Self {
            name: None,
            url: None,
            path: Some(path.to_str().unwrap().to_string()),
            subdir: None,
            rev: None,
            version: None
        }
    }
}

impl Display for PackageSpec {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut fields = Vec::<String>::new();
        if let Some(name) = &self.name {
            fields.push(format!("name=\"{}\"", name));
        }
        if let Some(url) = &self.url {
            fields.push(format!("url=\"{}\"", url));
        }
        if let Some(path) = &self.path {
            fields.push(format!("path=\"{}\"", path));
        }
        if let Some(subdir) = &self.subdir {
            fields.push(format!("subdir=\"{}\"", subdir));
        }
        if let Some(rev) = &self.rev {
            fields.push(format!("rev=\"{}\"", rev));
        }
        if let Some(version) = &self.version {
            fields.push(format!("version=\"{}\"", version));
        }
        write!(f, "PackageSpec({})", fields.join(", "))
    }
}

pub fn package_spec_list(matches: &ArgMatches) -> String {
    let packages = matches
        .get_many::<String>("PACKAGE")
        .into_iter()
        .flatten();

    packages.map(|p| PackageSpec::new(p))
        .map(|p| format!("{}", p))
        .collect::<Vec<_>>()
        .join(",")
}
