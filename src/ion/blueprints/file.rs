use super::{Config, Context};
use anyhow::{format_err, Result};
use handlebars::Handlebars;
use log::debug;
use serde_derive::Deserialize;
use std::fmt::Display;
use std::io::Read;
use std::path::{self, PathBuf};

#[derive(Debug, Deserialize)]
pub struct TemplateFile {
    pub root: PathBuf,
    pub path: PathBuf,
    pub file: String,
}

impl TemplateFile {
    pub fn root(&self, config: &Config) -> Result<PathBuf> {
        if self.root.is_relative() {
            Ok(config.resources().join(&self.root))
        } else {
            Ok(self.root.to_owned())
        }
    }

    pub fn to_path_buf(&self, config: &Config) -> Result<PathBuf> {
        Ok(self.root(config)?.join(&self.path).join(&self.file))
    }

    pub fn read_source(&self, config: &Config) -> Result<String> {
        debug!("reading template:\n {:#?}", self);
        let path: PathBuf = self.to_path_buf(config)?;
        if !path.is_file() {
            return Err(format_err!("Template file not found: {}", path.display()));
        }
        let mut template_file = std::fs::File::open(path)?;
        let mut source = String::new();
        template_file.read_to_string(&mut source)?;
        Ok(source)
    }

    pub fn write(&self, content: &String, ctx: &Context, name: &str) -> Result<()> {
        if name.contains(path::MAIN_SEPARATOR) {
            return Err(format_err!(
                "target file name cannot contain path separator: {}",
                name
            ));
        }
        let dst = ctx.project.path.join(&self.path);
        if !dst.is_dir() {
            debug!("creating directory: {}", dst.display());
            std::fs::create_dir_all(&dst).unwrap();
        }
        std::fs::write(dst.join(name), content)?;
        Ok(())
    }

    pub fn copy(&self, config: &Config, ctx: &Context, name: &str) -> Result<()> {
        self.read_source(config)
            .and_then(|source| self.write(&source, ctx, name))
    }

    pub fn render(&self, config: &Config, ctx: &Context, name: &str) -> Result<()> {
        if name.contains(path::MAIN_SEPARATOR) {
            return Err(format_err!(
                "target file name cannot contain path separator: {}",
                name
            ));
        }
        debug!("rendering template:\n {:?}", self);
        debug!("start rendering for name: {}", name);
        let source = self.read_source(config)?;
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
        write!(
            f,
            "{}:{}",
            self.root.display(),
            self.path.join(&self.file).display()
        )
    }
}

pub struct TemplateHandler<'de> {
    pub config: &'de Config,
    pub template: TemplateFile,
}

impl<'de> TemplateHandler<'de> {
    pub fn render(&self, ctx: &Context, name: &str) -> Result<()> {
        self.template.render(self.config, ctx, name)
    }

    pub fn copy(&self, ctx: &Context, name: &str) -> Result<()> {
        self.template.copy(self.config, ctx, name)
    }
}

pub trait AsTemplate {
    fn as_template<'de>(&self, config: &'de Config) -> Result<TemplateHandler<'de>>;
}

impl<T: Display> AsTemplate for T {
    fn as_template<'de>(&self, config: &'de Config) -> Result<TemplateHandler<'de>> {
        let path = self.to_string();
        let components = path.split(':').collect::<Vec<_>>();
        let (root, path) = if components.len() == 2 {
            let root = components[0].parse::<PathBuf>()?;
            let path = components[1].parse::<PathBuf>()?;
            (root, path)
        } else if components.len() == 1 {
            let root = config.components_dir();
            let path = components[0].parse::<PathBuf>()?;
            (root, path)
        } else {
            return Err(format_err!("Invalid template string syntax: {}", path));
        };

        if path.components().count() < 2 {
            return Err(format_err!(
                "Template file path must have at \
                least one directory in path, can be '.'"
            ));
        }

        let file = path
            .file_name()
            .ok_or_else(|| format_err!("Template file path must have a file name"))?
            .to_str()
            .ok_or_else(|| format_err!("encountered non-utf8 path"))?
            .to_string();
        let path = path
            .parent()
            .ok_or_else(|| format_err!("the path terminates in a root or prefix"))?;

        Ok(TemplateHandler {
            config,
            template: TemplateFile {
                root,
                path: path.to_path_buf(),
                file,
            },
        })
    }
}
