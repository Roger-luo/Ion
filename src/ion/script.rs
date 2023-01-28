use crate::{utils::Julia, JuliaProject, Manifest, PackageSpec};
use anyhow::{format_err, Result};
use node_semver::{Range, Version};
use serde_derive::{Deserialize, Serialize};
use std::process::Command;
use std::{collections::BTreeMap, path::PathBuf};
use toml;

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum DepdencyList {
    List(Vec<String>),
    Dict(BTreeMap<String, String>),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ScriptDeps {
    pub registry: Option<String>,
    pub deps: DepdencyList,
}

impl ScriptDeps {
    pub fn from_script(script: impl AsRef<str>) -> Result<Option<Self>> {
        let script = script.as_ref();
        let lines = script.lines();
        let mut toml_str = String::new();
        let mut within_toml = false;
        for line in lines {
            if line.starts_with("#=ion") {
                within_toml = true;
            } else if within_toml && line.starts_with("=#") {
                break;
            } else if within_toml {
                toml_str.push_str(line);
                toml_str.push('\n');
            }
        }
        log::debug!("toml_str:\n {}", toml_str);
        if within_toml {
            let deps: ScriptDeps = toml::from_str(&toml_str)?;
            Ok(Some(deps))
        } else {
            Ok(None)
        }
    }
}

pub struct Script {
    pub path: String,
    pub deps: Option<ScriptDeps>,
    pub env: Option<String>,
}

impl Script {
    pub fn from_path(path: impl AsRef<str>) -> Result<Self> {
        log::debug!("loading script: {}", path.as_ref());
        let path = path.as_ref();
        let script = std::fs::read_to_string(path)?;
        log::debug!("script: {}", script);
        let deps = ScriptDeps::from_script(&script)?;
        log::debug!("deps: {:?}", deps);
        let env = if let Some(deps) = &deps {
            Some(create_env(path, deps)?)
        } else {
            None
        };
        Ok(Script {
            path: path.to_owned(),
            deps,
            env,
        })
    }

    pub fn cmd(&self) -> Command {
        let mut cmd = Command::new("julia");

        if let Some(env) = &self.env {
            cmd.arg(format!("--project={env}"));
        } else {
            cmd.arg("--project");
        }
        cmd
    }
}

fn create_env(path: impl AsRef<str>, deps: &ScriptDeps) -> Result<String> {
    let sha = crc32fast::hash(path.as_ref().as_bytes());
    let env = std::env::current_exe()?;
    let env = env
        .parent()
        .expect("cannot find parent of executable")
        .join("env")
        .join(format!("env-{sha}"));

    log::debug!("env: {}", env.display());
    if env.is_dir() {
        if check_deps(env.to_owned(), deps)? {
            return Ok(env.display().to_string());
        } else {
            std::fs::remove_dir_all(&env)?;
        }
    }

    std::fs::create_dir_all(&env)?;
    match deps.deps {
        DepdencyList::List(ref deps) => {
            let script = deps
                .iter()
                .map(|s| format!(r#""{s}""#))
                .collect::<Vec<_>>()
                .join(", ");

            log::debug!("script: {}", script);
            format!(
                "using Pkg; Pkg.activate(\"{path}\"); Pkg.add([{script}])",
                path = env.display()
            )
            .julia_exec()?;
        }
        DepdencyList::Dict(ref deps) => {
            let mut pkgs = Vec::<PackageSpec>::new();
            for (name, version) in deps {
                pkgs.push(PackageSpec {
                    name: Some(name.to_string()),
                    version: Some(version.to_string()),
                    url: None,
                    path: None,
                    subdir: None,
                    rev: None,
                })
            }

            let script = pkgs
                .into_iter()
                .map(|s| format!("{s}"))
                .collect::<Vec<_>>()
                .join(", ");
            log::debug!("script: {}", script);
            format!(
                "using Pkg; Pkg.activate(\"{path}\"); Pkg.add([{script}])",
                path = env.display()
            )
            .julia_exec()?;
        }
    }

    Ok(env.display().to_string())
}

fn check_deps(env: PathBuf, deps: &ScriptDeps) -> Result<bool> {
    if !env.join("Project.toml").is_file() {
        return Ok(false);
    }

    let project = JuliaProject::from_file(env.join("Project.toml"))?;
    match deps.deps {
        DepdencyList::List(ref deps) => {
            for name in deps {
                if !project.deps.contains_key(name) {
                    return Ok(false);
                }
            }
            Ok(true)
        }
        DepdencyList::Dict(ref deps) => {
            let manifest = Manifest::from_file(env.join("Manifest.toml"))?;
            for (name, version) in deps {
                if project.deps.contains_key(name) {
                    let range = Range::parse(version)?;
                    let pkgs = manifest
                        .deps
                        .get(name)
                        .ok_or_else(|| format_err!("Package {} not found in manifest", name))?;
                    for pkg in pkgs {
                        if let Some(input) = &pkg.version {
                            if !range.satisfies(&Version::parse(input)?) {
                                return Ok(false);
                            }
                        }
                    }
                } else {
                    return Ok(false);
                }
            }
            Ok(true)
        }
    }
}
