use crate::utils::git;
use crate::{config::Config, registry::Registry};
use anyhow::{format_err, Result};
use dialoguer::Confirm;
use octocrab::Octocrab;
use std::path::{Path, PathBuf};
use tokio::runtime::Builder;
use url::Url;

// clone::Clone::new("General")
//     .from_github(url_or_name)
//     .dest(dest)
//     .run()?;
pub struct Clone {
    registry: String,
}

pub struct RemoteProject {
    url: Url,
    owner: String,
    repo: String,
    dest: PathBuf,
}

impl Clone {
    pub fn new(registry: impl AsRef<str>) -> Self {
        Clone {
            registry: registry.as_ref().to_string(),
        }
    }

    pub fn from_github(
        &self,
        config: &Config,
        url_or_name: impl AsRef<str>,
    ) -> Result<RemoteProject> {
        let url = match Url::parse(url_or_name.as_ref()) {
            Ok(url) => url,
            Err(_) => {
                let url = Registry::read(config, self.registry.clone())?
                    .package(config)
                    .name(url_or_name.as_ref())
                    .get_url()?;
                url
            }
        };

        let repo = match url.path_segments() {
            Some(segments) => segments
                .last()
                .ok_or_else(|| format_err!("invliad URL"))?
                .to_string(),
            None => return Err(format_err!("invalid URL")),
        };
        log::debug!("url: {}", url);
        log::debug!("repo: {}", repo);

        let (repo, dest) = if repo.ends_with(".jl.git") {
            (
                repo[..repo.len() - 4].to_string(),
                repo[..repo.len() - 7].to_string(),
            )
        } else if repo.ends_with(".git") {
            (
                repo[..repo.len() - 4].to_string(),
                repo[..repo.len() - 4].to_string(),
            )
        } else {
            (repo.clone(), repo)
        };

        log::debug!("repo: {}", repo);

        let owner = match url.path_segments() {
            Some(mut segments) => segments
                .nth_back(1)
                .ok_or_else(|| format_err!("invalid URL"))?
                .to_string(),
            None => return Err(format_err!("invalid URL")),
        };

        Ok(RemoteProject {
            url,
            owner,
            repo,
            dest: PathBuf::from(dest),
        })
    }
}

impl RemoteProject {
    pub fn dest<S>(&mut self, dest: Option<S>) -> Result<&mut Self>
    where
        S: AsRef<Path>,
    {
        if let Some(dest) = dest {
            self.dest = dest.as_ref().to_path_buf();
        }
        Ok(self)
    }

    pub fn run(&self, config: &mut Config, force: bool) -> Result<()> {
        let github = &config.github()?;
        if force && self.dest.exists() {
            std::fs::remove_dir_all(&self.dest)?;
        }

        Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(self.run_task(&github.username, &github.token))?;
        Ok(())
    }

    async fn run_task(&self, username: &String, token: &String) -> Result<()> {
        let octocrab = Octocrab::builder()
            .personal_token(token.to_owned())
            .build()?;
        let is_collab = octocrab
            .repos(self.owner.clone(), self.repo.clone())
            .is_collaborator(username)
            .await?;
        let path = self.dest.clone();

        if !is_collab
            && Confirm::new()
                .with_prompt(
                    "You are not a collaborator on this repository. Would you like to fork it?",
                )
                .default(true)
                .interact()?
        {
            log::debug!("forking {}/{} to {}", self.owner, self.repo, username);
            let fork = octocrab
                .repos(self.owner.clone(), self.repo.clone())
                .create_fork()
                .send()
                .await?;
            if let Some(full_name) = fork.full_name {
                println!("Forked to {full_name}");
            }
            if let Some(clone_url) = fork.clone_url {
                git::clone(clone_url.as_str(), &path)?;
                let p = std::process::Command::new("git")
                    .arg("remote")
                    .arg("add")
                    .arg("upstream")
                    .arg(self.url.as_str())
                    .current_dir(&path)
                    .status()?;
                if !p.success() {
                    return Err(format_err!("failed to add upstream remote"));
                }
                return Ok(());
            } else {
                log::warn!("failed to get clone url from forked repository");
            }
        }
        git::clone(self.url.as_str(), &path)?;
        Ok(())
    }
}
