use anyhow::Result;
use serde_derive::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::utils::{auth, config_file};

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
    #[serde(default)]
    github: Option<GitHub>,
    #[serde(default)]
    julia: Julia,
    #[serde(default)]
    template: Template, // url to the template registry
    #[serde(default = "Config::default_env")]
    env: PathBuf, // env directory path
    #[serde(default = "Config::default_resources")]
    resources: PathBuf, // resources directory path
}

impl Config {
    pub fn write(&self) -> Result<()> {
        let content = toml::to_string_pretty(self)?;
        std::fs::write(config_file()?, content)?;
        Ok(())
    }

    pub fn read() -> Result<Self> {
        let file = Self::file()?;
        let config = if !file.exists() {
            let config = Self::default();
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

    #[cfg(debug_assertions)]
    pub fn dir() -> Result<PathBuf> {
        let exe = std::env::current_exe()?;
        let bin = exe
            .parent()
            .expect("Failed to get parent directory of executable");
        Ok(bin.join("config"))
    }

    #[cfg(not(debug_assertions))]
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
        std::fs::remove_file(config_file()?)?;
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
        let registry =
            url::Url::parse("https://github.com/Roger-luo/ion-templates/releases/latest/download/")
                .unwrap();
        Self {
            default: "project".into(),
            registry,
        }
    }
}

impl Default for Julia {
    fn default() -> Self {
        Self {
            exe: PathBuf::from("julia"),
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
