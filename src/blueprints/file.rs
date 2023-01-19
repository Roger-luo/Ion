use std::io::Read;
use std::fmt::Display;
use std::path::PathBuf;
use anyhow::{format_err, Error};
use handlebars::Handlebars;
use serde_derive::Deserialize;

use crate::{dirs::{components_dir, resources_dir}, blueprints::project_dir};

use super::Context;

#[derive(Debug, Deserialize)]
pub struct TemplateFile {
    pub root: PathBuf,
    pub path: PathBuf,
    pub file: String,
}

impl TemplateFile {
    pub fn from_str(path: &str) -> TemplateFile {
        let path = PathBuf::from(path);
        TemplateFile::from_path(path)
    }

    pub fn from_path(path: PathBuf) -> TemplateFile {
        if !path.is_file() {
            panic!("cannot find template file: {}", path.display());
        }

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

    pub fn render(&self, ctx: &Context, name: &str) -> Result<(), Error> {
        let path : PathBuf = self.to_path_buf();
        let mut template_file = std::fs::File::open(path)?;
        let mut source = String::new();
        template_file.read_to_string(&mut source)?;
        let mut handlebars = Handlebars::new();
        assert!(handlebars.register_template_string(name, source).is_ok());
        let result = match handlebars.render(name, &ctx.meta) {
            Ok(s) => s,
            Err(e) => return Err(format_err!("Error rendering result: {}", e)),
        };
        let dst = project_dir(ctx)
            .join(self.path.to_owned());

        if !dst.is_dir() {
            std::fs::create_dir_all(&dst).unwrap();
        }
        std::fs::write(dst.join(name), result)?;
        Ok(())
    }
}

impl Display for TemplateFile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.path.display())
    }
}
