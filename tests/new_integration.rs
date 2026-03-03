use std::process::Command;

fn ion_cmd() -> Command {
    Command::new(env!("CARGO_BIN_EXE_ion"))
}

#[test]
fn new_help_is_exposed() {
    let output = ion_cmd().args(["new", "--help"]).output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(output.status.success());
    assert!(stdout.contains("Create a new skill"));
}

#[test]
fn new_creates_skill_md_in_current_dir() {
    let dir = tempfile::tempdir().unwrap();

    let output = ion_cmd()
        .args(["new"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(output.status.success(), "stdout={stdout}\nstderr={stderr}");

    let skill_md = dir.path().join("SKILL.md");
    assert!(skill_md.exists(), "SKILL.md should be created");

    let content = std::fs::read_to_string(&skill_md).unwrap();
    assert!(content.contains("name:"));
    assert!(content.contains("description:"));
    assert!(content.contains("## Overview"));
}

#[test]
fn new_with_path_creates_skill_md_in_specified_dir() {
    let base = tempfile::tempdir().unwrap();
    let target = base.path().join("my-new-skill");

    let output = ion_cmd()
        .args(["new", "--path", target.to_str().unwrap()])
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(output.status.success(), "stdout={stdout}\nstderr={stderr}");

    let skill_md = target.join("SKILL.md");
    assert!(skill_md.exists(), "SKILL.md should be created at --path");

    let content = std::fs::read_to_string(&skill_md).unwrap();
    assert!(content.contains("name: my-new-skill"));
}

#[test]
fn new_errors_if_skill_md_exists() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("SKILL.md"), "existing content").unwrap();

    let output = ion_cmd()
        .args(["new"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("already exists"));
    assert!(stderr.contains("--force"));

    let content = std::fs::read_to_string(dir.path().join("SKILL.md")).unwrap();
    assert_eq!(content, "existing content");
}

#[test]
fn new_force_overwrites_existing_skill_md() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("SKILL.md"), "old content").unwrap();

    let output = ion_cmd()
        .args(["new", "--force"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(output.status.success(), "stdout={stdout}\nstderr={stderr}");

    let content = std::fs::read_to_string(dir.path().join("SKILL.md")).unwrap();
    assert!(content.contains("## Overview"), "should have new template content");
    assert!(!content.contains("old content"));
}

#[test]
fn new_bin_creates_cargo_project_and_skill_md() {
    let base = tempfile::tempdir().unwrap();
    let target = base.path().join("my-bin-skill");
    std::fs::create_dir(&target).unwrap();

    let output = ion_cmd()
        .args(["new", "--bin", "--path", target.to_str().unwrap()])
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(output.status.success(), "stdout={stdout}\nstderr={stderr}");

    assert!(target.join("SKILL.md").exists());
    assert!(target.join("Cargo.toml").exists());
    assert!(target.join("src/main.rs").exists());

    let content = std::fs::read_to_string(target.join("SKILL.md")).unwrap();
    assert!(content.contains("name: my-bin-skill"));
}
