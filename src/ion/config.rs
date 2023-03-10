use anyhow::Result;
use serde_derive::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::utils::auth;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Julia {
    pub exe: PathBuf, // the Julia command path
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GitHub {
    pub username: String, // GitHub username
    pub token: String,    // GitHub token
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Template {
    pub default: String,
    pub registry: url::Url,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Config {
    #[serde(default = "Config::default_env")]
    env: PathBuf, // env directory path
    #[serde(default = "Config::default_resources")]
    resources: PathBuf, // resources directory path
    #[serde(default)]
    julia: Julia,
    #[serde(default)]
    template: Template, // url to the template registry
    #[serde(default)]
    github: Option<GitHub>,
}

impl Config {
    pub fn write(&self) -> Result<()> {
        log::debug!("writing config file: {:#?}", self);
        let content = toml::to_string_pretty(self).unwrap();
        log::debug!("writing config file: {:#?}", content);
        std::fs::write(Self::file()?, content)?;
        Ok(())
    }

    pub fn read() -> Result<Self> {
        let file = Self::file()?;
        log::debug!("config file: {}", file.display());
        if !Self::dir()?.exists() {
            std::fs::create_dir_all(Self::dir()?)?;
        };

        log::debug!("config file: {}", file.exists());

        let config = if !file.exists() {
            let config = Self::default();
            log::debug!("creating config file: {:#?}", config);
            config.write()?;
            config
        } else {
            let content = std::fs::read_to_string(file)?;
            toml::from_str(&content)?
        };
        Ok(config)
    }

    pub fn resources(&self) -> PathBuf {
        self.resources.clone()
    }

    pub fn components_dir(&self) -> PathBuf {
        self.resources.join("components")
    }

    pub fn template_dir(&self) -> PathBuf {
        self.resources.join("templates")
    }

    pub fn env(&self) -> PathBuf {
        self.env.clone()
    }

    pub fn julia(&self) -> Julia {
        self.julia.clone()
    }

    pub fn github(&mut self) -> Result<GitHub> {
        self.ensure_login()?;
        Ok(self.github.as_ref().unwrap().clone())
    }

    pub fn template(&self) -> Template {
        self.template.clone()
    }

    pub fn login(&mut self) -> Result<()> {
        let auth = auth::Auth::new(vec!["repo", "read:org"]);
        let handler = auth.github();
        let token = handler.get_token().expect("Failed to get GitHub token");
        let username = handler
            .get_username(token.clone())
            .expect("Failed to get GitHub username");

        self.github = Some(GitHub {
            username: username.clone(),
            token,
        });

        self.write()?;
        println!("Logged in as {username}");
        Ok(())
    }

    pub fn logout(&mut self) -> Result<()> {
        self.github = None;
        self.write()?;
        Ok(())
    }

    pub fn ensure_login(&mut self) -> Result<&Self> {
        if self.github.is_none() {
            self.login()?;
        }
        Ok(self)
    }

    #[cfg(not(feature = "config-dir"))]
    pub fn dir() -> Result<PathBuf> {
        let exe = std::env::current_exe()?;
        let bin = exe
            .parent()
            .expect("Failed to get parent directory of executable");
        Ok(bin.join("config"))
    }

    #[cfg(feature = "config-dir")]
    pub fn dir() -> Result<PathBuf> {
        match dirs::config_dir() {
            Some(root) => Ok(root.join("ion")),
            None => Err(anyhow::anyhow!("Failed to get config directory")),
        }
    }

    pub fn file() -> Result<PathBuf> {
        Ok(Self::dir()?.join("config.toml"))
    }

    pub fn dir_panic() -> PathBuf {
        Self::dir().expect("Failed to get config directory")
    }

    pub fn delete() -> Result<()> {
        std::fs::remove_file(Self::file()?)?;
        Ok(())
    }

    pub fn default_env() -> PathBuf {
        Self::dir_panic().join("env")
    }

    pub fn default_resources() -> PathBuf {
        Self::dir_panic().join("resources")
    }
}

impl Default for Template {
    fn default() -> Self {
        let registry = "https://github.com/Roger-luo/ion-templates\
        /releases/latest/download/ion-templates.tar.gz";
        Self {
            default: "project".into(),
            registry: url::Url::parse(registry).unwrap(),
        }
    }
}

impl Default for Julia {
    fn default() -> Self {
        Self {
            exe: "julia".into(),
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            github: None,
            julia: Julia::default(),
            template: Template::default(),
            env: Config::default_env(),
            resources: Config::default_resources(),
        }
    }
}
