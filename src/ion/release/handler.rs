use super::version_spec::VersionSpec;
use crate::{
    utils::{current_root_project, find, git},
    JuliaProject,
};
use anyhow::{format_err, Error};
use colorful::Colorful;
use node_semver::Version;
use std::path::PathBuf;

pub struct ReleaseHandler {
    version_spec: VersionSpec, // version spec
    registry_name: String,     // registry name

    project: Option<JuliaProject>,
    toml: Option<PathBuf>,
    latest: Option<Version>,
    repo: Option<PathBuf>,   // path to the git repo
    subdir: Option<PathBuf>, // subdir to the package
    branch: Option<String>,  // branch to release

    note: Option<String>,                // release note
    version_to_release: Option<Version>, // version to release
}

impl ReleaseHandler {
    pub fn new(version_spec: VersionSpec, registry_name: impl AsRef<str>) -> Self {
        Self {
            version_spec,
            registry_name: registry_name.as_ref().to_string(),
            project: None,
            toml: None,
            latest: None,
            repo: None,
            subdir: None,
            branch: None,
            note: None,
            version_to_release: None,
        }
    }

    pub fn path(&mut self, path: PathBuf) -> Result<&mut Self, Error> {
        let (project, toml) = match current_root_project(path) {
            Some((project, path)) => (project, path),
            None => return Err(format_err!("No Project.toml found")),
        };

        let path_to_project = match toml.parent() {
            Some(path) => path.to_path_buf().canonicalize()?,
            None => return Err(format_err!("No parent directory found")),
        };

        let path_to_repo = git::get_toplevel_path(&path_to_project)?;
        let subdir = path_to_project.strip_prefix(&path_to_repo)?;
        let latest_ver = match find::maximum_version(
            project.name.as_ref().unwrap(),
            &self.registry_name,
        ) {
            Ok(ver) => Some(ver),
            Err(_) => None,
        };

        if git::isdirty(&path_to_repo)? {
            return Err(format_err!("The repository is dirty"));
        }

        self.project = Some(project);
        self.latest = latest_ver;
        self.toml = Some(toml);
        self.repo = Some(path_to_repo);
        self.subdir = Some(subdir.into());
        Ok(self)
    }

    pub fn branch<S: ToString>(&mut self, branch: S) -> &mut Self {
        self.branch = Some(branch.to_string());
        self
    }

    pub fn note<S: ToString>(&mut self, note: S) -> &mut Self {
        self.note = Some(note.to_string());
        self
    }

    pub fn update_version(&mut self) -> Result<&mut Self, Error> {
        let version = self.get_version()?;
        let version_to_release = self.version_spec.update_version(version);
        self.version_to_release = Some(version_to_release);
        Ok(self)
    }

    pub fn set_release_version(&mut self, version: Version) -> &mut Self {
        self.version_to_release = Some(version);
        self
    }

    pub fn get_branch(&self) -> Result<&String, Error> {
        self.branch
            .as_ref()
            .ok_or_else(|| format_err!("No branch found"))
    }

    pub fn get_project(&self) -> Result<&JuliaProject, Error> {
        self.project
            .as_ref()
            .ok_or_else(|| format_err!("No project found"))
    }

    pub fn get_version(&self) -> Result<&Version, Error> {
        self.get_project()?
            .version
            .as_ref()
            .ok_or_else(|| format_err!("No version found"))
    }

    pub fn get_release_version(&self) -> Result<&Version, Error> {
        self.version_to_release
            .as_ref()
            .ok_or_else(|| format_err!("No version to release found"))
    }

    pub fn get_latest_version(&self) -> Option<&Version> {
        self.latest.as_ref()
    }

    pub fn get_project_name(&self) -> Result<&String, Error> {
        self.get_project()?
            .name
            .as_ref()
            .ok_or_else(|| format_err!("No project name found"))
    }

    pub fn report(&mut self) -> Result<&mut Self, Error> {
        let project_name = self.get_project_name()?.to_owned();
        let version = self.get_version()?;
        let latest_version = self.get_latest_version();
        let release_version = self.get_release_version()?.to_string();
        let registry_name = self.registry_name.to_owned();

        eprintln!("{}: {}", "          project".cyan(), project_name);
        if let Some(b) = self.branch.as_ref() { eprintln!("{}: {}", "           branch".cyan(), b) }
        eprintln!("{}: {}", "         registry".cyan(), registry_name);
        if let Some(latest) = latest_version {
            if latest == version {
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

    pub fn not_registered(&self) -> Result<bool, Error> {
        match self.get_latest_version() {
            Some(_) => Ok(false),
            None => Ok(true),
        }
    }

    pub fn current_larger_than_latest(&self) -> Result<bool, Error> {
        let ver = self.get_version()?;
        match self.get_latest_version() {
            Some(latest) => Ok(ver > latest),
            None => Ok(false),
        }
    }

    pub fn is_current_continuously_greater(&self) -> Result<bool, Error> {
        let ver = self.get_version()?;
        match self.get_latest_version() {
            Some(latest) => Ok(is_version_continuously_greater(latest, ver)),
            None => Ok(true),
        }
    }

    pub fn sync_with_remote(&self) -> Result<&Self, Error> {
        let repo = self
            .repo
            .as_ref()
            .ok_or_else(|| format_err!("repo not found"))?;
        git::pull(repo)?;
        git::push(repo)?;
        Ok(self)
    }

    pub fn write_project(&self) -> Result<&Self, Error> {
        match self.version_spec {
            VersionSpec::Current => {} // do nothing
            _ => {
                self.get_project()?.write(self.toml.as_ref().unwrap())?;
            }
        }
        Ok(self)
    }

    pub fn commit_changes(&self) -> Result<&Self, Error> {
        match self.version_spec {
            VersionSpec::Current => {} // do nothing
            _ => {
                let version_to_release = self
                    .version_to_release
                    .as_ref()
                    .ok_or_else(|| format_err!("No version to release found"))?;
                let message = format!("bump version  to {}", version_to_release);
                git::commit(self.repo.as_ref().unwrap(), &message)?;
            }
        }
        Ok(self)
    }

    pub fn summon_registrator(&mut self) -> Result<&mut Self, Error> {
        Ok(self)
    }

    pub fn revert_commit(&mut self) -> Result<&mut Self, Error> {
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
