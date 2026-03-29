//! Tests for ion CLI behavior across different project setups.
//!
//! Uses the `scenario` crate's `Project::from_template()` to create
//! reproducible project fixtures, then runs ion commands against them
//! to verify behavior under various configurations.

use std::time::Duration;

use scenario::{Project, Scenario, Terminal};

const ION: &str = env!("CARGO_BIN_EXE_ion");

fn ion() -> Scenario {
    Scenario::new(ION).timeout(Duration::from_secs(10))
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// Empty project
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

#[test]
fn empty_project_skill_list_is_empty() {
    let project = Project::from_template("tests/fixtures/empty-project")
        .build()
        .unwrap();
    let output = ion()
        .args(["skill", "list"])
        .project(&project)
        .run()
        .unwrap();
    assert!(output.success());
}

#[test]
fn empty_project_skill_list_json_returns_empty_array() {
    let project = Project::from_template("tests/fixtures/empty-project")
        .build()
        .unwrap();
    let output = ion()
        .args(["--json", "skill", "list"])
        .project(&project)
        .run()
        .unwrap();
    assert!(output.success());
    let json: serde_json::Value = serde_json::from_str(output.stdout()).unwrap();
    assert_eq!(json["success"], true);
    assert_eq!(json["data"], serde_json::json!([]));
}

#[test]
fn empty_project_update_reports_no_skills() {
    let project = Project::from_template("tests/fixtures/empty-project")
        .build()
        .unwrap();
    let output = ion().args(["update"]).project(&project).run().unwrap();
    assert!(output.success());
    assert!(output.stdout().contains("No skills to update"));
}

#[test]
fn empty_project_validate_finds_nothing() {
    let project = Project::from_template("tests/fixtures/empty-project")
        .build()
        .unwrap();
    let output = ion()
        .args(["skill", "validate"])
        .project(&project)
        .run()
        .unwrap();
    assert!(output.success());
    assert!(output.stdout().contains("No SKILL.md files found"));
}

#[test]
fn empty_project_remove_nonexistent_fails() {
    let project = Project::from_template("tests/fixtures/empty-project")
        .build()
        .unwrap();
    let output = ion()
        .args(["remove", "--yes", "nonexistent"])
        .project(&project)
        .run()
        .unwrap();
    assert!(!output.success());
    assert_eq!(output.exit_code(), 1);
}

#[test]
fn empty_project_init_force_succeeds() {
    let project = Project::from_template("tests/fixtures/empty-project")
        .build()
        .unwrap();
    let output = ion()
        .args(["init", "-t", "claude", "--force"])
        .project(&project)
        .run()
        .unwrap();
    assert!(output.success());
    let toml = std::fs::read_to_string(project.path().join("Ion.toml")).unwrap();
    assert!(toml.contains("claude"));
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// Single skill project
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

#[test]
fn single_skill_list_finds_skill() {
    let project = Project::from_template("tests/fixtures/single-skill")
        .build()
        .unwrap();
    let output = ion()
        .args(["skill", "list"])
        .project(&project)
        .run()
        .unwrap();
    assert!(output.success());
    assert!(
        output.stdout().contains("test-skill"),
        "should list the default skill name, got: {}",
        output.stdout()
    );
}

#[test]
fn single_skill_list_with_custom_name() {
    let project = Project::from_template("tests/fixtures/single-skill")
        .var("name", "custom-skill")
        .var("description", "A custom skill")
        .build()
        .unwrap();
    let output = ion()
        .args(["skill", "list"])
        .project(&project)
        .run()
        .unwrap();
    assert!(output.success());
    assert!(
        output.stdout().contains("custom-skill"),
        "should list the custom skill name, got: {}",
        output.stdout()
    );
}

#[test]
fn single_skill_json_list_has_one_entry() {
    let project = Project::from_template("tests/fixtures/single-skill")
        .build()
        .unwrap();
    let output = ion()
        .args(["--json", "skill", "list"])
        .project(&project)
        .run()
        .unwrap();
    assert!(output.success());
    let json: serde_json::Value = serde_json::from_str(output.stdout()).unwrap();
    let skills = json["data"].as_array().unwrap();
    assert_eq!(skills.len(), 1);
    assert_eq!(skills[0]["name"], "test-skill");
}

#[test]
fn single_skill_validate_passes() {
    let project = Project::from_template("tests/fixtures/single-skill")
        .build()
        .unwrap();
    let skill_path = project
        .path()
        .join(".agents")
        .join("skills")
        .join("test-skill");
    let output = ion()
        .args(["skill", "validate", &skill_path.display().to_string()])
        .project(&project)
        .run()
        .unwrap();
    assert!(output.success());
}

#[test]
fn single_skill_validate_json_structure() {
    let project = Project::from_template("tests/fixtures/single-skill")
        .build()
        .unwrap();
    let skill_path = project
        .path()
        .join(".agents")
        .join("skills")
        .join("test-skill");
    let output = ion()
        .args([
            "--json",
            "skill",
            "validate",
            &skill_path.display().to_string(),
        ])
        .project(&project)
        .run()
        .unwrap();
    assert!(output.success());
    let json: serde_json::Value = serde_json::from_str(output.stdout()).unwrap();
    assert_eq!(json["success"], true);
    assert!(json["data"]["skills"].is_array());
    assert_eq!(json["data"]["total_errors"], 0);
}

#[test]
fn single_skill_remove_with_yes_flag() {
    let project = Project::from_template("tests/fixtures/single-skill")
        .build()
        .unwrap();
    let output = ion()
        .args(["remove", "--yes", "test-skill"])
        .project(&project)
        .run()
        .unwrap();
    assert!(output.success());
    let manifest = std::fs::read_to_string(project.path().join("Ion.toml")).unwrap();
    assert!(
        !manifest.contains("test-skill"),
        "skill should be removed from Ion.toml"
    );
}

#[test]
fn single_skill_remove_piped_stdin_confirm() {
    let project = Project::from_template("tests/fixtures/single-skill")
        .build()
        .unwrap();
    let output = ion()
        .args(["remove", "test-skill"])
        .stdin(b"y\n".to_vec())
        .project(&project)
        .run()
        .unwrap();
    assert!(output.success());
}

#[test]
fn single_skill_remove_piped_stdin_reject() {
    let project = Project::from_template("tests/fixtures/single-skill")
        .build()
        .unwrap();
    let output = ion()
        .args(["remove", "test-skill"])
        .stdin(b"n\n".to_vec())
        .project(&project)
        .run()
        .unwrap();
    assert!(!output.success());
    let manifest = std::fs::read_to_string(project.path().join("Ion.toml")).unwrap();
    assert!(
        manifest.contains("test-skill"),
        "skill should still be in Ion.toml after rejection"
    );
}

#[test]
fn single_skill_remove_interactive_pty() {
    let project = Project::from_template("tests/fixtures/single-skill")
        .build()
        .unwrap();
    let mut session = ion()
        .args(["remove", "test-skill"])
        .project(&project)
        .terminal(Terminal::pty(80, 24))
        .spawn()
        .unwrap();
    session.expect("Proceed?").unwrap();
    session.send_line("y").unwrap();
    let output = session.wait().unwrap();
    assert!(output.success());
}

#[test]
fn single_skill_update_skips_local() {
    let project = Project::from_template("tests/fixtures/single-skill")
        .build()
        .unwrap();
    let output = ion().args(["update"]).project(&project).run().unwrap();
    // Local skills should be skipped during update
    assert!(output.success());
}

#[test]
fn single_skill_update_json_skips_local() {
    let project = Project::from_template("tests/fixtures/single-skill")
        .build()
        .unwrap();
    let output = ion()
        .args(["--json", "update"])
        .project(&project)
        .run()
        .unwrap();
    assert!(output.success());
    let json: serde_json::Value = serde_json::from_str(output.stdout()).unwrap();
    assert_eq!(json["success"], true);
    let data = &json["data"];
    assert!(data["updated"].as_array().unwrap().is_empty());
}

#[test]
fn single_skill_pty_list_has_color() {
    let project = Project::from_template("tests/fixtures/single-skill")
        .build()
        .unwrap();
    let output = ion()
        .args(["skill", "list"])
        .project(&project)
        .terminal(Terminal::pty(80, 24))
        .run()
        .unwrap();
    assert!(output.success());
    assert!(output.stdout().contains("test-skill"));
    assert!(
        output.stdout_raw().contains(&0x1B),
        "PTY output should contain ANSI color codes"
    );
}

#[test]
fn single_skill_piped_list_no_color() {
    let project = Project::from_template("tests/fixtures/single-skill")
        .build()
        .unwrap();
    let output = ion()
        .args(["skill", "list"])
        .project(&project)
        .run()
        .unwrap();
    assert!(output.success());
    assert!(
        !output.stdout_raw().contains(&0x1B),
        "piped output should NOT contain ANSI codes"
    );
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// Multi-skill project
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

#[test]
fn multi_skill_list_finds_all_skills() {
    let project = Project::from_template("tests/fixtures/multi-skill")
        .build()
        .unwrap();
    let output = ion()
        .args(["skill", "list"])
        .project(&project)
        .run()
        .unwrap();
    assert!(output.success());
    let stdout = output.stdout();
    assert!(
        stdout.contains("alpha-skill"),
        "should list alpha-skill, got: {stdout}"
    );
    assert!(
        stdout.contains("beta-skill"),
        "should list beta-skill, got: {stdout}"
    );
}

#[test]
fn multi_skill_json_list_has_two_entries() {
    let project = Project::from_template("tests/fixtures/multi-skill")
        .build()
        .unwrap();
    let output = ion()
        .args(["--json", "skill", "list"])
        .project(&project)
        .run()
        .unwrap();
    assert!(output.success());
    let json: serde_json::Value = serde_json::from_str(output.stdout()).unwrap();
    let skills = json["data"].as_array().unwrap();
    assert_eq!(skills.len(), 2, "should have 2 skills, got: {skills:?}");
}

#[test]
fn multi_skill_remove_one_keeps_other() {
    let project = Project::from_template("tests/fixtures/multi-skill")
        .build()
        .unwrap();
    let output = ion()
        .args(["remove", "--yes", "alpha-skill"])
        .project(&project)
        .run()
        .unwrap();
    assert!(output.success());
    let manifest = std::fs::read_to_string(project.path().join("Ion.toml")).unwrap();
    assert!(!manifest.contains("alpha-skill"), "alpha should be removed");
    assert!(manifest.contains("beta-skill"), "beta should remain");
}

#[test]
fn multi_skill_validate_all() {
    let project = Project::from_template("tests/fixtures/multi-skill")
        .build()
        .unwrap();
    let output = ion()
        .args(["skill", "validate"])
        .project(&project)
        .run()
        .unwrap();
    assert!(
        output.success(),
        "all skills should pass validation, got: {}{}",
        output.stdout(),
        output.stderr()
    );
}

#[test]
fn multi_skill_validate_json_reports_both() {
    let project = Project::from_template("tests/fixtures/multi-skill")
        .build()
        .unwrap();
    let output = ion()
        .args(["--json", "skill", "validate"])
        .project(&project)
        .run()
        .unwrap();
    let json: serde_json::Value = serde_json::from_str(output.stdout()).unwrap();
    let skills = json["data"]["skills"].as_array().unwrap();
    assert!(
        skills.len() >= 2,
        "should validate at least 2 skills, got: {skills:?}"
    );
}

#[test]
fn multi_skill_with_custom_names() {
    let project = Project::from_template("tests/fixtures/multi-skill")
        .var("alpha_name", "first-skill")
        .var("beta_name", "second-skill")
        .build()
        .unwrap();
    let output = ion()
        .args(["--json", "skill", "list"])
        .project(&project)
        .run()
        .unwrap();
    assert!(output.success());
    let json: serde_json::Value = serde_json::from_str(output.stdout()).unwrap();
    let skills = json["data"].as_array().unwrap();
    let names: Vec<&str> = skills.iter().filter_map(|s| s["name"].as_str()).collect();
    assert!(
        names.contains(&"first-skill"),
        "should have first-skill, got: {names:?}"
    );
    assert!(
        names.contains(&"second-skill"),
        "should have second-skill, got: {names:?}"
    );
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// Project with targets (symlinks)
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

#[test]
fn with_targets_skill_list_works() {
    let project = Project::from_template("tests/fixtures/with-targets")
        .build()
        .unwrap();
    let output = ion()
        .args(["skill", "list"])
        .project(&project)
        .run()
        .unwrap();
    assert!(output.success());
    assert!(
        output.stdout().contains("my-skill"),
        "should list the skill, got: {}",
        output.stdout()
    );
}

#[test]
fn with_targets_symlink_exists() {
    let project = Project::from_template("tests/fixtures/with-targets")
        .build()
        .unwrap();
    let symlink = project
        .path()
        .join(".claude")
        .join("skills")
        .join("my-skill");
    assert!(
        symlink.symlink_metadata().is_ok(),
        "target symlink should exist at .claude/skills/my-skill"
    );
}

#[test]
fn with_targets_skill_accessible_via_symlink() {
    let project = Project::from_template("tests/fixtures/with-targets")
        .build()
        .unwrap();
    let via_symlink = project
        .path()
        .join(".claude")
        .join("skills")
        .join("my-skill")
        .join("SKILL.md");
    assert!(
        via_symlink.exists(),
        "SKILL.md should be accessible through the target symlink"
    );
}

#[test]
fn with_targets_remove_skill() {
    let project = Project::from_template("tests/fixtures/with-targets")
        .build()
        .unwrap();
    let output = ion()
        .args(["remove", "--yes", "my-skill"])
        .project(&project)
        .run()
        .unwrap();
    assert!(output.success());
    let manifest = std::fs::read_to_string(project.path().join("Ion.toml")).unwrap();
    assert!(
        !manifest.contains("my-skill"),
        "skill should be removed from manifest"
    );
}

#[test]
fn with_targets_validate_via_target_dir() {
    let project = Project::from_template("tests/fixtures/with-targets")
        .build()
        .unwrap();
    // Validate by pointing at the skill through the target symlink path
    let target_skill = project
        .path()
        .join(".claude")
        .join("skills")
        .join("my-skill");
    let output = ion()
        .args(["skill", "validate", &target_skill.display().to_string()])
        .project(&project)
        .run()
        .unwrap();
    assert!(
        output.success(),
        "validation via symlinked target path should work, got: {}{}",
        output.stdout(),
        output.stderr()
    );
}

#[test]
fn with_targets_init_force_preserves_target_config() {
    let project = Project::from_template("tests/fixtures/with-targets")
        .build()
        .unwrap();
    let output = ion()
        .args(["init", "-t", "claude", "--force"])
        .project(&project)
        .run()
        .unwrap();
    assert!(output.success());
    let toml = std::fs::read_to_string(project.path().join("Ion.toml")).unwrap();
    assert!(
        toml.contains("claude"),
        "claude target should be in Ion.toml"
    );
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// Custom skills directory
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

#[test]
fn custom_skills_dir_list_finds_skill() {
    let project = Project::from_template("tests/fixtures/custom-skills-dir")
        .build()
        .unwrap();
    let output = ion()
        .args(["skill", "list"])
        .project(&project)
        .run()
        .unwrap();
    assert!(output.success());
    assert!(
        output.stdout().contains("my-skill"),
        "should find skill in custom dir, got: {}",
        output.stdout()
    );
}

#[test]
fn custom_skills_dir_json_list() {
    let project = Project::from_template("tests/fixtures/custom-skills-dir")
        .build()
        .unwrap();
    let output = ion()
        .args(["--json", "skill", "list"])
        .project(&project)
        .run()
        .unwrap();
    assert!(output.success());
    let json: serde_json::Value = serde_json::from_str(output.stdout()).unwrap();
    let skills = json["data"].as_array().unwrap();
    assert_eq!(skills.len(), 1);
    assert_eq!(skills[0]["name"], "my-skill");
}

#[test]
fn custom_skills_dir_validate() {
    let project = Project::from_template("tests/fixtures/custom-skills-dir")
        .build()
        .unwrap();
    let skill_path = project.path().join("my-skills").join("my-skill");
    let output = ion()
        .args(["skill", "validate", &skill_path.display().to_string()])
        .project(&project)
        .run()
        .unwrap();
    assert!(
        output.success(),
        "skill in custom dir should validate, got: {}{}",
        output.stdout(),
        output.stderr()
    );
}

#[test]
fn custom_skills_dir_remove() {
    let project = Project::from_template("tests/fixtures/custom-skills-dir")
        .build()
        .unwrap();
    let output = ion()
        .args(["remove", "--yes", "my-skill"])
        .project(&project)
        .run()
        .unwrap();
    assert!(output.success());
    let manifest = std::fs::read_to_string(project.path().join("Ion.toml")).unwrap();
    assert!(
        !manifest.contains("my-skill ="),
        "skill entry should be removed from Ion.toml, got: {manifest}"
    );
}

#[test]
fn custom_skills_dir_skill_new_uses_custom_dir() {
    let project = Project::from_template("tests/fixtures/custom-skills-dir")
        .build()
        .unwrap();
    let skill_path = project.path().join("my-skills").join("new-skill");
    let output = ion()
        .args(["skill", "new", "--path", &skill_path.display().to_string()])
        .project(&project)
        .run()
        .unwrap();
    assert!(
        output.success(),
        "skill new should work with custom skills dir, got: {}{}",
        output.stdout(),
        output.stderr()
    );
    assert!(
        skill_path.join("SKILL.md").exists(),
        "new skill should be created at {}",
        skill_path.display()
    );
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// Invalid skill project
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

#[test]
fn invalid_skill_validate_reports_errors() {
    let project = Project::from_template("tests/fixtures/invalid-skill")
        .build()
        .unwrap();
    let skill_path = project
        .path()
        .join(".agents")
        .join("skills")
        .join("bad-skill");
    let output = ion()
        .args(["skill", "validate", &skill_path.display().to_string()])
        .project(&project)
        .run()
        .unwrap();
    assert_eq!(
        output.exit_code(),
        1,
        "validation should fail for invalid skill, got: {}{}",
        output.stdout(),
        output.stderr()
    );
}

#[test]
fn invalid_skill_validate_json_has_errors() {
    let project = Project::from_template("tests/fixtures/invalid-skill")
        .build()
        .unwrap();
    let skill_path = project
        .path()
        .join(".agents")
        .join("skills")
        .join("bad-skill");
    let output = ion()
        .args([
            "--json",
            "skill",
            "validate",
            &skill_path.display().to_string(),
        ])
        .project(&project)
        .run()
        .unwrap();
    assert_eq!(output.exit_code(), 1);
    let json: serde_json::Value = serde_json::from_str(output.stdout()).unwrap();
    assert_eq!(json["success"], false);
    assert!(
        json["data"]["total_errors"].as_u64().unwrap() > 0,
        "should report validation errors"
    );
}

#[test]
fn invalid_skill_list_still_shows_skill() {
    let project = Project::from_template("tests/fixtures/invalid-skill")
        .build()
        .unwrap();
    let output = ion()
        .args(["skill", "list"])
        .project(&project)
        .run()
        .unwrap();
    assert!(output.success());
    assert!(
        output.stdout().contains("bad-skill"),
        "invalid skill should still appear in list, got: {}",
        output.stdout()
    );
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// Project with lockfile
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

#[test]
fn with_lockfile_skill_list_works() {
    let project = Project::from_template("tests/fixtures/with-lockfile")
        .build()
        .unwrap();
    let output = ion()
        .args(["skill", "list"])
        .project(&project)
        .run()
        .unwrap();
    assert!(output.success());
    assert!(
        output.stdout().contains("my-skill"),
        "should list skill with lockfile, got: {}",
        output.stdout()
    );
}

#[test]
fn with_lockfile_json_list() {
    let project = Project::from_template("tests/fixtures/with-lockfile")
        .build()
        .unwrap();
    let output = ion()
        .args(["--json", "skill", "list"])
        .project(&project)
        .run()
        .unwrap();
    assert!(output.success());
    let json: serde_json::Value = serde_json::from_str(output.stdout()).unwrap();
    let skills = json["data"].as_array().unwrap();
    assert_eq!(skills.len(), 1);
    assert_eq!(skills[0]["name"], "my-skill");
}

#[test]
fn with_lockfile_update_skips_local() {
    let project = Project::from_template("tests/fixtures/with-lockfile")
        .build()
        .unwrap();
    let output = ion()
        .args(["--json", "update"])
        .project(&project)
        .run()
        .unwrap();
    assert!(output.success());
    let json: serde_json::Value = serde_json::from_str(output.stdout()).unwrap();
    assert_eq!(json["success"], true);
}

#[test]
fn with_lockfile_remove_cleans_lockfile() {
    let project = Project::from_template("tests/fixtures/with-lockfile")
        .build()
        .unwrap();
    let output = ion()
        .args(["remove", "--yes", "my-skill"])
        .project(&project)
        .run()
        .unwrap();
    assert!(output.success());
    let lock_content = std::fs::read_to_string(project.path().join("Ion.lock")).unwrap();
    assert!(
        !lock_content.contains("my-skill"),
        "skill should be removed from lockfile, got: {lock_content}"
    );
}

#[test]
fn with_lockfile_validate_works() {
    let project = Project::from_template("tests/fixtures/with-lockfile")
        .build()
        .unwrap();
    let skill_path = project
        .path()
        .join(".agents")
        .join("skills")
        .join("my-skill");
    let output = ion()
        .args(["skill", "validate", &skill_path.display().to_string()])
        .project(&project)
        .run()
        .unwrap();
    assert!(output.success());
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// Cross-fixture: skill new in various project setups
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

#[test]
fn skill_new_in_empty_project() {
    let project = Project::from_template("tests/fixtures/empty-project")
        .build()
        .unwrap();
    let skill_path = project
        .path()
        .join(".agents")
        .join("skills")
        .join("fresh-skill");
    let output = ion()
        .args(["skill", "new", "--path", &skill_path.display().to_string()])
        .project(&project)
        .run()
        .unwrap();
    assert!(
        output.success(),
        "should create skill in empty project, got: {}{}",
        output.stdout(),
        output.stderr()
    );
    assert!(
        skill_path.join("SKILL.md").exists(),
        "SKILL.md should be created"
    );
}

#[test]
fn skill_new_in_single_skill_project() {
    let project = Project::from_template("tests/fixtures/single-skill")
        .build()
        .unwrap();
    let skill_path = project
        .path()
        .join(".agents")
        .join("skills")
        .join("second-skill");
    let output = ion()
        .args(["skill", "new", "--path", &skill_path.display().to_string()])
        .project(&project)
        .run()
        .unwrap();
    assert!(
        output.success(),
        "should create a second skill, got: {}{}",
        output.stdout(),
        output.stderr()
    );
    // Both skills should now be listed
    let output = ion()
        .args(["--json", "skill", "list"])
        .project(&project)
        .run()
        .unwrap();
    let json: serde_json::Value = serde_json::from_str(output.stdout()).unwrap();
    let skills = json["data"].as_array().unwrap();
    assert!(
        skills.len() >= 2,
        "should have at least 2 skills after adding, got: {skills:?}"
    );
}

#[test]
fn skill_new_json_in_empty_project() {
    let project = Project::from_template("tests/fixtures/empty-project")
        .build()
        .unwrap();
    let skill_path = project
        .path()
        .join(".agents")
        .join("skills")
        .join("json-skill");
    let output = ion()
        .args([
            "--json",
            "skill",
            "new",
            "--path",
            &skill_path.display().to_string(),
        ])
        .project(&project)
        .run()
        .unwrap();
    assert!(output.success());
    let json: serde_json::Value = serde_json::from_str(output.stdout()).unwrap();
    assert_eq!(json["success"], true);
}
