//! Structural validation tests for `ion --json` commands.
//!
//! SKILL.md is generated at build time by build.rs from templates/ion-cli.md.j2.
//! These tests validate that the real `ion --json` command output matches the
//! expected structure documented in the generated SKILL.md.

use std::path::Path;
use std::process::Command;

fn ion() -> Command {
    Command::new(env!("CARGO_BIN_EXE_ion"))
}

/// Run an ion command and return stdout. Accepts exit 0 and 2 (action_required).
fn capture_json(args: &[&str], dir: &Path) -> String {
    let output = ion()
        .args(args)
        .current_dir(dir)
        .output()
        .expect("failed to execute ion");

    let code = output.status.code().unwrap_or(-1);
    assert!(
        code == 0 || code == 2,
        "ion {:?} failed with exit {code}\nstderr: {}",
        args,
        String::from_utf8_lossy(&output.stderr)
    );

    String::from_utf8(output.stdout)
        .expect("non-utf8 stdout")
        .trim()
        .to_string()
}

fn parse(s: &str) -> serde_json::Value {
    serde_json::from_str(s).expect("invalid JSON")
}

// ---------------------------------------------------------------------------
// Structural validation: verify real command output matches documented format
// ---------------------------------------------------------------------------

#[test]
fn json_init_no_targets_structure() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir(dir.path().join(".claude")).unwrap();
    let out = parse(&capture_json(&["--json", "project", "init"], dir.path()));

    assert_eq!(out["success"], false);
    assert_eq!(out["action_required"], "target_selection");
    assert!(out["data"]["available_targets"].is_array());
    assert!(out["data"]["hint"].is_string());

    // Each target has name, path, detected
    let targets = out["data"]["available_targets"].as_array().unwrap();
    assert!(!targets.is_empty());
    for t in targets {
        assert!(t["name"].is_string());
        assert!(t["path"].is_string());
        assert!(t["detected"].is_boolean());
    }
}

#[test]
fn json_init_with_targets_structure() {
    let dir = tempfile::tempdir().unwrap();
    let out = parse(&capture_json(
        &[
            "--json", "project", "init", "--target", "claude", "--target", "cursor",
        ],
        dir.path(),
    ));

    assert_eq!(out["success"], true);
    assert!(out["data"]["targets"].is_object());
    assert!(out["data"]["manifest"].is_string());
}

#[test]
fn json_remove_confirm_structure() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("Ion.toml"),
        "[skills]\ntest-skill = \"owner/repo\"\n",
    )
    .unwrap();
    std::fs::write(dir.path().join("Ion.lock"), "").unwrap();

    let out = parse(&capture_json(
        &["--json", "remove", "test-skill"],
        dir.path(),
    ));

    assert_eq!(out["success"], false);
    assert_eq!(out["action_required"], "confirm_removal");
    assert!(out["data"]["skills"].is_array());
}

#[test]
fn json_remove_yes_structure() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("Ion.toml"),
        "[skills]\ntest-skill = \"owner/repo\"\n",
    )
    .unwrap();
    std::fs::write(dir.path().join("Ion.lock"), "").unwrap();
    std::fs::create_dir_all(dir.path().join(".agents/skills/test-skill")).unwrap();

    let out = parse(&capture_json(
        &["--json", "remove", "test-skill", "--yes"],
        dir.path(),
    ));

    assert_eq!(out["success"], true);
    assert!(out["data"]["removed"].is_array());
}

#[test]
fn json_skill_list_structure() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("Ion.toml"), "[skills]\n").unwrap();

    let out = parse(&capture_json(&["--json", "skill", "list"], dir.path()));

    assert_eq!(out["success"], true);
    assert!(out["data"].is_array());
}

#[test]
fn json_validate_structure() {
    let dir = tempfile::tempdir().unwrap();
    let skill_dir = dir.path().join("test-skill");
    std::fs::create_dir_all(&skill_dir).unwrap();
    std::fs::write(
        skill_dir.join("SKILL.md"),
        "---\nname: test-skill\ndescription: A test skill\n---\n\n# Test Skill\n",
    )
    .unwrap();

    let out = parse(&capture_json(
        &[
            "--json",
            "skill",
            "validate",
            &skill_dir.display().to_string(),
        ],
        dir.path(),
    ));

    assert_eq!(out["success"], true);
    assert!(out["data"]["skills"].is_array());
    assert!(out["data"]["total_errors"].is_number());
    assert!(out["data"]["total_warnings"].is_number());
    assert!(out["data"]["total_infos"].is_number());
}

#[test]
fn json_config_list_structure() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("Ion.toml"),
        "[options.targets]\nclaude = \".claude/skills\"\n",
    )
    .unwrap();

    let out = parse(&capture_json(
        &["--json", "config", "list", "--project"],
        dir.path(),
    ));

    assert_eq!(out["success"], true);
    assert!(out["data"].is_object());
}

#[test]
fn json_config_get_structure() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("Ion.toml"),
        "[options.targets]\nclaude = \".claude/skills\"\n",
    )
    .unwrap();

    let out = parse(&capture_json(
        &["--json", "config", "get", "targets.claude", "--project"],
        dir.path(),
    ));

    assert_eq!(out["success"], true);
    assert!(out["data"]["key"].is_string());
    assert!(out["data"]["value"].is_string());
}

#[test]
fn json_config_set_structure() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("Ion.toml"),
        "[options.targets]\nclaude = \".claude/skills\"\n",
    )
    .unwrap();

    let out = parse(&capture_json(
        &[
            "--json",
            "config",
            "set",
            "targets.claude",
            ".claude/commands",
            "--project",
        ],
        dir.path(),
    ));

    assert_eq!(out["success"], true);
    assert!(out["data"]["key"].is_string());
    assert!(out["data"]["value"].is_string());
}

#[test]
fn json_gc_dry_run_structure() {
    let dir = tempfile::tempdir().unwrap();
    let out = parse(&capture_json(
        &["--json", "cache", "gc", "--dry-run"],
        dir.path(),
    ));

    assert_eq!(out["success"], true);
    assert_eq!(out["data"]["dry_run"], true);
    assert!(out["data"]["removed"].is_array());
}

#[test]
fn json_self_info_structure() {
    let dir = tempfile::tempdir().unwrap();
    let out = parse(&capture_json(&["--json", "self", "info"], dir.path()));

    assert_eq!(out["success"], true);
    assert!(out["data"]["version"].is_string());
    assert!(out["data"]["target"].is_string());
    assert!(out["data"]["exe"].is_string());
}
