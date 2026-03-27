use std::process::Command;

fn ion_cmd() -> Command {
    Command::new(env!("CARGO_BIN_EXE_ion"))
}

#[test]
fn init_creates_claude_symlink_when_agents_md_exists() {
    let project = tempfile::tempdir().unwrap();
    std::fs::write(project.path().join("AGENTS.md"), "# My Agents\n").unwrap();

    let output = ion_cmd()
        .args(["init", "--target", "claude"])
        .current_dir(project.path())
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "init failed: stdout={stdout}\nstderr={stderr}"
    );

    let symlink = project.path().join("CLAUDE.md");
    assert!(symlink.exists(), "CLAUDE.md should exist after init");
    assert!(
        symlink.symlink_metadata().unwrap().is_symlink(),
        "CLAUDE.md should be a symlink"
    );
}

#[test]
fn init_no_symlink_without_agents_md() {
    let project = tempfile::tempdir().unwrap();

    let output = ion_cmd()
        .args(["init", "--target", "claude"])
        .current_dir(project.path())
        .output()
        .unwrap();
    assert!(output.status.success());

    assert!(
        !project.path().join("CLAUDE.md").exists(),
        "CLAUDE.md should not exist without AGENTS.md"
    );
}

#[test]
fn install_all_creates_claude_symlink() {
    let project = tempfile::tempdir().unwrap();
    std::fs::write(project.path().join("AGENTS.md"), "# My Agents\n").unwrap();
    std::fs::write(
        project.path().join("Ion.toml"),
        "[skills]\n\n[options.targets]\nclaude = \".claude/skills\"\n",
    )
    .unwrap();

    let output = ion_cmd()
        .args(["add"])
        .current_dir(project.path())
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "add failed: stdout={stdout}\nstderr={stderr}"
    );

    let symlink = project.path().join("CLAUDE.md");
    assert!(symlink.exists(), "CLAUDE.md should exist after install-all");
    assert!(symlink.symlink_metadata().unwrap().is_symlink());
}
