use anyhow::Result;
use assert_cmd::{cargo::cargo_bin, Command};
use rexpect::spawn;
use std::{
    env,
    fs::{create_dir_all, remove_dir_all},
    path::PathBuf,
};

fn setup() {
    let root = env::var("CARGO_MANIFEST_DIR").unwrap();
    let scratch = PathBuf::from(root)
        .join("tests")
        .join("packages")
        .join("scratch");
    create_dir_all(scratch.clone()).unwrap();
    env::set_current_dir(scratch).unwrap();
}

#[test]
fn test_cli_example() {
    setup();
    Command::cargo_bin("ion")
        .unwrap()
        .arg("help")
        .assert()
        .success();
    Command::cargo_bin("ion")
        .unwrap()
        .arg("clone")
        .arg("Example")
        .assert()
        .success();
    remove_dir_all("Example").unwrap();
}

#[test]
fn test_new_package() -> Result<()> {
    setup();
    let ion = cargo_bin("ion");
    let program = format!("{} new TestNew -f --template=package", ion.display());
    let mut p = spawn(program.as_str(), Some(30_000))?;
    p.send_line("")?; // authors
    p.send_line("")?; // description
    p.send_line("")?; // license
    p.send_line("")?; // year
    p.exp_eof()?;
    assert!(PathBuf::from("TestNew").join("Project.toml").is_file());
    assert!(PathBuf::from("TestNew").join("LICENSE").is_file());
    assert!(PathBuf::from("TestNew").join("README.md").is_file());
    assert!(PathBuf::from("TestNew").join(".gitignore").is_file());
    assert!(PathBuf::from("TestNew")
        .join("src")
        .join("TestNew.jl")
        .is_file());

    let test_dir = PathBuf::from("TestNew").join("tests");
    assert!(test_dir.join("runtests.jl").is_file());
    assert!(test_dir.join("Project.toml").is_file());

    let docs_dir = PathBuf::from("TestNew").join("docs");
    assert!(docs_dir.join("make.jl").is_file());
    assert!(docs_dir.join("Project.toml").is_file());
    assert!(docs_dir.join("src").join("index.md").is_file());

    let github_dir = PathBuf::from("TestNew").join("github");
    let workflow_dir = github_dir.join("workflows");
    assert!(workflow_dir.join("CI.yml").is_file());
    assert!(workflow_dir.join("CompatHelper.yml").is_file());
    assert!(workflow_dir.join("TagBot.yml").is_file());

    remove_dir_all("TestNew").unwrap();
    Ok(())
}
