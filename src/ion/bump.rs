use colorful::core::color_string::CString;
use node_semver::Version;
use colorful::Colorful;
use dialoguer::Confirm;
use anyhow::{format_err, Result};
use crate::{spec::{VersionSpec, JuliaProjectFile}, report::ReleaseReport};

pub enum VersionBumpValidation {
    NotRegistered,
    CurrentNotRegistered,
    CurrentContinuousGreater,
    NoIssues,
}
use VersionBumpValidation::*;

pub struct VersionBump {
    pub version_spec: VersionSpec,
    pub registry_name: Option<String>,
    pub project: JuliaProjectFile,

    // inferred from the registry
    pub latest_version: Option<Version>,
    pub version_to_release: Option<Version>,
}

impl VersionBump {
    pub fn registry(&mut self, registry_name: impl AsRef<str>) -> &mut Self {
        self.registry_name = Some(registry_name.as_ref().to_string());
        self
    }

    pub fn get_version(&self) -> Version {
        self.project.project.version.clone().unwrap()
    }

    pub fn report(&mut self) -> ReleaseReport {
        ReleaseReport {
            name: self.project.project.name.as_ref().unwrap().clone(),
            current_version: self.get_version(),
            latest_version: self.latest_version.clone(),
            release_version: self.version_to_release.clone().unwrap(),
            registry: self.registry_name.clone(),
            branch: None,
            commit: None,
            subdir: None,
        }
    }

    pub fn print_report(&mut self) -> &mut Self {
        println!("{}", self.report());
        self
    }

    pub fn validate(&mut self) -> Result<VersionBumpValidation> {
        self.version_to_release = Some(self.version_spec.update_version(&self.get_version()));
        if self.not_registered() {
            return Ok(NotRegistered);
        }

        if self.current_larger_than_latest()? {
            if self.is_current_continuously_greater()? {
                return Ok(CurrentContinuousGreater);
            } else {
                return Ok(CurrentNotRegistered);
            }
        }
        Ok(NoIssues)
    }

    pub fn confirm(&mut self, prompt: bool) -> Result<&mut Self> {
        match self.validate()? {
            NotRegistered => {
                if prompt && Confirm::new()
                    .with_prompt("This project is not registered in the registry. Do you want to register it?")
                    .interact()?
                {
                    Ok(self)
                } else {
                    Err(format_err!("Aborted"))
                }
            }
            CurrentNotRegistered => {
                Err(format_err!(
                    "The current version ({}) is not \
                    a registered version.", self.get_version())
                )
            }
            CurrentContinuousGreater => {
                let msg = format!("The current version ({}) \
                    is larger than the latest version ({}) and is \
                    continuously greater. Do you want to \
                    register it instead?",
                    self.get_version(),
                    self.latest_version.clone().unwrap()
                );

                if prompt && Confirm::new()
                    .with_prompt(msg)
                    .interact()?
                {
                    self.version_to_release = Some(self.get_version());
                    Ok(self)
                } else {
                    Err(format_err!("Aborted"))
                }
            }
            NoIssues => Ok(self),
        }
    }

    pub fn not_registered(&self) -> bool {
        self.latest_version.is_none()
    }

    pub fn current_larger_than_latest(&self) -> Result<bool> {
        let ver = self.get_version();
        match &self.latest_version {
            Some(latest) => Ok(&ver > latest),
            None => Ok(false),
        }
    }

    pub fn is_current_continuously_greater(&self) -> Result<bool> {
        let ver = self.get_version();
        match &self.latest_version {
            Some(latest) => Ok(is_version_continuously_greater(latest, &ver)),
            None => Ok(true),
        }
    }

    pub fn write(&self) -> Result<&Self> {
        let version = self.version_to_release.clone().unwrap();
        let mut project = self.project.project.clone();
        project.version = Some(version.clone());
        self.project.write()?;
        println!(
            "Version bumped to {}",
            CString::new(version.to_string()).green()
        );
        Ok(self)
    }

    pub fn commit(&self, no_commit: bool) -> Result<&Self> {
        if !no_commit {
            let mut cmd = std::process::Command::new("git");
            cmd.arg("add").arg("Project.toml");
            cmd.arg("commit")
                .arg("-m")
                .arg(format!("Bump version to {}", self.version_to_release.clone().unwrap()));
            cmd.status()?;
        }
        Ok(self)
    }
}

fn is_version_continuously_greater(latest: &Version, release: &Version) -> bool {
    // major release
    if latest.major + 1 == release.major && release.minor == 0 && release.patch == 0 {
        return true;
    }

    // minor release
    if latest.major == release.major && latest.minor + 1 == release.minor && release.patch == 0 {
        return true;
    }

    // patch release
    if latest.major == release.major
        && latest.minor == release.minor
        && latest.patch + 1 == release.patch
    {
        return true;
    }
    false
}