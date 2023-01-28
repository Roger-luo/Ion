use serde_derive::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Author {
    pub firstname: String,
    pub lastname: Option<String>,
    pub email: Option<String>,
    pub url: Option<String>,
    pub affiliation: Option<String>,
    pub orcid: Option<String>,
}
