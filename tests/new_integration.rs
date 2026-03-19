use std::process::Command;

fn ion_cmd() -> Command {
    Command::new(env!("CARGO_BIN_EXE_ion"))
}

#[test]
fn new_help_is_exposed() {
    let output = ion_cmd().args(["skill", "new", "--help"]).output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(output.status.success());
    assert!(stdout.contains("Create a new skill"));
}

#[test]
fn new_creates_skill_md_in_current_dir() {
    let dir = tempfile::tempdir().unwrap();

    let output = ion_cmd()
        .args(["skill", "new"])
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
        .args(["skill", "new", "--path", target.to_str().unwrap()])
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
        .args(["skill", "new"])
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
        .args(["skill", "new", "--force"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(output.status.success(), "stdout={stdout}\nstderr={stderr}");

    let content = std::fs::read_to_string(dir.path().join("SKILL.md")).unwrap();
    assert!(
        content.contains("## Overview"),
        "should have new template content"
    );
    assert!(!content.contains("old content"));
}

#[test]
fn new_bin_creates_cargo_project_and_skill_md() {
    let base = tempfile::tempdir().unwrap();
    let target = base.path().join("my-bin-skill");
    std::fs::create_dir(&target).unwrap();

    let output = ion_cmd()
        .args(["skill", "new", "--bin", "--path", target.to_str().unwrap()])
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(output.status.success(), "stdout={stdout}\nstderr={stderr}");

    assert!(target.join("SKILL.md").exists());
    assert!(target.join("Cargo.toml").exists());
    assert!(target.join("src/main.rs").exists());

    // SKILL.md should have binary metadata
    let skill_content = std::fs::read_to_string(target.join("SKILL.md")).unwrap();
    assert!(skill_content.contains("name: my-bin-skill"));
    assert!(
        skill_content.contains("binary: my-bin-skill"),
        "SKILL.md should have binary metadata"
    );
    assert!(
        skill_content.contains("ion run my-bin-skill"),
        "SKILL.md should reference ion run"
    );

    // Cargo.toml should have clap dependency
    let cargo_content = std::fs::read_to_string(target.join("Cargo.toml")).unwrap();
    assert!(
        cargo_content.contains("clap"),
        "Cargo.toml should have clap dependency"
    );

    // src/main.rs should have self command group with skill subcommand
    let main_content = std::fs::read_to_string(target.join("src/main.rs")).unwrap();
    assert!(
        main_content.contains("SelfCommands"),
        "main.rs should have SelfCommands enum"
    );
    assert!(
        main_content.contains("Skill"),
        "main.rs should have Skill command variant"
    );
    assert!(
        main_content.contains("include_str!"),
        "main.rs should include SKILL.md"
    );
    assert!(
        main_content.contains("SelfManager"),
        "main.rs should use SelfManager from ionlib"
    );

    // Cargo.toml should have ionlib dependency
    assert!(
        cargo_content.contains("ionlib"),
        "Cargo.toml should have ionlib dependency"
    );

    // build.rs should exist for TARGET env var
    assert!(
        target.join("build.rs").exists(),
        "build.rs should exist for TARGET env var"
    );
}

#[test]
fn new_collection_creates_skills_dir_and_readme() {
    let dir = tempfile::tempdir().unwrap();

    let output = ion_cmd()
        .args(["skill", "new", "--collection"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(output.status.success(), "stdout={stdout}\nstderr={stderr}");

    assert!(
        dir.path().join("skills").is_dir(),
        "skills/ directory should be created"
    );
    assert!(
        dir.path().join("README.md").exists(),
        "README.md should be created"
    );

    let readme = std::fs::read_to_string(dir.path().join("README.md")).unwrap();
    assert!(readme.contains("collection of skills"));
    assert!(readme.contains("ion skill new"));
}

#[test]
fn new_collection_with_path_creates_in_specified_dir() {
    let base = tempfile::tempdir().unwrap();
    let target = base.path().join("my-collection");

    let output = ion_cmd()
        .args([
            "skill",
            "new",
            "--collection",
            "--path",
            target.to_str().unwrap(),
        ])
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(output.status.success(), "stdout={stdout}\nstderr={stderr}");

    assert!(target.join("skills").is_dir());
    assert!(target.join("README.md").exists());

    let readme = std::fs::read_to_string(target.join("README.md")).unwrap();
    assert!(readme.contains("My Collection"));
}

#[test]
fn new_collection_errors_if_readme_exists() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("README.md"), "existing readme").unwrap();

    let output = ion_cmd()
        .args(["skill", "new", "--collection"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("already exists"));
    assert!(stderr.contains("--force"));

    let content = std::fs::read_to_string(dir.path().join("README.md")).unwrap();
    assert_eq!(content, "existing readme");
}

#[test]
fn new_collection_force_overwrites_readme() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("README.md"), "old readme").unwrap();

    let output = ion_cmd()
        .args(["skill", "new", "--collection", "--force"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(output.status.success(), "stdout={stdout}\nstderr={stderr}");

    let content = std::fs::read_to_string(dir.path().join("README.md")).unwrap();
    assert!(content.contains("collection of skills"));
    assert!(!content.contains("old readme"));
}

#[test]
fn new_collection_and_bin_errors() {
    let dir = tempfile::tempdir().unwrap();

    let output = ion_cmd()
        .args(["skill", "new", "--collection", "--bin"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Cannot combine"));
}
