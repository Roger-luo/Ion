//! Project fixture builder for setting up test directory structures.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::Error;

/// A materialized project directory, ready for use in a [`Scenario`](crate::Scenario).
pub struct Project {
    dir: ProjectDir,
}

enum ProjectDir {
    Temp(tempfile::TempDir),
    External(PathBuf),
}

impl Project {
    /// Path to the materialized project directory.
    pub fn path(&self) -> &Path {
        match &self.dir {
            ProjectDir::Temp(tmp) => tmp.path(),
            ProjectDir::External(p) => p,
        }
    }

    /// Start building a project from a template directory on disk.
    pub fn from_template(path: impl AsRef<Path>) -> ProjectBuilder {
        ProjectBuilder {
            source: BuilderSource::Template(path.as_ref().to_path_buf()),
            vars: HashMap::new(),
            excludes: Vec::new(),
            includes: Vec::new(),
            overrides: HashMap::new(),
            extra_files: Vec::new(),
            extra_dirs: Vec::new(),
        }
    }

    /// Start building an empty project (no template).
    pub fn empty() -> ProjectBuilder {
        ProjectBuilder {
            source: BuilderSource::Empty,
            vars: HashMap::new(),
            excludes: Vec::new(),
            includes: Vec::new(),
            overrides: HashMap::new(),
            extra_files: Vec::new(),
            extra_dirs: Vec::new(),
        }
    }
}

#[allow(dead_code)] // Template variant used in Task 4
enum BuilderSource {
    Empty,
    Template(PathBuf),
}

/// Builder for creating a [`Project`] fixture.
pub struct ProjectBuilder {
    source: BuilderSource,
    vars: HashMap<String, String>,
    excludes: Vec<String>,
    includes: Vec<String>,
    overrides: HashMap<String, String>,
    extra_files: Vec<(String, String)>,
    extra_dirs: Vec<String>,
}

impl ProjectBuilder {
    pub fn var(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.vars.insert(key.into(), value.into());
        self
    }

    pub fn vars<I, K, V>(mut self, vars: I) -> Self
    where
        I: IntoIterator<Item = (K, V)>,
        K: Into<String>,
        V: Into<String>,
    {
        for (k, v) in vars {
            self.vars.insert(k.into(), v.into());
        }
        self
    }

    pub fn exclude(mut self, path: impl Into<String>) -> Self {
        self.excludes.push(path.into());
        self
    }

    pub fn include(mut self, path: impl Into<String>) -> Self {
        self.includes.push(path.into());
        self
    }

    pub fn override_file(mut self, path: impl Into<String>, content: impl Into<String>) -> Self {
        self.overrides.insert(path.into(), content.into());
        self
    }

    pub fn file(mut self, path: impl Into<String>, content: impl Into<String>) -> Self {
        self.extra_files.push((path.into(), content.into()));
        self
    }

    pub fn dir(mut self, path: impl Into<String>) -> Self {
        self.extra_dirs.push(path.into());
        self
    }

    pub fn build(self) -> Result<Project, Error> {
        let tmp = tempfile::tempdir()?;
        self.build_into(tmp.path())?;
        Ok(Project {
            dir: ProjectDir::Temp(tmp),
        })
    }

    pub fn build_in(self, path: impl AsRef<Path>) -> Result<Project, Error> {
        let path = path.as_ref().to_path_buf();
        self.build_into(&path)?;
        Ok(Project {
            dir: ProjectDir::External(path),
        })
    }

    fn build_into(self, target: &Path) -> Result<(), Error> {
        match self.source {
            BuilderSource::Empty => {}
            BuilderSource::Template(_) => {
                todo!("template rendering not yet implemented");
            }
        }

        for (path, content) in &self.extra_files {
            let dest = target.join(path);
            if let Some(parent) = dest.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::write(&dest, content)?;
        }

        for dir in &self.extra_dirs {
            std::fs::create_dir_all(target.join(dir))?;
        }

        Ok(())
    }
}
