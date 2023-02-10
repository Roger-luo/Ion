use anyhow::Result;
use super::*;
use std::{
    fs::remove_dir_all,
    path::PathBuf,
};

#[test]
fn test_new_package() -> Result<()> {
    let mut p = Ion::new()
        .arg("new")
        .arg("TestNew")
        .arg("-f")
        .arg("--template=package")
        .spawn(Some(30_000))?;

    p.send_line("")?; // download
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

    let github_dir = PathBuf::from("TestNew").join(".github");
    let workflow_dir = github_dir.join("workflows");
    println!("workflow_dir: {:?}", workflow_dir.join("CI.yml"));
    assert!(workflow_dir.join("CI.yml").is_file());
    assert!(workflow_dir.join("CompatHelper.yml").is_file());
    assert!(workflow_dir.join("TagBot.yml").is_file());

    remove_dir_all("TestNew").unwrap();
    Ok(())
}
