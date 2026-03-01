use std::process::Command;

fn ion_cmd() -> Command {
    Command::new(env!("CARGO_BIN_EXE_ion"))
}

#[test]
fn add_and_remove_local_skill() {
    let project = tempfile::tempdir().unwrap();
    let skill_base = tempfile::tempdir().unwrap();
    let skill_path = skill_base.path().join("test-skill");
    std::fs::create_dir(&skill_path).unwrap();

    // Create a valid skill
    std::fs::write(
        skill_path.join("SKILL.md"),
        "---\nname: test-skill\ndescription: Integration test skill.\nmetadata:\n  version: \"1.0\"\n---\n\n# Test\n\nDo things.\n",
    )
    .unwrap();

    // ion add
    let output = ion_cmd()
        .args(["add", &skill_path.display().to_string()])
        .current_dir(project.path())
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "add failed: stdout={stdout}\nstderr={stderr}"
    );
    assert!(project
        .path()
        .join(".agents/skills/test-skill/SKILL.md")
        .exists());
    assert!(project.path().join("ion.toml").exists());
    assert!(project.path().join("ion.lock").exists());

    // ion list
    let output = ion_cmd()
        .args(["list"])
        .current_dir(project.path())
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(output.status.success());
    assert!(stdout.contains("test-skill"));

    // ion info
    let output = ion_cmd()
        .args(["info", "test-skill"])
        .current_dir(project.path())
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(output.status.success());
    assert!(stdout.contains("Integration test skill"));

    // ion remove
    let output = ion_cmd()
        .args(["remove", "test-skill"])
        .current_dir(project.path())
        .output()
        .unwrap();
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(output.status.success(), "remove failed: {stderr}");
    assert!(!project.path().join(".agents/skills/test-skill").exists());
}

#[test]
fn install_from_manifest() {
    let project = tempfile::tempdir().unwrap();
    let skill_base = tempfile::tempdir().unwrap();
    let skill_path = skill_base.path().join("manifest-skill");
    std::fs::create_dir(&skill_path).unwrap();

    // Create a valid skill
    std::fs::write(
        skill_path.join("SKILL.md"),
        "---\nname: manifest-skill\ndescription: Manifest test.\n---\n\nBody.\n",
    )
    .unwrap();

    // Write ion.toml manually
    std::fs::write(
        project.path().join("ion.toml"),
        format!(
            "[skills]\nmanifest-skill = {{ type = \"path\", source = \"{}\" }}\n",
            skill_path.display()
        ),
    )
    .unwrap();

    // ion install
    let output = ion_cmd()
        .args(["install"])
        .current_dir(project.path())
        .output()
        .unwrap();
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(output.status.success(), "install failed: {stderr}");
    assert!(project
        .path()
        .join(".agents/skills/manifest-skill/SKILL.md")
        .exists());
}

#[test]
fn help_shows_all_commands() {
    let output = ion_cmd().args(["--help"]).output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("add"));
    assert!(stdout.contains("remove"));
    assert!(stdout.contains("install"));
    assert!(stdout.contains("list"));
    assert!(stdout.contains("info"));
}
