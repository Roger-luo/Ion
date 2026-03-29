//! Project fixture builder for setting up test directory structures.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::Error;
use crate::manifest::TemplateManifest;

/// A materialized project directory, ready for use in a [`Scenario`](crate::Scenario).
pub struct Project {
    dir: ProjectDir,
}

impl std::fmt::Debug for Project {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Project")
            .field("path", &self.path())
            .finish()
    }
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
        if let BuilderSource::Template(ref template_dir) = self.source {
            let template_dir = template_dir.clone();
            if !template_dir.exists() {
                return Err(Error::TemplateNotFound { path: template_dir });
            }

            let manifest = TemplateManifest::from_dir(&template_dir)?;

            // Validate variables
            let mut missing = Vec::new();
            for (name, decl) in &manifest.variables {
                if decl.default.is_none() && !self.vars.contains_key(name) {
                    missing.push(name.clone());
                }
            }
            if !missing.is_empty() {
                missing.sort();
                return Err(Error::MissingVariable { names: missing });
            }

            for name in self.vars.keys() {
                if !manifest.variables.contains_key(name) {
                    return Err(Error::UnknownVariable { name: name.clone() });
                }
            }

            // Build minijinja context: defaults, then user vars
            let mut context = HashMap::new();
            for (name, decl) in &manifest.variables {
                if let Some(default) = &decl.default {
                    context.insert(name.clone(), default.clone());
                }
            }
            for (name, value) in &self.vars {
                context.insert(name.clone(), value.clone());
            }

            // Walk template dir, collect files (excluding template.toml)
            let template_files = walk_template_dir(&template_dir)?;

            // Compute file set with optional/include/exclude filtering
            let file_set = self.compute_file_set(&template_files, &manifest)?;

            // Create minijinja environment
            let mut env = minijinja::Environment::new();
            env.set_keep_trailing_newline(true);

            // Render and write each file
            for rel_path in &file_set {
                let source_path = template_dir.join(rel_path);

                // Determine destination: mapped or natural
                let dest_template = manifest
                    .files
                    .mappings
                    .get(rel_path.as_str())
                    .cloned()
                    .unwrap_or_else(|| rel_path.clone());

                // Check for overrides
                let content_template =
                    if let Some(override_content) = self.overrides.get(rel_path.as_str()) {
                        override_content.clone()
                    } else {
                        std::fs::read_to_string(&source_path)?
                    };

                // Render destination path
                let dest_rendered = env.render_str(&dest_template, &context).map_err(|source| {
                    Error::TemplateRender {
                        file: rel_path.clone(),
                        source,
                    }
                })?;

                // Render content
                let content_rendered =
                    env.render_str(&content_template, &context)
                        .map_err(|source| Error::TemplateRender {
                            file: rel_path.clone(),
                            source,
                        })?;

                // Write file
                let dest = target.join(&dest_rendered);
                if let Some(parent) = dest.parent() {
                    std::fs::create_dir_all(parent)?;
                }
                std::fs::write(&dest, content_rendered)?;
            }

            // Create symlinks
            for (link_template, target_template) in &manifest.files.symlinks {
                let link_rendered = env.render_str(link_template, &context).map_err(|source| {
                    Error::TemplateRender {
                        file: link_template.clone(),
                        source,
                    }
                })?;
                let target_rendered =
                    env.render_str(target_template, &context)
                        .map_err(|source| Error::TemplateRender {
                            file: link_template.clone(),
                            source,
                        })?;

                let link_path = target.join(&link_rendered);
                if let Some(parent) = link_path.parent() {
                    std::fs::create_dir_all(parent)?;
                }

                let abs_target = link_path.parent().unwrap().join(&target_rendered);
                if !abs_target.exists() {
                    return Err(Error::SymlinkTarget { path: abs_target });
                }

                #[cfg(unix)]
                std::os::unix::fs::symlink(&target_rendered, &link_path)?;
                #[cfg(windows)]
                {
                    if abs_target.is_dir() {
                        std::os::windows::fs::symlink_dir(&target_rendered, &link_path)?;
                    } else {
                        std::os::windows::fs::symlink_file(&target_rendered, &link_path)?;
                    }
                }
            }
        }

        // Write extra files (verbatim, no rendering)
        for (path, content) in &self.extra_files {
            let dest = target.join(path);
            if let Some(parent) = dest.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::write(&dest, content)?;
        }

        // Create extra dirs
        for dir in &self.extra_dirs {
            std::fs::create_dir_all(target.join(dir))?;
        }

        Ok(())
    }

    fn compute_file_set(
        &self,
        all_files: &[String],
        manifest: &TemplateManifest,
    ) -> Result<Vec<String>, Error> {
        let optional_globs = if manifest.files.optional.is_empty() {
            None
        } else {
            let mut builder = globset::GlobSetBuilder::new();
            for pattern in &manifest.files.optional {
                builder.add(
                    globset::Glob::new(pattern).map_err(|e| Error::TemplateRender {
                        file: "template.toml".into(),
                        source: minijinja::Error::new(
                            minijinja::ErrorKind::SyntaxError,
                            format!("invalid glob in optional: {e}"),
                        ),
                    })?,
                );
            }
            Some(builder.build().map_err(|e| Error::TemplateRender {
                file: "template.toml".into(),
                source: minijinja::Error::new(
                    minijinja::ErrorKind::SyntaxError,
                    format!("invalid glob set: {e}"),
                ),
            })?)
        };

        let mut result = Vec::new();
        for file in all_files {
            let is_optional = optional_globs.as_ref().is_some_and(|gs| gs.is_match(file));

            let explicitly_included = self
                .includes
                .iter()
                .any(|inc| file == inc || file.starts_with(&format!("{inc}/")));

            let explicitly_excluded = self
                .excludes
                .iter()
                .any(|exc| file == exc || file.starts_with(&format!("{exc}/")));

            if explicitly_excluded {
                continue;
            }
            if is_optional && !explicitly_included {
                continue;
            }
            result.push(file.clone());
        }

        Ok(result)
    }
}

fn walk_template_dir(dir: &Path) -> Result<Vec<String>, Error> {
    let mut files = Vec::new();
    walk_recursive(dir, dir, &mut files)?;
    Ok(files)
}

fn walk_recursive(root: &Path, current: &Path, files: &mut Vec<String>) -> Result<(), Error> {
    for entry in std::fs::read_dir(current)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            walk_recursive(root, &path, files)?;
        } else {
            let rel = path
                .strip_prefix(root)
                .expect("path is under root")
                .to_string_lossy()
                .to_string();
            if rel != "template.toml" {
                files.push(rel);
            }
        }
    }
    Ok(())
}
