use std::process::Command;

fn ion() -> Command {
    Command::new(env!("CARGO_BIN_EXE_ion"))
}

#[test]
fn json_flag_appears_in_help() {
    let output = ion().arg("--help").output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("--json"));
}

#[test]
fn json_error_is_structured() {
    let dir = tempfile::tempdir().unwrap();
    // No Ion.toml exists, so remove will fail
    let output = ion()
        .args(["--json", "remove", "--yes", "nonexistent-skill-xyz"])
        .current_dir(dir.path())
        .output()
        .unwrap();
    assert_ne!(output.status.code(), Some(0));
    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value = serde_json::from_str(&stdout).expect("valid JSON error");
    assert_eq!(parsed["success"], false);
    assert!(parsed["error"].is_string());
}

#[test]
fn json_self_info() {
    let output = ion()
        .args(["--json", "self", "info"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value = serde_json::from_str(&stdout).expect("valid JSON");
    assert_eq!(parsed["success"], true);
    assert!(parsed["data"]["version"].is_string());
    assert!(parsed["data"]["target"].is_string());
    assert!(parsed["data"]["exe"].is_string());
}

#[test]
fn json_init_without_targets_returns_action_required() {
    let dir = tempfile::tempdir().unwrap();
    let output = ion()
        .args(["--json", "project", "init"])
        .current_dir(dir.path())
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2));
    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value = serde_json::from_str(&stdout).expect("valid JSON");
    assert_eq!(parsed["success"], false);
    assert_eq!(parsed["action_required"], "target_selection");
    assert!(parsed["data"]["available_targets"].is_array());
}

#[test]
fn json_init_with_targets_succeeds() {
    let dir = tempfile::tempdir().unwrap();
    let output = ion()
        .args(["--json", "project", "init", "--target", "claude"])
        .current_dir(dir.path())
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(0));
    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value = serde_json::from_str(&stdout).expect("valid JSON");
    assert_eq!(parsed["success"], true);
    assert!(parsed["data"]["targets"].is_object());
}

#[test]
fn json_remove_without_yes_returns_action_required() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("Ion.toml"),
        "[skills]\ntest-skill = \"owner/repo\"\n",
    )
    .unwrap();
    std::fs::write(dir.path().join("Ion.lock"), "").unwrap();

    let output = ion()
        .args(["--json", "remove", "test-skill"])
        .current_dir(dir.path())
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2));
    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value = serde_json::from_str(&stdout).expect("valid JSON");
    assert_eq!(parsed["success"], false);
    assert_eq!(parsed["action_required"], "confirm_removal");
    assert!(parsed["data"]["skills"].is_array());
}

#[test]
fn json_remove_yes_returns_pure_json() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("Ion.toml"),
        "[skills]\ntest-skill = \"owner/repo\"\n",
    )
    .unwrap();
    std::fs::write(dir.path().join("Ion.lock"), "").unwrap();
    std::fs::create_dir_all(dir.path().join(".agents/skills/test-skill")).unwrap();

    let output = ion()
        .args(["--json", "remove", "test-skill", "--yes"])
        .current_dir(dir.path())
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value = serde_json::from_str(&stdout)
        .expect("stdout should be valid JSON with no extra text");
    assert_eq!(parsed["success"], true);
    assert!(parsed["data"]["removed"].is_array());
}

#[test]
fn json_config_no_subcommand_errors() {
    let output = ion()
        .args(["--json", "config"])
        .output()
        .unwrap();
    assert!(!output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value = serde_json::from_str(&stdout).expect("valid JSON");
    assert_eq!(parsed["success"], false);
    assert!(parsed["error"].as_str().unwrap().contains("--json mode"));
}

#[test]
fn json_skill_list_empty_project() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("Ion.toml"), "[skills]\n").unwrap();
    let output = ion()
        .args(["--json", "skill", "list"])
        .current_dir(dir.path())
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value = serde_json::from_str(&stdout).expect("valid JSON");
    assert_eq!(parsed["success"], true);
}

#[test]
fn json_gc_dry_run() {
    let output = ion()
        .args(["--json", "cache", "gc", "--dry-run"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value = serde_json::from_str(&stdout).expect("valid JSON");
    assert_eq!(parsed["success"], true);
    assert_eq!(parsed["data"]["dry_run"], true);
}

#[test]
fn interactive_flag_removed_from_search() {
    let output = ion()
        .args(["search", "--interactive", "test"])
        .output()
        .unwrap();
    // Should fail because --interactive no longer exists
    assert!(!output.status.success());
}
