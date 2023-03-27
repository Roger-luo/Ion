use std::path::{Component, Path, PathBuf};

// from cargo:utils/paths.rs
pub fn normalize_path(path: impl AsRef<Path>) -> PathBuf {
    let mut components = path.as_ref().components().peekable();
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normpath() {
        assert_eq!(normalize_path("a/b/c"), PathBuf::from("a/b/c"));
        assert_eq!(normalize_path("a/b/../c"), PathBuf::from("a/c"));
        assert_eq!(normalize_path("a/b/./c"), PathBuf::from("a/b/c"));
        assert_eq!(normalize_path("a/b/../../c"), PathBuf::from("c"));
    }
}
