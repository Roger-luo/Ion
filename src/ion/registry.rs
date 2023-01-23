use url::Url;
use std::collections::BTreeMap;
use serde_derive::{Serialize, Deserialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct PackageInfo {
    pub name: String,
    pub path: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Registry {
    pub name: String,
    pub repo: Url,
    pub uuid: String,
    pub description: String,
    pub packages: BTreeMap<String, PackageInfo>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RegistryList {
    pub registry: Vec<Registry>,
}
