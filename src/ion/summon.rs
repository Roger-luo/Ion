use crate::spec::JuliaProjectFile;
use crate::utils::{git, Auth};
use anyhow::{format_err, Result};
use dialoguer::{Confirm, Editor};
use octocrab::{models::commits::Comment, Octocrab};
use spinoff::{Color, Spinner, Spinners};
use std::path::PathBuf;
use tokio::runtime::Builder;

pub struct JuliaRegistrator {
    pub project: JuliaProjectFile,
    pub subdir: Option<String>,
    pub path_to_repo: PathBuf,
    // optional
    pub prompt: bool,
    pub branch: Option<String>,
    pub note: Option<String>, // release note
}

impl JuliaRegistrator {
    pub fn from_project(project: JuliaProjectFile) -> Result<Self> {
        let path_to_repo = match git::get_toplevel_path(&project.path) {
            Ok(path) => path,
            Err(_) => return Err(format_err!("No git repository found")),
        };

        if git::isdirty(&path_to_repo)? {
            return Err(format_err!("The repository is dirty"));
        }

        if !git::remote_exists(&path_to_repo)? {
            return Err(format_err!("remote does not exist"));
        }

        let subdir = project.path.strip_prefix(&path_to_repo)?;
        let subdir = if subdir.components().count() == 0 {
            None
        } else {
            Some(
                subdir
                    .to_path_buf()
                    .to_str()
                    .expect("non-unicode path")
                    .to_string(),
            )
        };

        Ok(JuliaRegistrator {
            project,
            subdir,
            path_to_repo,
            prompt: false,
            branch: None,
            note: None,
        })
    }

    pub fn summon(&mut self, skip_note: bool) -> Result<()> {
        let repo = &self.path_to_repo.clone();
        if git::isdirty(repo)? {
            return Err(format_err!("The repository is dirty"));
        }

        if self.prompt {
            self.ask_branch()?;
            self.ask_note(skip_note)?;
        }

        let commet = self.registerator_comment();
        println!("You are about to summon JuliaRegistrator with the following comment:");
        println!("{}", commet);
        if self.prompt
            && !Confirm::new()
                .with_prompt("Do you want to continue?")
                .default(true)
                .interact()?
        {
            return Ok(());
        }

        let path = self.path_to_repo.clone();
        let branch = self.branch.clone();
        git::checkout_and(&path, &branch, || {
            log::debug!("syncing with remote");
            git::pull(repo)?;
            git::push(repo)?;

            let auth = Auth::new(vec!["repo", "read:org"]);
            let token = auth.get_token()?;

            let spinner = Spinner::new(Spinners::Dots, "Summon JuliaRegistrator...", Color::Blue);
            let result = Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap()
                .block_on(self.summon_task(token, &commet));

            match result {
                Ok(comment) => {
                    spinner.success("JuliaRegistrator summoned! You are good to go!");
                    println!("Comment: {}", comment.html_url);
                    Ok(())
                }
                Err(e) => {
                    spinner.fail("Failed to summon JuliaRegistrator");
                    Err(e.into())
                }
            }
        })
    }

    async fn summon_task(&self, token: String, body: &String) -> Result<Comment> {
        let (owner, repo) = git::remote_repo(&self.path_to_repo)?;
        let sha = self.current_sha256()?;
        let octocrab = Octocrab::builder().personal_token(token).build()?;
        log::debug!("owner: {}, repo: {}, sha: {}", owner, repo, sha);
        let commits = octocrab.commits(owner, repo);
        let future = commits.create_comment(sha, body).send().await;
        match future {
            Ok(comment) => Ok(comment),
            Err(e) => Err(e.into()),
        }
    }

    pub fn branch<S>(&mut self, branch: Option<S>) -> &mut Self
    where
        S: Into<String>,
    {
        self.branch = branch.and_then(|b| Some(b.into()));
        self
    }

    pub fn note<S: Into<String>>(&mut self, note: S) -> &mut Self {
        self.note = Some(note.into());
        self
    }

    pub fn prompt(&mut self, prompt: bool) -> &mut Self {
        self.prompt = prompt;
        self
    }

    pub fn current_sha256(&self) -> Result<String> {
        git::sha_256(&self.path_to_repo, &self.get_branch()?)
    }

    pub fn get_branch(&self) -> Result<String> {
        match self.branch {
            Some(ref branch) => Ok(branch.clone()),
            None => Ok(git::current_branch(&self.path_to_repo)?),
        }
    }

    fn ask_branch(&mut self) -> Result<&mut Self> {
        let current_branch = git::current_branch(&self.path_to_repo)?;
        let default_branch = git::default_branch(&self.path_to_repo)?;
        let branch = match self.branch {
            Some(ref branch) => branch.clone(),
            None => {
                let branch = dialoguer::Input::new()
                    .with_prompt("Branch to release")
                    .default(current_branch)
                    .show_default(true)
                    .interact()?;
                branch
            }
        };

        if branch != default_branch {
            let confirm = dialoguer::Confirm::new()
                .with_prompt(format!(
                    "You are not on the default branch ({}), continue?",
                    default_branch
                ))
                .interact()?;
            if !confirm {
                return Err(format_err!("Aborted"));
            }
        }

        self.branch = Some(branch);
        Ok(self)
    }

    fn ask_note(&mut self, skip: bool) -> Result<&mut Self> {
        if skip {
            return Ok(self);
        }

        if let Some(note) = Editor::new().extension("md").edit("## Release Note\n")? {
            self.note = Some(note);
        } else {
            println!("Abort!");
        }
        Ok(self)
    }

    fn registerator_comment(&self) -> String {
        let watermark: String = "release via [ion](https://rogerluo.dev)\n".into();

        #[cfg(debug_assertions)]
        let body: String = "JuliaRegistrator register".into();
        #[cfg(not(debug_assertions))]
        let body: String = "@JuliaRegistrator register".into();

        let body = format!("{}\n\n{}", watermark, body);

        let body = match &self.branch {
            Some(branch) => format!("{} branch={}", body, branch),
            None => body,
        };

        let body = match &self.subdir {
            Some(subdir) => format!("{} subdir={}", body, subdir),
            None => body,
        };

        let body = match &self.note {
            Some(note) => format!("{}\n\nRelease notes:\n\n{}", body, note),
            None => body,
        };
        body
    }
}
