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

pub fn resources_dir() -> Result<PathBuf> {
    Ok(std::env::current_exe()?
        .parent()
        .ok_or_else(|| {
            anyhow::anyhow!("Could not find the parent directory of the current executable")
        })?
        .join("resources"))
}

pub fn components_dir() -> Result<PathBuf> {
    let path = resources_dir()?;
    Ok(path.join("components"))
}

pub fn template_dir() -> Result<PathBuf> {
    let path = resources_dir()?;
    Ok(path.join("templates"))
}
