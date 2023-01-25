use anyhow::Result;
use ion::utils::*;
use std::env::*;
use std::path::PathBuf;

#[test]
fn test_current_project() -> Result<()> {
    let root_dir = PathBuf::from(var("CARGO_MANIFEST_DIR").unwrap());
    let tests_dir = root_dir.join("tests");
    let package_dir = tests_dir.join("packages");
    let path = package_dir.join("A").join("B").join("C");
    std::env::set_current_dir(path).unwrap();

    let cwd = current_dir().unwrap();
    let project = current_project(cwd.to_owned()).unwrap();
    assert_eq!(
        project,
        package_dir.join("A").join("B").join("Project.toml")
    );

    let (toml, path) = current_root_project(cwd.to_owned()).unwrap();
    assert_eq!(path, package_dir.join("A").join("Project.toml"));
    assert_eq!(toml.name.unwrap(), "A");
    Ok(())
}
