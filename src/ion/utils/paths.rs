use std::path::{Path, PathBuf, Component};

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

pub fn resources_dir() -> PathBuf {
    if cfg!(debug_assertions) {
        let mut template = PathBuf::new();
        template.push(std::env::var("CARGO_MANIFEST_DIR").unwrap());
        template.push("resources");
        template
    } else {
        let mut template = PathBuf::new();
        template.push(dirs::config_dir().unwrap());
        template.push("ion");
        template
    }
}

pub fn components_dir() -> PathBuf {
    let path = resources_dir();
    path.join("components")
}

pub fn template_dir() -> PathBuf {
    let path = resources_dir();
    path.join("templates")
}
