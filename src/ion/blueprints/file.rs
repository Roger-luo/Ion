use log::debug;
use std::io::Read;
use super::Context;
use std::fmt::Display;
use handlebars::Handlebars;
use serde_derive::Deserialize;
use std::path::{self, PathBuf};
use anyhow::{format_err, Error};
use crate::dirs::{components_dir, resources_dir};

#[derive(Debug, Deserialize)]
pub struct TemplateFile {
    #[serde(default = "TemplateFile::default_root")]
    pub root: PathBuf,
    pub path: PathBuf,
    pub file: String,
}

impl TemplateFile {
    pub fn default_root() -> PathBuf {
        components_dir()
    }

    pub fn from_str(path: &str) -> TemplateFile {
        let path = PathBuf::from(path);
        TemplateFile::from_path(path)
    }

    pub fn from_path(path: PathBuf) -> TemplateFile {
        let nlevels = path.components().collect::<Vec<_>>().len();
        assert!(nlevels > 1, "Template file path must have at \
            least one directory in path, can be '.'");
        let file = path.file_name().unwrap().to_str().unwrap().to_string();
        let path = path.parent().unwrap();
        TemplateFile { root: components_dir(), path: path.to_path_buf(), file: file}
    }

    pub fn to_path_buf(&self) -> PathBuf {
        let root = if self.root.is_relative() {
            resources_dir().join(&self.root)
        } else {
            self.root.to_owned()
        };
        root
            .join(self.path.to_owned())
            .join(self.file.to_owned())
    }

    pub fn read_source(&self) -> Result<String, Error> {
        debug!("reading template:\n {:?}", self);
        let path : PathBuf = self.to_path_buf();
        if !path.is_file() {
            return Err(format_err!("Template file not found: {}", path.display()));
        }
        let mut template_file = std::fs::File::open(path)?;
        let mut source = String::new();
        template_file.read_to_string(&mut source)?;
        Ok(source)
    }

    pub fn write(&self, content: &String, ctx: &Context, name: &str) -> Result<(), Error> {
        if name.contains(path::MAIN_SEPARATOR) {
            return Err(format_err!("target file name cannot contain path separator: {}", name));
        }
        let dst = ctx.project.path.join(self.path.to_owned());
        if !dst.is_dir() {
            debug!("creating directory: {}", dst.display());
            std::fs::create_dir_all(&dst).unwrap();
        }
        std::fs::write(dst.join(name), content)?;
        Ok(())
    }

    pub fn copy(&self, ctx: &Context, name: &str) -> Result<(), Error> {
        self.read_source().and_then(|source| {
            self.write(&source, ctx, name)
        })
    }

    pub fn render(&self, ctx: &Context, name: &str) -> Result<(), Error> {
        if name.contains(path::MAIN_SEPARATOR) {
            return Err(format_err!("target file name cannot contain path separator: {}", name));
        }
        debug!("rendering template:\n {:?}", self);
        debug!("start rendering for name: {}", name);
        let source = self.read_source()?;
        let mut handlebars = Handlebars::new();

        debug!("registering template: {}", name);
        if let Err(e) = handlebars.register_template_string(name, source) {
            return Err(format_err!("Error registering template: {}", e));
        }
        debug!("template registered: {}", name);
        let result = match handlebars.render(name, &ctx) {
            Ok(s) => s,
            Err(e) => return Err(format_err!("Error rendering result: {}", e)),
        };
        debug!("template rendered");
        self.write(&result, ctx, name)
    }
}

impl Display for TemplateFile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.path.display())
    }
}
