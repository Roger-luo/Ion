use super::version_spec::VersionSpec;
use crate::{
    utils::{current_root_project, git},
    JuliaProject, Registry,
};
use anyhow::{format_err, Result};
use colorful::Colorful;
use dialoguer::Confirm;
use node_semver::Version;
use std::path::PathBuf;

// user inputs

#[derive(Debug, Clone)]
pub struct Release {
    pub version_spec: VersionSpec,
    pub registry_name: String,

    // inferred
    pub project: JuliaProject,
    pub project_toml: PathBuf,
    pub path_to_repo: PathBuf,
    pub latest_version: Option<Version>,
    pub subdir: Option<PathBuf>,

    // optional
    pub branch: Option<String>,
    pub note: Option<String>, // release note
}

impl Release {
    pub fn plan(
        path: PathBuf,
        version_spec: VersionSpec,
        registry_name: impl AsRef<str>,
    ) -> Result<Release> {
        let (project, toml) = match current_root_project(path) {
            Some((project, path)) => (project, path),
            None => return Err(format_err!("No Project.toml found")),
        };

        let path_to_project = match toml.parent() {
            Some(path) => path.to_path_buf().canonicalize()?,
            None => return Err(format_err!("No parent directory found")),
        };

        let path_to_repo = match git::get_toplevel_path(&path_to_project) {
            Ok(path) => path,
            Err(_) => return Err(format_err!("No git repository found")),
        };

        if git::isdirty(&path_to_repo)? {
            return Err(format_err!("The repository is dirty"));
        }

        if !git::remote_exists(&path_to_repo)? {
            return Err(format_err!("remote does not exist"));
        }

        let subdir = path_to_project.strip_prefix(&path_to_repo)?;
        let registry = Registry::read(registry_name.as_ref())?;
        let uuid = project
            .uuid
            .as_ref()
            .ok_or_else(|| format_err!("No UUID found"))?;

        let mut handle = registry.package();
        let latest_ver = handle
            .uuid(uuid)
            .has_package()
            .then_some(handle.get_latest_version()?);

        if git::isdirty(&path_to_repo)? {
            return Err(format_err!("The repository is dirty"));
        }

        Ok(Release {
            version_spec,
            registry_name: registry_name.as_ref().to_string(),
            branch: None,
            project,
            project_toml: toml,
            path_to_repo,
            subdir: Some(subdir.to_path_buf()),
            latest_version: latest_ver,
            note: None,
        })
    }

    pub fn get_branch(&self) -> Result<String> {
        match self.branch {
            Some(ref branch) => Ok(branch.clone()),
            None => Ok(git::current_branch(&self.path_to_repo)?),
        }
    }

    pub fn ask_branch(&mut self) -> Result<&mut Self> {
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

    pub fn ask_note(&mut self) -> Result<&mut Self> {
        if let Some(note) = dialoguer::Editor::new()
            .extension("md")
            .edit("your release note")?
        {
            self.note = Some(note);
        } else {
            println!("Abort!");
        }
        Ok(self)
    }

    pub fn handle(&mut self) -> ReleaseHandler {
        ReleaseHandler {
            info: self,
            version_bump_sha: None,
            version_to_release: None,
            final_confirm: true,
            revert_changes: false,
        }
    }

    pub fn get_version(&self) -> Result<Version> {
        let version = self
            .project
            .version
            .as_ref()
            .ok_or(format_err!("No version found"))?;
        Ok(version.clone())
    }
}

pub struct ReleaseHandler<'a> {
    info: &'a Release,
    version_bump_sha: Option<String>, // commit sha of version bump
    version_to_release: Option<Version>, // version to release
    final_confirm: bool,
    revert_changes: bool,
}

impl ReleaseHandler<'_> {
    pub fn figure_release_version(&mut self) -> Result<&mut Self> {
        let version = self.info.get_version()?;
        let version_to_release = self.info.version_spec.update_version(&version);
        self.version_to_release = Some(version_to_release);
        Ok(self)
    }

    pub fn ask_about_current_version(&mut self) -> Result<&mut Self> {
        if self.current_larger_than_latest()? {
            if self.is_current_continuously_greater()? {
                // confirm from user
                // update release version to current
                // print report again
                eprintln!(
                    "{}: current version ({}) is a valid release version \
                    and is not released yet",
                    "warning".yellow().bold(),
                    self.info.get_version()?,
                );
                if Confirm::new()
                    .with_prompt("do you want to release current version?")
                    .interact()?
                {
                    self.version_to_release = Some(self.info.get_version()?.clone());
                    self.report()?;
                } else {
                    return Err(anyhow::format_err!("release cancelled").into());
                }
            } else {
                return Err(
                    anyhow::format_err!("current version is not a registered version").into(),
                );
            }
        }

        Ok(self)
    }

    pub fn ask_about_new_package(&mut self) -> Result<&mut Self> {
        if self.not_registered() {
            eprintln!(
                "{}: this package is not registered yet",
                "warning".yellow().bold()
            );
            self.confirm_release()?;
        }
        Ok(self)
    }

    pub fn confirm_release(&mut self) -> Result<&mut Self> {
        if self.final_confirm {
            if !Confirm::new()
                .with_prompt("do you want to register this version?")
                .default(true)
                .show_default(true)
                .interact()?
            {
                return Err(anyhow::format_err!("release cancelled").into());
            }
            self.final_confirm = false;
        }
        Ok(self)
    }

    pub fn report(&mut self) -> Result<&mut Self> {
        let project_name = self
            .info
            .project
            .name
            .as_ref()
            .ok_or(format_err!("No name found"))?;
        let version = self.info.get_version()?;
        let latest_version = &self.info.latest_version;
        let release_version = match &self.version_to_release {
            Some(v) => v,
            None => return Err(format_err!("No release version found")),
        };
        let registry_name = self.info.registry_name.to_owned();

        eprintln!("{}: {}", "          project".cyan(), project_name);
        if let Some(b) = self.info.branch.as_ref() {
            eprintln!("{}: {}", "           branch".cyan(), b)
        }
        eprintln!("{}: {}", "         registry".cyan(), registry_name);
        if let Some(latest) = latest_version {
            if latest == &version {
                eprintln!("{}: {}", "latest/current version".cyan(), latest);
            } else {
                eprintln!("{}: {}", "   latest version".blue(), latest);
                eprintln!("{}: {}", "  current version".blue(), version);
            }
        } else {
            eprintln!("{}: {}", "  current version".blue(), version);
        }
        eprintln!("{}: {}", "  release version".blue(), release_version);
        Ok(self)
    }

    pub fn not_registered(&self) -> bool {
        self.info.latest_version.is_none()
    }

    pub fn current_larger_than_latest(&self) -> Result<bool> {
        let ver = self.info.get_version()?;
        match &self.info.latest_version {
            Some(latest) => Ok(&ver > latest),
            None => Ok(false),
        }
    }

    pub fn is_current_continuously_greater(&self) -> Result<bool> {
        let ver = self.info.get_version()?;
        match &self.info.latest_version {
            Some(latest) => Ok(is_version_continuously_greater(&latest, &ver)),
            None => Ok(true),
        }
    }

    pub fn sync_with_remote(&mut self) -> Result<&mut Self> {
        let repo = &self.info.path_to_repo;
        git::pull(repo)?;
        git::push(repo)?;
        Ok(self)
    }

    pub fn write_project(&mut self) -> Result<&mut Self> {
        match self.info.version_spec {
            VersionSpec::Current => {} // do nothing
            _ => {
                self.info.project.write(&self.info.project_toml)?;
            }
        }
        Ok(self)
    }

    pub fn current_sha256(&self) -> Result<String> {
        git::sha_256(&self.info.path_to_repo, &self.info.get_branch()?)
    }

    pub fn commit_changes(&mut self) -> Result<&mut Self> {
        match self.info.version_spec {
            VersionSpec::Current => {} // do nothing
            _ => {
                let version_to_release = self
                    .version_to_release
                    .as_ref()
                    .ok_or_else(|| format_err!("No version to release found"))?;
                let message = format!("bump version  to {}", version_to_release);
                git::commit(&self.info.path_to_repo, &message)?;
            }
        }
        self.version_bump_sha = Some(self.current_sha256()?);
        Ok(self)
    }

    pub fn summon_registrator(&mut self) -> Result<&mut Self> {
        let watermark: String = "release via [ion](https://rogerluo.dev)\n".into();
        let body: String = "@JuliaRegistrator register".into();
        let body = format!("{}\n\n{}", watermark, body);

        let body = match &self.info.branch {
            Some(branch) => format!("{} branch={}", body, branch),
            None => body,
        };

        let body = match &self.info.subdir {
            Some(subdir) => format!("{} subdir={}", body, subdir.display()),
            None => body,
        };

        let body = match &self.info.note {
            Some(note) => format!("{}\n\nRelease notes:\n\n{}", body, note),
            None => body,
        };

        let (owner, repo) = git::remote_repo(&self.info.path_to_repo)?;
        let sha = self.current_sha256()?;

        // let octocrab = octocrab::instance();
        // let page = octocrab
        //     .commits(owner, repo)
        //     .create_comment(sha, body);
        Ok(self)
    }

    pub fn revert_commit_maybe(&mut self) -> Result<&mut Self> {
        Ok(self)
    }
}

fn is_version_continuously_greater(latest: &Version, release: &Version) -> bool {
    if release > latest {
        return true;
    }
    // patch release
    if latest.major == release.major
        && latest.minor == release.minor
        && latest.patch + 1 == release.patch
    {
        return true;
    }

    // minor release
    if latest.major == release.major
        && latest.minor + 1 == release.minor
        && latest.patch == release.patch
    {
        return true;
    }

    // major release
    if latest.major + 1 == release.major
        && latest.minor == release.minor
        && latest.patch == release.patch
    {
        return true;
    }
    false
}
