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

#[cfg(debug_assertions)]
pub fn resources_dir() -> Result<PathBuf> {
    next_bin_resources_dir()
}

#[cfg(not(debug_assertions))]
pub fn resources_dir() -> Result<PathBuf> {
    match dot_julia_dir() {
        Ok(mut dot_julia) => {
            dot_julia.push("resources");
            Ok(dot_julia)
        }
        Err(_) => next_bin_resources_dir(),
    }
}

fn next_bin_resources_dir() -> Result<PathBuf> {
    let exe = std::env::current_exe()?;
    let bin = exe
        .parent()
        .expect("Failed to get parent directory of executable");
    let resources = bin
        .parent()
        .expect("Failed to get parent directory of bin")
        .join("resources");
    Ok(normalize_path(&resources))
}

pub fn components_dir() -> Result<PathBuf> {
    let path = resources_dir()?;
    Ok(path.join("components"))
}

pub fn template_dir() -> Result<PathBuf> {
    let path = resources_dir()?;
    Ok(path.join("templates"))
}
