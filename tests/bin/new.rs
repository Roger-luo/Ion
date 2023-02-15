use super::*;
use anyhow::Result;

#[test]
fn test_new_package() -> Result<()> {
    if !scratch_dir().exists() {
        std::fs::create_dir_all(scratch_dir())?;
    }

    let mut p = Ion::new()
        .arg("new")
        .arg("TestNew")
        .arg("-f")
        .arg("--template=package")
        .scratch()
        .spawn(Some(30_000))?;

    p.send_line("")?; // download
    p.send_line("")?; // authors
    p.send_line("")?; // description
    p.send_line("")?; // license
    p.send_line("")?; // year
    p.exp_eof()?;

    let project_dir = scratch_dir().join("TestNew");
    assert!(project_dir.join("Project.toml").is_file());
    assert!(project_dir.join("LICENSE").is_file());
    assert!(project_dir.join("README.md").is_file());
    assert!(project_dir.join(".gitignore").is_file());
    assert!(project_dir.join("src").join("TestNew.jl").is_file());

    let test_dir = project_dir.join("test");
    assert!(test_dir.join("runtests.jl").is_file());
    assert!(test_dir.join("Project.toml").is_file());

    let docs_dir = project_dir.join("docs");
    assert!(docs_dir.join("make.jl").is_file());
    assert!(docs_dir.join("Project.toml").is_file());
    assert!(docs_dir.join("src").join("index.md").is_file());

    let github_dir = project_dir.join(".github");
    let workflow_dir = github_dir.join("workflows");
    println!("workflow_dir: {:?}", workflow_dir.join("CI.yml"));
    assert!(workflow_dir.join("CI.yml").is_file());
    assert!(workflow_dir.join("CompatHelper.yml").is_file());
    assert!(workflow_dir.join("TagBot.yml").is_file());
    Ok(())
}
