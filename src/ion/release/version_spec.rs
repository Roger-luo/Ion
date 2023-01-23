use anyhow::Error;
use node_semver::Version;

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

impl Into<String> for VersionSpec {
    fn into(self) -> String {
        match self {
            VersionSpec::Patch => "patch".into(),
            VersionSpec::Minor => "minor".into(),
            VersionSpec::Major => "major".into(),
            VersionSpec::Spec(v) => v.to_string().into(),
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
