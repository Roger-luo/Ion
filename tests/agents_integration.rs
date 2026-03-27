use std::process::Command;

fn ion_cmd() -> Command {
    Command::new(env!("CARGO_BIN_EXE_ion"))
}

#[test]
fn agents_init_from_local_path() {
    let project = tempfile::tempdir().unwrap();
    std::fs::write(project.path().join("Ion.toml"), "[skills]\n").unwrap();

    let template_dir = tempfile::tempdir().unwrap();
    std::fs::write(
        template_dir.path().join("AGENTS.md"),
        "# Org Standard Agents\n\nDo things the org way.\n",
    )
    .unwrap();

    let output = ion_cmd()
        .args(["agents", "init", template_dir.path().to_str().unwrap()])
        .current_dir(project.path())
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "agents init failed: stdout={stdout}\nstderr={stderr}"
    );

    // Should copy AGENTS.md as starting point
    let agents_md = project.path().join("AGENTS.md");
    assert!(agents_md.exists(), "AGENTS.md should be created");
    let content = std::fs::read_to_string(&agents_md).unwrap();
    assert!(content.contains("Org Standard Agents"));

    // Should write [agents] to Ion.toml
    let manifest = std::fs::read_to_string(project.path().join("Ion.toml")).unwrap();
    assert!(manifest.contains("[agents]"));
    assert!(manifest.contains("template"));

    // Should write to Ion.lock
    let lockfile = std::fs::read_to_string(project.path().join("Ion.lock")).unwrap();
    assert!(lockfile.contains("[agents]"));
    assert!(lockfile.contains("checksum"));
}

#[test]
fn agents_init_preserves_existing_agents_md() {
    let project = tempfile::tempdir().unwrap();
    std::fs::write(project.path().join("Ion.toml"), "[skills]\n").unwrap();
    std::fs::write(
        project.path().join("AGENTS.md"),
        "# My Custom Agents\n\nMy local content.\n",
    )
    .unwrap();

    let template_dir = tempfile::tempdir().unwrap();
    std::fs::write(template_dir.path().join("AGENTS.md"), "# Org Template\n").unwrap();

    let output = ion_cmd()
        .args(["agents", "init", template_dir.path().to_str().unwrap()])
        .current_dir(project.path())
        .output()
        .unwrap();
    assert!(output.status.success());

    // Original AGENTS.md should be preserved
    let content = std::fs::read_to_string(project.path().join("AGENTS.md")).unwrap();
    assert!(content.contains("My Custom Agents"));

    // Upstream should be staged
    let upstream = project.path().join(".agents/templates/AGENTS.md.upstream");
    assert!(upstream.exists());
}

#[test]
fn agents_init_errors_when_already_configured() {
    let project = tempfile::tempdir().unwrap();
    std::fs::write(
        project.path().join("Ion.toml"),
        "[skills]\n\n[agents]\ntemplate = \"org/old\"\n",
    )
    .unwrap();

    let template_dir = tempfile::tempdir().unwrap();
    std::fs::write(template_dir.path().join("AGENTS.md"), "# New\n").unwrap();

    let output = ion_cmd()
        .args(["agents", "init", template_dir.path().to_str().unwrap()])
        .current_dir(project.path())
        .output()
        .unwrap();
    assert!(
        !output.status.success(),
        "should error when already configured"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("already configured"));
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
