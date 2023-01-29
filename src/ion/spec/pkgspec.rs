use node_semver::Range;
use serde_derive::{Deserialize, Serialize};
use std::fmt::Display;
use std::path::{Path, PathBuf};
use url::Url;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageSpec {
    pub name: Option<String>,
    pub uuid: Option<String>,
    pub version: Option<String>,
    pub tree_hash: Option<String>,
    // pub repo: Option<String>,
    pub path: Option<String>,
    pub pinned: Option<bool>,
    pub url: Option<String>,
    pub rev: Option<String>,
    pub subdir: Option<String>,
}

impl PackageSpec {
    pub fn new(expr: &String) -> Self {
        let mut name: String = expr.to_owned();
        let mut version: Option<String> = None;
        let mut rev: Option<String> = None;

        name = if expr.contains('@') {
            let parts = expr.split('@').collect::<Vec<_>>();
            assert!(parts.len() == 2, "Invalid package name: {expr}");
            let version_str = expr.split('@').last().unwrap();
            assert!(
                Range::parse(version_str).is_ok(),
                "Invalid version: {version_str}"
            );
            version = Some(version_str.to_string());
            parts[0].to_string()
        } else {
            name
        };

        name = if name.contains('#') {
            let parts = name.split('#').collect::<Vec<_>>();
            println!("{parts:?}");
            assert!(parts.len() == 2, "Invalid package name: {expr}");
            rev = Some(parts[1].to_string());
            parts[0].to_string()
        } else {
            name
        };

        if PathBuf::from(name.clone()).is_dir() {
            return Self {
                name: None,
                uuid: None,
                url: None,
                path: Some(name),
                subdir: None,
                rev,
                version,
                tree_hash: None,
                pinned: None,
            };
        }

        if Url::parse(name.as_str()).is_ok() {
            return Self {
                name: None,
                url: Some(name),
                path: None,
                subdir: None,
                rev,
                version,
                uuid: None,
                tree_hash: None,
                pinned: None,
            };
        }

        Self {
            name: Some(name),
            url: None,
            path: None,
            subdir: None,
            rev,
            version,
            uuid: None,
            tree_hash: None,
            pinned: None,
        }
    }

    pub fn from_path(path: &Path) -> PackageSpec {
        Self {
            name: None,
            url: None,
            path: Some(path.to_str().unwrap().to_string()),
            subdir: None,
            rev: None,
            version: None,
            uuid: None,
            tree_hash: None,
            pinned: None,
        }
    }
}

impl Display for PackageSpec {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut fields = Vec::<String>::new();
        if let Some(name) = &self.name {
            fields.push(format!("name=\"{name}\""));
        }
        if let Some(url) = &self.url {
            fields.push(format!("url=\"{url}\""));
        }
        if let Some(path) = &self.path {
            fields.push(format!("path=\"{path}\""));
        }
        if let Some(subdir) = &self.subdir {
            fields.push(format!("subdir=\"{subdir}\""));
        }
        if let Some(rev) = &self.rev {
            fields.push(format!("rev=\"{rev}\""));
        }
        if let Some(version) = &self.version {
            fields.push(format!("version=\"{version}\""));
        }
        if let Some(uuid) = &self.uuid {
            fields.push(format!("uuid=\"{uuid}\""));
        }
        if let Some(tree_hash) = &self.tree_hash {
            fields.push(format!("tree_hash=\"{tree_hash}\""));
        }
        if let Some(pinned) = &self.pinned {
            if *pinned {
                fields.push(format!("pinned=true"));
            } else {
                fields.push(format!("pinned=false"));
            }
        }
        write!(f, "PackageSpec({})", fields.join(", "))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_package_spec() {
        let spec = PackageSpec::new(&"foo".to_string());
        assert_eq!(spec.name, Some("foo".to_string()));
        assert_eq!(spec.url, None);
        assert_eq!(spec.path, None);
        assert_eq!(spec.subdir, None);
        assert_eq!(spec.rev, None);
        assert_eq!(spec.version, None);

        let spec = PackageSpec::new(&"Example@0.1".to_string());
        assert_eq!(spec.name, Some("Example".to_string()));
        assert_eq!(spec.url, None);
        assert_eq!(spec.path, None);
        assert_eq!(spec.subdir, None);
        assert_eq!(spec.rev, None);
        assert_eq!(spec.version, Some("0.1".to_string()));

        let spec = PackageSpec::new(&r#"https://github.com/Example/Example.git"#.to_string());
        assert_eq!(spec.name, None);
        assert_eq!(
            spec.url,
            Some(r#"https://github.com/Example/Example.git"#.to_string())
        );
        assert_eq!(spec.path, None);
        assert_eq!(spec.subdir, None);
        assert_eq!(spec.rev, None);
        assert_eq!(spec.version, None);

        let spec = PackageSpec::new(&r#"https://github.com/Example/Example.git#main"#.to_string());
        assert_eq!(spec.name, None);
        assert_eq!(
            spec.url,
            Some(r#"https://github.com/Example/Example.git"#.to_string())
        );
        assert_eq!(spec.path, None);
        assert_eq!(spec.subdir, None);
        assert_eq!(spec.rev, Some("main".to_string()));
        assert_eq!(spec.version, None);
    }
}
