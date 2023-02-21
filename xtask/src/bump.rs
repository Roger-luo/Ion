use cargo_metadata::MetadataCommand;
use toml_edit::{Document, value};
use clap::{arg, ArgMatches, Command};
use anyhow::Result;

pub fn cli() -> Command {
    Command::new("bump")
        .about("Bump the version of the current crate")
        .arg(arg!(<VERSION> "The version to bump to"))
}

#[derive(Debug, Clone)]
pub enum Modifier {
    Major,
    Minor,
    Patch,
    Custom(semver::Version),
}

impl Modifier {
    pub fn parse(s: impl AsRef<str>) -> Result<Self> {
        let s = s.as_ref().trim();
        if s == "major" {
            Ok(Self::Major)
        } else if s == "minor" {
            Ok(Self::Minor)
        } else if s == "patch" {
            Ok(Self::Patch)
        } else {
            Ok(Self::Custom(semver::Version::parse(s)?))
        }
    }
}

pub trait VersionModifier {
    fn bump(&self, version: Modifier) -> semver::Version;
}

impl VersionModifier for semver::Version {
    fn bump(&self, version: Modifier) -> semver::Version {
        match version {
            Modifier::Major => {
                let mut v = self.clone();
                v.major += 1;
                v.minor = 0;
                v.patch = 0;
                v
            },
            Modifier::Minor => {
                let mut v = self.clone();
                v.minor += 1;
                v.patch = 0;
                v
            },
            Modifier::Patch => {
                let mut v = self.clone();
                v.patch += 1;
                v
            },
            Modifier::Custom(v) => v,
        }
    }
}

pub fn exec(matches: &ArgMatches) -> Result<()> {
    let metadata_cmd = MetadataCommand::new();
    let metadata = metadata_cmd.exec()?;
    let root_package = metadata.root_package().unwrap();
    let manifest = root_package.manifest_path.to_owned();
    let source = std::fs::read_to_string(manifest.to_owned())?;
    let mut doc = source.parse::<Document>()?;
    let modifier_str = matches.get_one::<String>("VERSION").expect("VERSION is required");
    let modifier = Modifier::parse(modifier_str)?;

    if let Some(version_str) = doc["package"]["version"].as_str() {
        let version = semver::Version::parse(version_str)?;
        let new_version = version.bump(modifier.clone());
        doc["package"]["version"] = value(new_version.to_string());
        std::fs::write(manifest, doc.to_string())?;
    }
    Ok(())
}
