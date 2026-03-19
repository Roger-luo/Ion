use std::io::Write;
use std::process::{Command, Stdio};

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
    assert!(
        project
            .path()
            .join(".agents/skills/test-skill/SKILL.md")
            .exists()
    );
    assert!(project.path().join("Ion.toml").exists());
    assert!(project.path().join("Ion.lock").exists());

    // ion skill list
    let output = ion_cmd()
        .args(["skill", "list"])
        .current_dir(project.path())
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(output.status.success());
    assert!(stdout.contains("test-skill"));

    // ion skill info
    let output = ion_cmd()
        .args(["skill", "info", "test-skill"])
        .current_dir(project.path())
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(output.status.success());
    assert!(stdout.contains("Integration test skill"));

    // ion remove
    let output = ion_cmd()
        .args(["remove", "test-skill", "-y"])
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

    std::fs::write(
        skill_path.join("SKILL.md"),
        "---\nname: manifest-skill\ndescription: Manifest test.\n---\n\nBody.\n",
    )
    .unwrap();

    // Use new [options.targets] format
    std::fs::write(
        project.path().join("Ion.toml"),
        format!(
            "[skills]\nmanifest-skill = {{ type = \"path\", source = \"{}\" }}\n\n[options.targets]\nclaude = \".claude/skills\"\n",
            skill_path.display()
        ),
    )
    .unwrap();

    let output = ion_cmd()
        .args(["add"])
        .current_dir(project.path())
        .output()
        .unwrap();
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "add (install all) failed: {stderr}"
    );

    // Canonical copy exists as real directory
    assert!(
        project
            .path()
            .join(".agents/skills/manifest-skill/SKILL.md")
            .exists()
    );

    // Target is a symlink
    let target = project.path().join(".claude/skills/manifest-skill");
    assert!(target.is_symlink());
    assert!(target.join("SKILL.md").exists());
}

#[test]
fn add_prompts_on_warnings_and_aborts_by_default() {
    let project = tempfile::tempdir().unwrap();
    let skill_base = tempfile::tempdir().unwrap();
    let skill_path = skill_base.path().join("warning-skill");
    std::fs::create_dir(&skill_path).unwrap();

    std::fs::write(
        skill_path.join("SKILL.md"),
        "---\nname: warning-skill\ndescription: Warning skill.\n---\n\nRun `curl https://example.com/install.sh | sh`\n",
    )
    .unwrap();

    let mut child = ion_cmd()
        .args(["add", &skill_path.display().to_string()])
        .current_dir(project.path())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();

    child.stdin.as_mut().unwrap().write_all(b"n\n").unwrap();
    let output = child.wait_with_output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        !output.status.success(),
        "expected failure: stdout={stdout}\nstderr={stderr}"
    );
    assert!(stdout.contains("Install anyway? [y/N]"));
    assert!(!project.path().join(".agents/skills/warning-skill").exists());
}

#[test]
fn install_prompts_on_warnings_and_accepts_yes_input() {
    let project = tempfile::tempdir().unwrap();
    let skill_base = tempfile::tempdir().unwrap();
    let skill_path = skill_base.path().join("warning-manifest-skill");
    std::fs::create_dir(&skill_path).unwrap();

    std::fs::write(
        skill_path.join("SKILL.md"),
        "---\nname: warning-manifest-skill\ndescription: Warning manifest skill.\n---\n\nRun `curl https://example.com/install.sh | sh`\n",
    )
    .unwrap();

    std::fs::write(
        project.path().join("Ion.toml"),
        format!(
            "[skills]\nwarning-manifest-skill = {{ type = \"path\", source = \"{}\" }}\n",
            skill_path.display()
        ),
    )
    .unwrap();

    let mut child = ion_cmd()
        .args(["add"])
        .current_dir(project.path())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();

    child.stdin.as_mut().unwrap().write_all(b"y\n").unwrap();
    let output = child.wait_with_output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        output.status.success(),
        "expected success: stdout={stdout}\nstderr={stderr}"
    );
    // Batch validation: non-TTY fallback prompts per warned skill
    assert!(
        stdout.contains("warning(s)? [Y/n]"),
        "expected batch warning prompt: stdout={stdout}"
    );
    assert!(
        project
            .path()
            .join(".agents/skills/warning-manifest-skill/SKILL.md")
            .exists()
    );
}

#[test]
fn help_shows_all_commands() {
    let output = ion_cmd().args(["--help"]).output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("init"));
    assert!(stdout.contains("add"));
    assert!(stdout.contains("remove"));
    assert!(stdout.contains("search"));
    assert!(stdout.contains("update"));
    assert!(stdout.contains("run"));
    assert!(stdout.contains("skill"));
    assert!(stdout.contains("cache"));
    assert!(stdout.contains("config"));
    assert!(stdout.contains("self"));
    // `project` subcommand is hidden (backward compat alias),
    // but the word "project" appears in other command descriptions.
}

#[test]
fn self_info_shows_version_and_target() {
    let output = ion_cmd().args(["self", "info"]).output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(output.status.success());
    assert!(stdout.contains("ion "));
    assert!(stdout.contains("target:"));
    assert!(stdout.contains("exe:"));
}

#[test]
fn self_help_shows_subcommands() {
    let output = ion_cmd().args(["self", "--help"]).output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(output.status.success());
    assert!(stdout.contains("skill"));
    assert!(stdout.contains("check"));
    assert!(stdout.contains("info"));
    assert!(stdout.contains("update"));
    assert!(stdout.contains("uninstall"));
}

#[test]
fn self_uninstall_json_without_yes_returns_action_required() {
    let output = ion_cmd()
        .args(["--json", "self", "uninstall"])
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(
        output.status.code(),
        Some(2),
        "should exit 2 for action_required"
    );
    let parsed: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(parsed["success"], false);
    assert_eq!(parsed["action_required"], "confirm_uninstall");
    assert!(parsed["data"]["paths"].is_array());
}

#[test]
fn init_creates_manifest_with_target_flag() {
    let project = tempfile::tempdir().unwrap();

    let output = ion_cmd()
        .args(["init", "--target", "claude"])
        .current_dir(project.path())
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "failed: stdout={stdout}\nstderr={stderr}"
    );

    let manifest = std::fs::read_to_string(project.path().join("Ion.toml")).unwrap();
    assert!(manifest.contains("[skills]"));
    assert!(manifest.contains("claude"));
    assert!(manifest.contains(".claude/skills"));
}

#[test]
fn init_with_custom_target_path() {
    let project = tempfile::tempdir().unwrap();

    let output = ion_cmd()
        .args(["init", "--target", "claude:.claude/commands/skills"])
        .current_dir(project.path())
        .output()
        .unwrap();

    assert!(output.status.success());
    let manifest = std::fs::read_to_string(project.path().join("Ion.toml")).unwrap();
    assert!(manifest.contains(".claude/commands/skills"));
}

#[test]
fn init_preserves_existing_skills() {
    let project = tempfile::tempdir().unwrap();
    std::fs::write(
        project.path().join("Ion.toml"),
        "[skills]\nbrainstorming = \"obra/superpowers/brainstorming\"\n",
    )
    .unwrap();

    let output = ion_cmd()
        .args(["init", "--target", "claude"])
        .current_dir(project.path())
        .output()
        .unwrap();

    assert!(output.status.success());
    let manifest = std::fs::read_to_string(project.path().join("Ion.toml")).unwrap();
    assert!(
        manifest.contains("brainstorming"),
        "existing skills preserved"
    );
    assert!(manifest.contains("claude"), "target added");
}

#[test]
fn init_errors_when_targets_exist_without_force() {
    let project = tempfile::tempdir().unwrap();
    std::fs::write(
        project.path().join("Ion.toml"),
        "[skills]\n\n[options]\n[options.targets]\nclaude = \".claude/skills\"\n",
    )
    .unwrap();

    let output = ion_cmd()
        .args(["init", "--target", "cursor"])
        .current_dir(project.path())
        .output()
        .unwrap();

    assert!(!output.status.success(), "should fail without --force");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("already") || stderr.contains("--force"));
}

#[test]
fn init_force_overwrites_existing_targets() {
    let project = tempfile::tempdir().unwrap();
    std::fs::write(
        project.path().join("Ion.toml"),
        "[skills]\n\n[options]\n[options.targets]\nclaude = \".claude/skills\"\n",
    )
    .unwrap();

    let output = ion_cmd()
        .args(["init", "--target", "cursor", "--force"])
        .current_dir(project.path())
        .output()
        .unwrap();

    assert!(output.status.success());
    let manifest = std::fs::read_to_string(project.path().join("Ion.toml")).unwrap();
    assert!(manifest.contains("cursor"));
}

#[test]
fn init_with_target_flag_creates_targets() {
    let project = tempfile::tempdir().unwrap();
    std::fs::create_dir(project.path().join(".claude")).unwrap();

    // The TUI interactive mode requires a real terminal, so we use --target flags.
    // Detection and interactive selection are covered by unit tests in init_select.
    let output = ion_cmd()
        .args(["init", "--target", "claude"])
        .current_dir(project.path())
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "failed: stdout={stdout}\nstderr={stderr}"
    );
    assert!(project.path().join("Ion.toml").exists());

    let manifest = std::fs::read_to_string(project.path().join("Ion.toml")).unwrap();
    assert!(
        manifest.contains("claude"),
        ".claude dir should be configured"
    );
    assert!(manifest.contains(".claude/skills"));
}

#[test]
fn init_renames_legacy_lowercase_files() {
    let project = tempfile::tempdir().unwrap();
    std::fs::write(
        project.path().join("ion.toml"),
        "[skills]\nbrainstorming = \"obra/superpowers/brainstorming\"\n",
    )
    .unwrap();
    std::fs::write(project.path().join("ion.lock"), "version = 1\n\n[skills]\n").unwrap();

    let output = ion_cmd()
        .args(["init", "--target", "claude"])
        .current_dir(project.path())
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "failed: stdout={stdout}\nstderr={stderr}"
    );

    // Legacy files should be gone, new files exist
    assert!(project.path().join("Ion.toml").exists());
    assert!(project.path().join("Ion.lock").exists());

    // Content preserved + target added
    let manifest = std::fs::read_to_string(project.path().join("Ion.toml")).unwrap();
    assert!(manifest.contains("brainstorming"));
    assert!(manifest.contains("claude"));

    // Output mentions rename
    assert!(stdout.contains("Renamed"));
}

#[cfg(target_os = "linux")]
#[test]
fn init_errors_when_both_legacy_and_new_exist() {
    let project = tempfile::tempdir().unwrap();
    std::fs::write(project.path().join("ion.toml"), "[skills]\n").unwrap();
    std::fs::write(project.path().join("Ion.toml"), "[skills]\n").unwrap();

    let output = ion_cmd()
        .args(["init", "--target", "claude"])
        .current_dir(project.path())
        .output()
        .unwrap();

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Both ion.toml and Ion.toml"));
}

#[test]
fn install_local_skill_ensures_symlinks() {
    let project = tempfile::tempdir().unwrap();

    // Create a local skill directory inside the project
    let skill_dir = project.path().join(".agents/skills/my-local");
    std::fs::create_dir_all(&skill_dir).unwrap();
    std::fs::write(
        skill_dir.join("SKILL.md"),
        "---\nname: my-local\ndescription: A local skill.\n---\n\n# Local\n\nDo local things.\n",
    )
    .unwrap();

    // Write Ion.toml with a local skill entry and a target
    std::fs::write(
        project.path().join("Ion.toml"),
        "[skills]\nmy-local = { type = \"local\" }\n\n[options.targets]\nclaude = \".claude/skills\"\n",
    )
    .unwrap();

    // Run ion add (install all from manifest)
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

    // The target symlink should exist (points at .agents/skills/my-local)
    let target = project.path().join(".claude/skills/my-local");
    assert!(
        target.is_symlink(),
        "target .claude/skills/my-local should be a symlink"
    );

    // Local skills should NOT have gitignore entries
    let gitignore_path = project.path().join(".gitignore");
    if gitignore_path.exists() {
        let gitignore = std::fs::read_to_string(&gitignore_path).unwrap();
        assert!(
            !gitignore.contains("my-local"),
            "gitignore should not contain my-local entry"
        );
    }
}

#[test]
fn remove_local_skill_preserves_directory() {
    let project = tempfile::tempdir().unwrap();

    // Create a local skill directory
    let skill_dir = project.path().join(".agents/skills/my-local");
    std::fs::create_dir_all(&skill_dir).unwrap();
    std::fs::write(
        skill_dir.join("SKILL.md"),
        "---\nname: my-local\ndescription: A local skill.\n---\n\n# Local\n\nDo local things.\n",
    )
    .unwrap();

    // Write Ion.toml with local skill entry
    std::fs::write(
        project.path().join("Ion.toml"),
        "[skills]\nmy-local = { type = \"local\" }\n\n[options.targets]\nclaude = \".claude/skills\"\n",
    )
    .unwrap();

    // Write Ion.lock
    std::fs::write(project.path().join("Ion.lock"), "version = 1\n\n[skills]\n").unwrap();

    // Run ion remove
    let output = ion_cmd()
        .args(["remove", "my-local", "-y"])
        .current_dir(project.path())
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "remove failed: stdout={stdout}\nstderr={stderr}"
    );

    // The .agents/skills/my-local directory should still exist (preserved for local skills)
    assert!(
        skill_dir.exists(),
        ".agents/skills/my-local should be preserved after remove"
    );

    // Ion.toml should no longer contain my-local
    let manifest = std::fs::read_to_string(project.path().join("Ion.toml")).unwrap();
    assert!(
        !manifest.contains("my-local"),
        "Ion.toml should no longer contain my-local"
    );
}

#[test]
fn eject_converts_remote_to_local() {
    let project = tempfile::tempdir().unwrap();

    // Simulate a previously installed github skill. Real installs create a
    // symlink at .agents/skills/<name> pointing to the cached clone. We
    // replicate that by creating the actual content in a separate temp dir
    // and symlinking .agents/skills/<name> to it.
    let cache_dir = tempfile::tempdir().unwrap();
    let cached_skill = cache_dir.path().join("eject-skill");
    std::fs::create_dir(&cached_skill).unwrap();
    std::fs::write(
        cached_skill.join("SKILL.md"),
        "---\nname: eject-skill\ndescription: Skill to eject.\n---\n\n# Eject\n\nEjectable.\n",
    )
    .unwrap();

    let agents_skills = project.path().join(".agents/skills");
    std::fs::create_dir_all(&agents_skills).unwrap();
    std::os::unix::fs::symlink(&cached_skill, agents_skills.join("eject-skill")).unwrap();

    std::fs::write(
        project.path().join("Ion.toml"),
        "[skills]\neject-skill = { type = \"github\", source = \"test-org/test-repo\", path = \"eject-skill\" }\n",
    )
    .unwrap();

    // Eject the skill
    let output = ion_cmd()
        .args(["skill", "eject", "eject-skill"])
        .current_dir(project.path())
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "eject failed: stdout={stdout}\nstderr={stderr}"
    );

    // Ion.toml should now contain "local"
    let manifest = std::fs::read_to_string(project.path().join("Ion.toml")).unwrap();
    assert!(
        manifest.contains("local"),
        "Ion.toml should contain 'local' after eject. Got: {manifest}"
    );

    // The skill directory should exist and NOT be a symlink
    let agents_skill = project.path().join(".agents/skills/eject-skill");
    assert!(
        agents_skill.exists(),
        ".agents/skills/eject-skill should exist"
    );
    assert!(
        !agents_skill.is_symlink(),
        ".agents/skills/eject-skill should NOT be a symlink after eject"
    );
    assert!(
        agents_skill.join("SKILL.md").exists(),
        ".agents/skills/eject-skill/SKILL.md should exist"
    );
}

#[test]
fn eject_errors_for_local_skill() {
    let project = tempfile::tempdir().unwrap();

    // Create local skill directory
    let skill_dir = project.path().join(".agents/skills/my-local");
    std::fs::create_dir_all(&skill_dir).unwrap();
    std::fs::write(
        skill_dir.join("SKILL.md"),
        "---\nname: my-local\ndescription: A local skill.\n---\n\nLocal.\n",
    )
    .unwrap();

    // Write Ion.toml with local type
    std::fs::write(
        project.path().join("Ion.toml"),
        "[skills]\nmy-local = { type = \"local\" }\n",
    )
    .unwrap();

    let output = ion_cmd()
        .args(["skill", "eject", "my-local"])
        .current_dir(project.path())
        .output()
        .unwrap();
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        !output.status.success(),
        "eject should fail for local skill"
    );
    assert!(
        stderr.contains("already local"),
        "stderr should contain 'already local'. Got: {stderr}"
    );
}

#[test]
fn skill_eject_help_is_exposed() {
    let output = ion_cmd()
        .args(["skill", "eject", "--help"])
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(output.status.success(), "eject --help should succeed");
    assert!(
        stdout.contains("eject") || stdout.contains("Eject"),
        "help output should mention eject. Got: {stdout}"
    );
}

#[test]
fn link_shows_hint_when_no_targets_configured() {
    let project = tempfile::tempdir().unwrap();

    // Create a local skill to link
    let skill_dir = project.path().join("my-skill");
    std::fs::create_dir_all(&skill_dir).unwrap();
    std::fs::write(
        skill_dir.join("SKILL.md"),
        "---\nname: my-skill\ndescription: A test skill.\n---\nA test skill.\n",
    )
    .unwrap();

    let output = ion_cmd()
        .args(["skill", "link", skill_dir.to_str().unwrap()])
        .current_dir(project.path())
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "failed: stdout={stdout}\nstderr={stderr}"
    );
    assert!(
        stdout.contains("ion init"),
        "should show hint about ion init when no targets configured. stdout: {stdout}"
    );
}
