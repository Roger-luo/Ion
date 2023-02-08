use anyhow::Result;
use std::path::{Component, Path, PathBuf};

// from cargo:utils/paths.rs
pub fn normalize_path(path: &Path) -> PathBuf {
    let mut components = path.components().peekable();
    let mut ret = if let Some(c @ Component::Prefix(..)) = components.peek().cloned() {
        components.next();
        PathBuf::from(c.as_os_str())
    } else {
        PathBuf::new()
    };

    for component in components {
        match component {
            Component::Prefix(..) => unreachable!(),
            Component::RootDir => {
                ret.push(component.as_os_str());
            }
            Component::CurDir => {}
            Component::ParentDir => {
                ret.pop();
            }
            Component::Normal(c) => {
                ret.push(c);
            }
        }
    }
    ret
}

pub fn dot_julia_dir() -> Result<PathBuf> {
    match dirs::home_dir() {
        Some(mut home) => {
            home.push(".julia");
            Ok(home)
        }
        None => Err(anyhow::anyhow!("Failed to get home directory")),
    }
}

pub fn config_dir() -> Result<PathBuf> {
    let exe = std::env::current_exe()?;
    let bin = exe
        .parent()
        .expect("Failed to get parent directory of executable");
    Ok(bin.join("config"))
}

pub fn config_file() -> Result<PathBuf> {
    Ok(config_dir()?.join("config.toml"))
}

#[cfg(not(debug_assertions))]
pub fn config_dir() -> Result<PathBuf> {
    match dirs::config_dir() {
        Some(root) => Ok(root.join("ion")),
        None => Err(anyhow::anyhow!("Failed to get config directory")),
    }
}

pub fn resources_dir() -> Result<PathBuf> {
    Ok(config_dir()?.join("resources"))
}

pub fn components_dir() -> Result<PathBuf> {
    let path = resources_dir()?;
    Ok(path.join("components"))
}

pub fn template_dir() -> Result<PathBuf> {
    let path = resources_dir()?;
    Ok(path.join("templates"))
}
