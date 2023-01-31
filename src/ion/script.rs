use crate::utils::normalize_path;
use crate::{utils::Julia, JuliaProject, Manifest, PackageSpec};
use anyhow::{format_err, Result};
use node_semver::Range;
use serde_derive::{Deserialize, Serialize};
use std::path::Path;
use std::process::Command;
use std::{collections::BTreeMap, path::PathBuf};
use toml;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum DepdencyInfo {
    /// short-hand version of Version {version } without uuid
    /// this means the package is from the default registry
    Short(String),
    /// dependency specified by version, and optionally uuid to disambiguate
    /// same name packages from different registries
    Version {
        version: String,
        uuid: Option<String>,
    },
    /// remote package does not rely on a version
    /// nor a registry, so we only need the url
    /// and optionally rev and subdir
    RemotePackage {
        url: String,
        rev: Option<String>,
        subdir: Option<String>,
    },
    /// local package does not rely on a version
    /// nor a registry, so we only need the path
    /// and optionally subdir
    LocalPackage {
        path: String,
        subdir: Option<String>,
    },
}

impl DepdencyInfo {
    pub fn normalize(&self, root: &Path) -> Self {
        if let DepdencyInfo::LocalPackage { path, subdir } = &self {
            let path = PathBuf::from(path);
            let path = root.join(path);
            let path = normalize_path(path.as_path());
            let path = path.to_str().unwrap();
            return DepdencyInfo::LocalPackage {
                path: path.to_string(),
                subdir: subdir.clone(),
            };
        } else {
            self.clone()
        }
    }

    pub fn to_package_spec(&self, name: &str) -> PackageSpec {
        match self {
            DepdencyInfo::Short(range) => PackageSpec {
                name: Some(name.to_string()),
                uuid: None,
                url: None,
                path: None,
                subdir: None,
                rev: None,
                version: Some(range.to_owned()),
                tree_hash: None,
                pinned: None,
            },
            DepdencyInfo::Version { version, uuid } => PackageSpec {
                name: Some(name.to_string()),
                url: None,
                path: None,
                subdir: None,
                rev: None,
                version: Some(version.to_owned()),
                tree_hash: None,
                pinned: None,
                uuid: uuid.clone(),
            },
            DepdencyInfo::RemotePackage { url, rev, subdir } => PackageSpec {
                name: Some(name.to_string()),
                url: Some(url.to_string()),
                rev: rev.clone(),
                subdir: subdir.clone(),
                path: None,
                version: None,
                uuid: None,
                tree_hash: None,
                pinned: None,
            },
            DepdencyInfo::LocalPackage { path, subdir } => PackageSpec {
                name: Some(name.to_string()),
                path: Some(path.to_owned()),
                subdir: subdir.clone(),
                url: None,
                rev: None,
                version: None,
                uuid: None,
                tree_hash: None,
                pinned: None,
            },
        }
    }
}

pub type ScriptDeps = BTreeMap<String, DepdencyInfo>;

pub struct Script {
    pub path: String,
    pub deps: Option<ScriptDeps>,
    pub env: Option<String>,
}

impl Script {
    pub fn from_path(path: impl AsRef<str>, verbose: bool) -> Result<Self> {
        log::debug!("loading script: {}", path.as_ref());
        let path = path.as_ref();
        let script = std::fs::read_to_string(path)?;
        log::debug!("script: {}", script);
        let deps = from_script(&script)?;
        log::debug!("deps: {:?}", deps);
        let env = if let Some(deps) = &deps {
            Some(create_env(path, deps, verbose)?)
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

fn from_script(script: impl AsRef<str>) -> Result<Option<ScriptDeps>> {
    log::debug!("parsing script: {}", script.as_ref());

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

fn create_env(path: impl AsRef<str>, deps: &ScriptDeps, verbose: bool) -> Result<String> {
    log::debug!("creating env for: {}", path.as_ref());

    let sha = crc32fast::hash(path.as_ref().as_bytes());
    let env = std::env::current_exe()?;
    let env = env
        .parent()
        .expect("cannot find parent of executable")
        .join("env")
        .join(format!("env-{sha}"));
    let project = env.to_str().expect("invalid path").to_string();

    log::debug!("env: {}", env.display());
    if env.is_dir() {
        if check_deps(&env, deps)? {
            log::debug!("deps are up to date");
            return Ok(project);
        } else {
            std::fs::remove_dir_all(&env)?;
        }
    }

    std::fs::create_dir_all(&env)?;

    let root = PathBuf::from(path.as_ref());
    let script = deps
        .iter()
        .map(|(name, info)| {
            format!("{}", {
                info.normalize(root.as_path()).to_package_spec(name)
            })
        })
        .collect::<Vec<_>>()
        .join(", ");

    log::debug!("script: {}", script);
    let mut cmd = format!("using Pkg; Pkg.add([{script}])",).julia_exec_cmd(&project);

    let p = if verbose {
        cmd.status()?
    } else {
        cmd.output()?.status
    };

    if !p.success() {
        return Err(format_err!("failed to create environment"));
    }
    Ok(project)
}

fn check_deps(env: &Path, deps: &ScriptDeps) -> Result<bool> {
    log::debug!("checking deps: {:?}", deps);

    if !env.join("Project.toml").is_file() {
        return Ok(false);
    }

    let project = JuliaProject::from_file(env.join("Project.toml"))?;
    let manifest = Manifest::from_file(env.join("Manifest.toml"))?;

    for (name, info) in deps {
        if project.deps.contains_key(name) {
            if let DepdencyInfo::Version {
                uuid: Some(pkg_uuid),
                ..
            } = info
            {
                let deps_uuid = project.deps.get(name).unwrap();
                if deps_uuid != pkg_uuid {
                    return Ok(false);
                }
            }
        } else {
            return Ok(false);
        }

        if !manifest.contains(name, info)? {
            return Ok(false);
        }
    }
    Ok(true)
}

trait Contains {
    fn contains(&self, name: impl AsRef<str>, info: &DepdencyInfo) -> Result<bool>;
    fn contains_version(
        &self,
        _name: impl AsRef<str>,
        _range: &Range,
        _uuid: &Option<String>,
    ) -> bool {
        false
    }
    fn contains_local(
        &self,
        _name: impl AsRef<str>,
        _path: &str,
        _subdir: &Option<String>,
    ) -> bool {
        false
    }
    fn contains_remote(
        &self,
        _name: impl AsRef<str>,
        _url: &str,
        _rev: &Option<String>,
        _subdir: &Option<String>,
    ) -> bool {
        false
    }
}

impl Contains for JuliaProject {
    fn contains(&self, name: impl AsRef<str>, info: &DepdencyInfo) -> Result<bool> {
        if !self.deps.contains_key(name.as_ref()) {
            return Ok(false);
        }
        match info {
            DepdencyInfo::Version { uuid, .. } => {
                if let Some(pkg_uuid) = uuid {
                    let deps_uuid = self.deps.get(name.as_ref()).unwrap();
                    Ok(deps_uuid != pkg_uuid)
                } else {
                    Ok(true)
                }
            }
            _ => Ok(true),
        }
    }
}

impl Contains for Manifest {
    fn contains(&self, name: impl AsRef<str>, info: &DepdencyInfo) -> Result<bool> {
        if !self.deps.contains_key(name.as_ref()) {
            return Ok(false);
        }

        Ok(match info {
            DepdencyInfo::Short(range) => self.contains_version(name, &Range::parse(range)?, &None),
            DepdencyInfo::Version { version, uuid } => {
                self.contains_version(name, &Range::parse(version)?, uuid)
            }
            DepdencyInfo::LocalPackage { path, subdir } => self.contains_local(name, path, subdir),
            DepdencyInfo::RemotePackage { url, rev, subdir } => {
                self.contains_remote(name, url, rev, subdir)
            }
        })
    }

    fn contains_version(
        &self,
        name: impl AsRef<str>,
        range: &Range,
        uuid: &Option<String>,
    ) -> bool {
        let info = self.deps.get(name.as_ref()).unwrap();

        for pkg in info {
            if let Some(version) = &pkg.version {
                if !range.satisfies(version) {
                    continue;
                }
                if let Some(uuid) = uuid {
                    if uuid != &pkg.uuid {
                        continue;
                    }
                }
                return true;
            }
        }
        false
    }

    fn contains_local(&self, name: impl AsRef<str>, path: &str, subdir: &Option<String>) -> bool {
        let path = PathBuf::from(path);
        let path = std::env::current_dir().unwrap().join(path);
        let path = normalize_path(path.as_path());
        let path = path.to_str().unwrap();

        let info = self.deps.get(name.as_ref()).unwrap();

        for pkg in info {
            if let Some(pkg_path) = &pkg.path {
                if pkg_path != path {
                    continue;
                }
                if let Some(subdir) = subdir {
                    if pkg.repo_subdir != Some(subdir.clone()) {
                        continue;
                    }
                }
                return true;
            }

            if let Some(pkg_url) = &pkg.repo_url {
                if pkg_url != path {
                    continue;
                }
                if let Some(subdir) = subdir {
                    if pkg.repo_subdir != Some(subdir.clone()) {
                        continue;
                    }
                }
                return true;
            }
        }
        false
    }

    fn contains_remote(
        &self,
        name: impl AsRef<str>,
        url: &str,
        rev: &Option<String>,
        subdir: &Option<String>,
    ) -> bool {
        let info = self.deps.get(name.as_ref()).unwrap();

        for pkg in info {
            if let Some(pkg_url) = &pkg.repo_url {
                if pkg_url != url {
                    continue;
                }
                if let Some(rev) = rev {
                    if pkg.repo_rev != Some(rev.clone()) {
                        continue;
                    }
                }
                if let Some(subdir) = subdir {
                    if pkg.repo_subdir != Some(subdir.clone()) {
                        continue;
                    }
                }
                return true;
            }
        }
        false
    }
}
