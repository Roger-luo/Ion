use crate::utils::git;
use crate::{
    report::ReleaseReport,
    spec::{JuliaProjectFile, VersionSpec},
    Registry,
};
use anyhow::{format_err, Result};
use colorful::core::color_string::CString;
use colorful::Colorful;
use dialoguer::Confirm;
use node_semver::Version;

pub enum VersionBumpValidation {
    NotRegistered,
    CurrentNotRegistered,
    CurrentContinuousGreater,
    NoIssues,
}
use VersionBumpValidation::*;

pub struct VersionBumpHandler {
    bump: VersionBump,
    branch: Option<String>,

    // options
    commit: bool,
    report: bool,
    confirm: bool,
}

impl VersionBumpHandler {
    pub fn new(project: JuliaProjectFile, version_spec: VersionSpec) -> Self {
        Self {
            bump: VersionBump::new(project, version_spec),
            branch: None,
            commit: true,
            report: true,
            confirm: true,
        }
    }

    pub fn registry(&mut self, registry: Registry) -> Result<&mut Self> {
        self.bump.registry(registry)?;
        Ok(self)
    }

    pub fn branch<S>(&mut self, branch: Option<S>) -> &mut Self
    where
        S: Into<String>,
    {
        self.branch = branch.map(|b| b.into());
        self
    }

    pub fn commit(&mut self, commit: bool) -> &mut Self {
        self.commit = commit;
        self
    }

    pub fn report(&mut self, report: bool) -> &mut Self {
        self.report = report;
        self
    }

    fn report_content(&mut self) -> ReleaseReport {
        ReleaseReport {
            name: self.bump.project.project.name.as_ref().unwrap().clone(),
            current_version: self.bump.get_version(),
            latest_version: self.bump.latest_version.clone(),
            release_version: self.bump.version_to_release.clone().unwrap(),
            registry: self.bump.registry_name.clone(),
            branch: self.branch.clone(),
            commit: None,
            subdir: None,
        }
    }

    pub fn confirm(&mut self, confirm: bool) -> &mut Self {
        self.confirm = confirm;
        self
    }

    pub fn write(&mut self) -> Result<()> {
        if self.commit && git::isdirty(&self.bump.project.path)? {
            return Err(format_err!("The repository is dirty"));
        }

        let path = self.bump.project.path.clone();
        let branch = self.branch.clone();
        git::checkout_and(&path, &branch, || {
            log::debug!("Writing version bump");
            self.bump.confirm(self.confirm)?;

            if self.report {
                println!("{}", &self.report_content());
            }

            if self.confirm
                && self.report
                && !Confirm::new()
                    .with_prompt("Do you want to continue?")
                    .default(true)
                    .interact()?
            {
                return Ok(());
            }

            self.bump.write()?;
            if self.commit {
                self.bump.commit()?;
            }
            Ok(())
        })?;
        Ok(())
    }
}

pub struct VersionBump {
    registry_name: Option<String>,
    project: JuliaProjectFile,

    // inferred from the registry
    latest_version: Option<Version>,
    version_to_release: Option<Version>,
}

impl VersionBump {
    pub fn new(project: JuliaProjectFile, version_spec: VersionSpec) -> Self {
        let version = project
            .project
            .version
            .clone()
            .expect("The project file does not contain a version");
        let version_to_release = Some(version_spec.update_version(&version));
        Self {
            registry_name: None,
            project,
            latest_version: None,
            version_to_release,
        }
    }

    /// add a registry to the version bumper
    /// this will also set the latest version
    /// to the latest version in the registry
    /// and the version to release to the
    /// version specified by the version spec
    /// in the registry
    ///
    /// # Arguments
    ///
    /// * `registry` - the registry to add
    ///
    /// # Returns
    ///
    /// * `Result<&mut Self>` - the version bumper
    ///
    /// # Errors
    ///
    /// * `Error` - if the registry does not contain
    /// the project
    pub fn registry(&mut self, registry: Registry) -> Result<&mut Self> {
        self.registry_name = Some(registry.name.to_owned());
        self.latest_version = Some(
            registry
                .package()
                .uuid(self.get_uuid()?)
                .get_latest_version()?,
        );
        Ok(self)
    }

    pub fn get_name(&self) -> Result<String> {
        self.project
            .project
            .name
            .clone()
            .ok_or_else(|| format_err!("The project file does not contain a name"))
    }

    pub fn get_uuid(&self) -> Result<String> {
        self.project
            .project
            .uuid
            .clone()
            .ok_or_else(|| format_err!("The project file does not contain a uuid"))
    }

    pub fn get_version(&self) -> Version {
        self.project.project.version.clone().unwrap()
    }

    pub fn get_release_version(&self) -> Version {
        self.version_to_release.clone().unwrap()
    }

    pub fn validate(&mut self) -> Result<VersionBumpValidation> {
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
            NotRegistered | CurrentNotRegistered => {
                println!(
                    "The current version ({}) is not \
                    registered in the registry {}.",
                    self.get_version(),
                    self.registry_name.clone().unwrap()
                );

                if Confirm::new()
                    .with_prompt("Do you want to change the version?")
                    .default(true)
                    .interact()?
                {
                    Ok(self)
                } else {
                    Err(format_err!("Aborted"))
                }
            }
            CurrentContinuousGreater => {
                println!(
                    "The current version ({}) \
                    is larger than the latest version ({}) and is \
                    continuously greater.",
                    self.get_version(),
                    self.latest_version.clone().unwrap()
                );

                if prompt
                    && Confirm::new()
                        .with_prompt("Do you want to keep it instead?")
                        .interact()?
                {
                    self.version_to_release = Some(self.get_version());
                    Ok(self)
                } else {
                    Ok(self)
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
        let release = self.version_to_release.clone().unwrap();
        let mut project = self.project.clone();
        let current = project.get_version()?;

        log::debug!("bump version from {} to {}", current, release);
        if current != release {
            project.update_version(&release);
            project.write()?;
            println!(
                "Version bumped to {}",
                CString::new(release.to_string()).green()
            );
        }
        Ok(self)
    }

    pub fn commit(&self) -> Result<&Self> {
        if self.get_release_version() == self.get_version() {
            return Ok(self);
        }

        git::add(&self.project.path, "Project.toml")?;
        let msg = format!(
            "Bump version to {}",
            self.version_to_release.clone().unwrap()
        );
        git::commit(&self.project.path, msg.as_str())?;
        Ok(self)
    }

    pub fn revert(&self) -> Result<&Self> {
        self.project.write()?;
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_version_continuously_greater() {
        let latest = Version::parse("0.1.0").unwrap();
        let release = Version::parse("0.1.1").unwrap();
        assert!(is_version_continuously_greater(&latest, &release));

        let latest = Version::parse("0.1.0").unwrap();
        let release = Version::parse("0.2.0").unwrap();
        assert!(is_version_continuously_greater(&latest, &release));

        let latest = Version::parse("0.1.0").unwrap();
        let release = Version::parse("1.0.0").unwrap();
        assert!(is_version_continuously_greater(&latest, &release));

        let latest = Version::parse("0.1.0").unwrap();
        let release = Version::parse("0.1.0").unwrap();
        assert!(!is_version_continuously_greater(&latest, &release));

        let latest = Version::parse("0.1.0").unwrap();
        let release = Version::parse("0.0.0").unwrap();
        assert!(!is_version_continuously_greater(&latest, &release));

        let latest = Version::parse("0.1.0").unwrap();
        let release = Version::parse("0.1.2").unwrap();
        assert!(!is_version_continuously_greater(&latest, &release));

        let latest = Version::parse("0.1.0").unwrap();
        let release = Version::parse("0.2.1").unwrap();
        assert!(!is_version_continuously_greater(&latest, &release));

        let latest = Version::parse("0.1.0").unwrap();
        let release = Version::parse("1.0.1").unwrap();
        assert!(!is_version_continuously_greater(&latest, &release));
    }
}
