use crate::config::Config;
use crate::JuliaProject;
use anyhow::Result;
use dirs::home_dir;
use std::path::PathBuf;
use std::process::Command;

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
        None => {
            let parent_dir = match guess.parent() {
                Some(parent) => parent,
                None => return None,
            };

            match parent_dir.parent() {
                Some(parent) => current_root_project(parent.to_path_buf()),
                None => None,
            }
        }
    }
}

pub fn julia_version(config: &Config) -> Result<node_semver::Version> {
    let output = Command::new(config.julia().exe).arg("--version").output()?;

    let version = String::from_utf8(output.stdout)?;
    let version = version.trim();
    let version = version.split_whitespace().last().unwrap();
    Ok(node_semver::Version::parse(version)?)
}
