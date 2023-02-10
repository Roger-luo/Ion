use crate::config::Config;
use crate::utils::{Julia, ReadCommand};
use anyhow::{format_err, Result};
use node_semver::Version;
use serde_derive::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::PathBuf;
use url::Url;

#[derive(Debug, Serialize, Deserialize)]
pub struct PackagePath {
    pub name: String,
    pub path: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Registry {
    pub name: String,
    pub repo: Url,
    pub uuid: String,
    pub description: String,
    pub packages: BTreeMap<String, PackagePath>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PackageInfo {
    name: String,
    uuid: String,
    repo: Url,
}

// registry queries
type VersionInfo = BTreeMap<Version, BTreeMap<String, String>>;
type DepsInfo = BTreeMap<String, BTreeMap<String, String>>;
type CompatInfo = BTreeMap<String, BTreeMap<String, String>>;

#[derive(Debug)]
pub struct RegistryHandler<'de> {
    registry: &'de Registry,
    config: Config,
    name: Option<String>, // package name
    uuid: Option<String>, // package uuid
    package_path: Option<String>,
    versions: Option<VersionInfo>,
    package: Option<PackageInfo>,
    deps: Option<DepsInfo>,
    compat: Option<CompatInfo>,
}

impl Registry {
    pub fn read(config: &Config, name: impl AsRef<str>) -> Result<Self> {
        let data = registry_data(config, "Registry.toml", name)?;
        let registry: Self = toml::from_str(&data)?;
        Ok(registry)
    }

    pub fn package(&self, config: &Config) -> RegistryHandler {
        RegistryHandler {
            registry: self,
            config: config.clone(),
            name: None,
            uuid: None,
            package_path: None,
            versions: None,
            package: None,
            deps: None,
            compat: None,
        }
    }
}

impl<'de> RegistryHandler<'de> {
    pub fn name(&mut self, name: impl AsRef<str>) -> &mut Self {
        self.name = Some(name.as_ref().to_string());
        self
    }

    pub fn uuid(&mut self, uuid: impl AsRef<str>) -> &mut Self {
        self.uuid = Some(uuid.as_ref().to_string());
        self
    }

    pub fn has_package(&mut self) -> bool {
        self.package_path().is_ok()
    }

    pub fn package_path(&mut self) -> Result<String> {
        if self.package_path.is_some() {
            return Ok(self.package_path.as_ref().unwrap().clone());
        }

        let path = if let Some(uuid) = &self.uuid {
            match self.registry.packages.get(uuid) {
                Some(package) => package.path.clone(),
                None => {
                    return Err(format_err!(
                        "Package {} not found in {}",
                        uuid,
                        self.registry.name
                    ));
                }
            }
        } else if let Some(name) = &self.name {
            let mut pkgs = self.registry.packages.values();
            loop {
                let pkginfo = pkgs.next();
                match pkginfo {
                    Some(pkginfo) => {
                        if pkginfo.name == *name {
                            break pkginfo.path.clone();
                        }
                    }
                    None => {
                        return Err(format_err!(
                            "Package {} not found in {}",
                            name,
                            self.registry.name
                        ));
                    }
                }
            }
        } else {
            return Err(format_err!("Package name or uuid not set"));
        };

        self.package_path = Some(path.clone());
        Ok(path)
    }

    pub fn version_info(&mut self) -> Result<&VersionInfo> {
        if self.versions.is_none() {
            let data = &self.registry_data("Versions.toml")?;
            let versions: VersionInfo = toml::from_str(data.as_str())?;
            self.versions = Some(versions);
        }
        Ok(self.versions.as_ref().unwrap())
    }

    pub fn package_info(&mut self) -> Result<&PackageInfo> {
        if self.package.is_none() {
            let data = &self.registry_data("Package.toml")?;
            let package: PackageInfo = toml::from_str(data.as_str())?;
            self.package = Some(package);
        }
        Ok(self.package.as_ref().unwrap())
    }

    pub fn deps_info(&mut self) -> Result<&DepsInfo> {
        if self.deps.is_none() {
            let data = &self.registry_data("Deps.toml")?;
            let deps: DepsInfo = toml::from_str(data.as_str())?;
            self.deps = Some(deps);
        }
        Ok(self.deps.as_ref().unwrap())
    }

    pub fn compat_info(&mut self) -> Result<&CompatInfo> {
        if self.compat.is_none() {
            let data = &self.registry_data("Compat.toml")?;
            let compat: CompatInfo = toml::from_str(data.as_str())?;
            self.compat = Some(compat);
        }
        Ok(self.compat.as_ref().unwrap())
    }

    pub fn get_url(&mut self) -> Result<Url> {
        let package = self.package_info()?;
        Ok(package.repo.clone())
    }

    pub fn get_uuid(&mut self) -> Result<String> {
        if self.uuid.is_none() {
            let package = self.package_info()?;
            self.uuid = Some(package.uuid.clone());
        }
        return Ok(self.uuid.as_ref().unwrap().clone());
    }

    pub fn get_latest_version(&mut self) -> Result<Version> {
        let versions = self.version_info()?;
        Ok(versions.keys().max().unwrap().clone())
    }

    pub fn registry_data(&mut self, name: impl AsRef<str>) -> Result<String> {
        let file = PathBuf::from(self.package_path()?).join(name.as_ref());
        let file = file.to_str().unwrap();
        let data = registry_data(&self.config, file, &self.registry.name)?;
        Ok(data)
    }
}

pub fn registry_data(
    config: &Config,
    file: impl AsRef<str>,
    name: impl AsRef<str>,
) -> Result<String> {
    format!(
        r#"
    using Pkg
    for reg in Pkg.Registry.reachable_registries()
        if reg.name == "{name}"
            data = if isnothing(reg.in_memory_registry)
                read(joinpath(reg.path, "{file}"), String)
            else
                reg.in_memory_registry["{file}"]
            end
            println(data)
            break
        end
    end
    "#,
        file = file.as_ref(),
        name = name.as_ref()
    )
    .as_julia_command(config)?
    .read_command()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registry() {
        let config = Config::default();
        let registry = Registry::read(&config, "General").unwrap();
        let mut handler = registry.package(&config);
        handler.name("Example");
        let url = handler.get_url().unwrap();
        assert_eq!(
            url,
            Url::parse("https://github.com/JuliaLang/Example.jl.git").unwrap()
        );
        let uuid = handler.get_uuid().unwrap();
        assert_eq!(uuid, "7876af07-990d-54b4-ab0e-23690620f79a");
    }

    #[test]
    fn test_registry_data() {
        let config = Config::default();
        let registry = Registry::read(&config, "General").unwrap();
        let mut handler = registry.package(&config);
        handler.name("Example");
        let data = handler.registry_data("Package.toml").unwrap();
        assert!(data.contains("Example"));
    }
}
