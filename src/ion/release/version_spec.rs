use anyhow::Error;
use node_semver::Version;

#[derive(Debug, Clone, PartialEq)]
pub enum VersionSpec {
    Patch,
    Minor,
    Major,
    Spec(Version),
    Current,
}

impl VersionSpec {
    pub fn from_string(s: impl AsRef<str>) -> Result<Self, Error> {
        match s.as_ref() {
            "patch" => Ok(VersionSpec::Patch),
            "minor" => Ok(VersionSpec::Minor),
            "major" => Ok(VersionSpec::Major),
            "current" => Ok(VersionSpec::Current),
            _ => Ok(VersionSpec::Spec(Version::parse(s)?)),
        }
    }

    pub fn update_version(&self, version: &Version) -> Version {
        match self {
            VersionSpec::Patch => version.bump_patch(),
            VersionSpec::Minor => version.bump_minor(),
            VersionSpec::Major => version.bump_major(),
            VersionSpec::Spec(v) => v.clone(),
            VersionSpec::Current => version.clone(),
        }
    }

    pub fn is_patch(&self) -> bool {
        matches!(self, VersionSpec::Patch)
    }
}

trait BumpVersion {
    fn bump_major(&self) -> Version;
    fn bump_minor(&self) -> Version;
    fn bump_patch(&self) -> Version;
}

impl BumpVersion for Version {
    fn bump_major(&self) -> Version {
        Version {
            major: self.major + 1,
            minor: 0,
            patch: 0,
            pre_release: Vec::new(),
            build: Vec::new(),
        }
    }

    fn bump_minor(&self) -> Version {
        Version {
            major: self.major,
            minor: self.minor + 1,
            patch: 0,
            pre_release: Vec::new(),
            build: Vec::new(),
        }
    }

    fn bump_patch(&self) -> Version {
        Version {
            major: self.major,
            minor: self.minor,
            patch: self.patch + 1,
            pre_release: Vec::new(),
            build: Vec::new(),
        }
    }
}

impl From<VersionSpec> for String {
    fn from(val: VersionSpec) -> Self {
        match val {
            VersionSpec::Patch => "patch".into(),
            VersionSpec::Minor => "minor".into(),
            VersionSpec::Major => "major".into(),
            VersionSpec::Spec(v) => v.to_string(),
            VersionSpec::Current => "current".into(),
        }
    }
}

impl From<String> for VersionSpec {
    fn from(s: String) -> Self {
        match s.as_str() {
            "patch" => VersionSpec::Patch,
            "minor" => VersionSpec::Minor,
            "major" => VersionSpec::Major,
            "current" => VersionSpec::Current,
            _ => VersionSpec::Spec(Version::parse(s.as_str()).unwrap()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_string() -> Result<(), Error> {
        assert_eq!(VersionSpec::from_string("patch")?, VersionSpec::Patch);
        assert_eq!(VersionSpec::from_string("minor")?, VersionSpec::Minor);
        assert_eq!(VersionSpec::from_string("major")?, VersionSpec::Major);
        assert_eq!(
            VersionSpec::from_string("current")?,
            VersionSpec::Current
        );
        assert_eq!(
            VersionSpec::from_string("0.1.0")?,
            VersionSpec::Spec(Version::parse("0.1.0")?)
        );
        Ok(())
    }

    #[test]
    fn test_update_version() -> Result<(), Error> {
        let version = Version::parse("0.1.0")?;
        assert_eq!(
            VersionSpec::Patch.update_version(&version),
            Version::parse("0.1.1")?
        );
        assert_eq!(
            VersionSpec::Minor.update_version(&version),
            Version::parse("0.2.0")?
        );
        assert_eq!(
            VersionSpec::Major.update_version(&version),
            Version::parse("1.0.0")?
        );
        assert_eq!(
            VersionSpec::Spec(Version::parse("0.2.0")?).update_version(&version),
            Version::parse("0.2.0")?
        );
        assert_eq!(
            VersionSpec::Current.update_version(&version),
            Version::parse("0.1.0")?
        );
        Ok(())
    }

    #[test]
    fn test_is_patch() -> Result<(), Error> {
        assert!(VersionSpec::Patch.is_patch());
        assert!(!VersionSpec::Minor.is_patch());
        assert!(!VersionSpec::Major.is_patch());
        assert!(!VersionSpec::Spec(Version::parse("0.2.0")?).is_patch());
        assert!(!VersionSpec::Current.is_patch());
        Ok(())
    }
}
