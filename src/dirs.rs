use std::path::PathBuf;

pub fn resources_dir() -> PathBuf {
    if cfg!(debug_assertions) {
        let mut template = PathBuf::new();
        template.push(std::env::var("CARGO_MANIFEST_DIR").unwrap());
        template.push("resources");
        template
    } else {
        let mut template = PathBuf::new();
        template.push(dirs::config_dir().unwrap());
        template.push("ion");
        template
    }
}

pub fn components_dir() -> PathBuf {
    let path = resources_dir();
    path.join("components")
}

pub fn template_dir() -> PathBuf {
    let path = resources_dir();
    path.join("templates")
}
