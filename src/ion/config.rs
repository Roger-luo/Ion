use std::path::PathBuf;

pub struct Julia {
    pub exename: PathBuf, // the Julia command path
}

pub struct Config {
    pub julia: Julia,
    pub template: String, // url to the template registry
    pub env: String,      // env directory path
}
