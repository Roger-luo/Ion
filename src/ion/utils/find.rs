use crate::utils::*;
use crate::JuliaProject;
use crate::Registry;
use anyhow::{format_err, Error};
use dirs::home_dir;
use node_semver::Version;
use std::collections::BTreeMap;
use std::path::PathBuf;

pub fn current_project(dir: PathBuf) -> Option<PathBuf> {
    let home = home_dir().unwrap();
    let mut dir = dir;
    loop {
        for proj in &["Project.toml", "JuliaProject.toml"] {
            let file = dir.join(proj);
            if file.is_file() {
                return Some(file);
            }
        }
        if dir == home {
            break;
        }
        let old = dir.clone();
        dir = dir.parent().unwrap().to_path_buf();
        if dir == old {
            break;
        }
    }
    None
}

pub fn current_root_project(dir: PathBuf) -> Option<(JuliaProject, PathBuf)> {
    let guess = match current_project(dir) {
        Some(toml) => toml,
        None => return None,
    };

    let project: JuliaProject = toml::from_str(&std::fs::read_to_string(&guess).unwrap()).unwrap();
    match project.name {
        Some(_) => Some((project, guess)),
        None => match guess.parent() {
            Some(parent) => current_root_project(parent.to_path_buf()),
            None => None,
        },
    }
}

pub fn maximum_version(
    package: impl AsRef<str>,
    registry_name: impl AsRef<str>,
) -> Result<Version, Error> {
    let registry = registry(registry_name.as_ref())?;
    for (_, pkginfo) in registry.packages {
        if pkginfo.name.as_str() == package.as_ref() {
            let ver = version(pkginfo.path, registry_name.as_ref())?;
            return Ok(ver.keys().max().unwrap().clone());
        }
    }
    Err(format_err!("Package {} not found", package.as_ref()))
}

pub fn registry(name: impl AsRef<str>) -> Result<Registry, Error> {
    let data = registry_data("Registry.toml", name)?;
    Ok(toml::from_str(&data)?)
}

type VersionList = BTreeMap<Version, BTreeMap<String, String>>;

pub fn version(path: String, registry_name: impl AsRef<str>) -> Result<VersionList, Error> {
    let data = registry_data(format!("{}/Versions.toml", path), registry_name)?;
    Ok(toml::from_str(&data)?)
}

pub fn registry_data(file: impl AsRef<str>, name: impl AsRef<str>) -> Result<String, Error> {
    format!(
        r#"
    using Pkg
    for reg in Pkg.Registry.reachable_registries()
        if reg.name == "{name}"
            data = if isnothing(reg.in_memory_registry)
                read(joinpath(reg.path, "{file}"), String)
            else
                reg.in_memory_registry["{file}"]
            end
            println(data)
            break
        end
    end
    "#,
        file = file.as_ref(),
        name = name.as_ref()
    )
    .as_julia_command()
    .read_command()
}
