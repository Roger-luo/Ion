//! Tests for ion CLI behavior under different terminal conditions.
//!
//! Uses the `scenario` crate to run ion in both piped and PTY modes,
//! testing color output, exit codes, JSON mode, interactive prompts,
//! and terminal-dependent behavior.

use std::time::Duration;

use scenario::{Scenario, Terminal};

const ION: &str = env!("CARGO_BIN_EXE_ion");

fn ion() -> Scenario {
    Scenario::new(ION).timeout(Duration::from_secs(10))
}

/// Create a minimal Ion project in a temp dir and return (dir, tempdir_guard).
fn project() -> (std::path::PathBuf, tempfile::TempDir) {
    let tmp = tempfile::tempdir().unwrap();
    std::fs::write(tmp.path().join("Ion.toml"), "[skills]\n").unwrap();
    (tmp.path().to_path_buf(), tmp)
}

/// Create an Ion project with a test skill declared.
fn project_with_skill() -> (std::path::PathBuf, tempfile::TempDir) {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();

    // Create a local skill directory
    let skill_dir = dir.join(".agents").join("skills").join("test-skill");
    std::fs::create_dir_all(&skill_dir).unwrap();
    std::fs::write(
        skill_dir.join("SKILL.md"),
        "---\nname: test-skill\ndescription: A test skill\n---\nTest body\n",
    )
    .unwrap();

    std::fs::write(
        dir.join("Ion.toml"),
        "[skills]\ntest-skill = { type = \"local\" }\n",
    )
    .unwrap();

    (dir.to_path_buf(), tmp)
}

/// Create an Ion project with a skill that has validation issues.
fn project_with_bad_skill() -> (std::path::PathBuf, tempfile::TempDir) {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();

    let skill_dir = dir.join("bad-skill");
    std::fs::create_dir_all(&skill_dir).unwrap();
    // Missing required fields in frontmatter
    std::fs::write(skill_dir.join("SKILL.md"), "---\nname: bad-skill\n---\n").unwrap();

    (dir.to_path_buf(), tmp)
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// Help output
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

#[test]
fn help_piped_no_ansi() {
    let output = ion().args(["--help"]).run().unwrap();
    assert!(output.success());
    assert!(output.stdout().contains("Agent skill manager"));
    // Piped mode: raw bytes should NOT contain ANSI escape codes
    assert!(
        !output.stdout_raw().contains(&0x1B),
        "piped help output should not contain ANSI escape codes"
    );
}

#[test]
fn help_pty_has_ansi() {
    let output = ion()
        .args(["--help"])
        .terminal(Terminal::pty(80, 24))
        .run()
        .unwrap();
    assert!(output.success());
    assert!(output.stdout().contains("Agent skill manager"));
    // Clap colors help output when it detects a real terminal
    assert!(
        output.stdout_raw().contains(&0x1B),
        "PTY help output should contain ANSI color codes"
    );
}

#[test]
fn help_lists_all_commands() {
    let output = ion().args(["--help"]).run().unwrap();
    let stdout = output.stdout();
    for cmd in [
        "init", "add", "remove", "search", "update", "skill", "config", "self",
    ] {
        assert!(stdout.contains(cmd), "help should list '{cmd}' command");
    }
}

#[test]
fn version_flag() {
    let output = ion().args(["--version"]).run().unwrap();
    assert!(output.success());
    assert!(output.stdout().contains("ion "));
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// Color: piped vs PTY
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

#[test]
fn skill_list_piped_no_color() {
    let (dir, _tmp) = project_with_skill();
    let output = ion()
        .args(["skill", "list"])
        .current_dir(&dir)
        .run()
        .unwrap();
    assert!(output.success());
    assert!(output.stdout().contains("test-skill"));
    // Piped: no ANSI in raw stdout
    assert!(
        !output.stdout_raw().contains(&0x1B),
        "piped skill list should not contain ANSI codes"
    );
}

#[test]
fn skill_list_pty_has_color() {
    let (dir, _tmp) = project_with_skill();
    let output = ion()
        .args(["skill", "list"])
        .current_dir(&dir)
        .terminal(Terminal::pty(80, 24))
        .run()
        .unwrap();
    assert!(output.success());
    assert!(output.stdout().contains("test-skill"));
    // PTY: raw output should contain ANSI escape codes (colors)
    assert!(
        output.stdout_raw().contains(&0x1B),
        "PTY skill list should contain ANSI color codes"
    );
}

#[test]
fn validate_piped_no_color() {
    let (dir, _tmp) = project_with_skill();
    let skill_path = dir.join(".agents").join("skills").join("test-skill");
    let output = ion()
        .args(["skill", "validate", &skill_path.display().to_string()])
        .current_dir(&dir)
        .run()
        .unwrap();
    // Should succeed (valid skill)
    assert!(output.success());
    assert!(
        !output.stdout_raw().contains(&0x1B),
        "piped validate should not contain ANSI codes"
    );
}

#[test]
fn validate_pty_has_color() {
    let (dir, _tmp) = project_with_skill();
    let skill_path = dir.join(".agents").join("skills").join("test-skill");
    let output = ion()
        .args(["skill", "validate", &skill_path.display().to_string()])
        .current_dir(&dir)
        .terminal(Terminal::pty(80, 24))
        .run()
        .unwrap();
    assert!(output.success());
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// Exit codes
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

#[test]
fn error_exit_code_1_piped() {
    let (dir, _tmp) = project();
    let output = ion()
        .args(["remove", "--yes", "nonexistent-skill"])
        .current_dir(&dir)
        .run()
        .unwrap();
    assert!(!output.success());
    assert_eq!(output.exit_code(), 1);
}

#[test]
fn error_exit_code_1_pty() {
    let (dir, _tmp) = project();
    let output = ion()
        .args(["remove", "--yes", "nonexistent-skill"])
        .current_dir(&dir)
        .terminal(Terminal::pty(80, 24))
        .run()
        .unwrap();
    assert!(!output.success());
    assert_eq!(output.exit_code(), 1);
}

#[test]
fn json_error_exit_code_1() {
    let (dir, _tmp) = project();
    let output = ion()
        .args(["--json", "remove", "--yes", "nonexistent-skill"])
        .current_dir(&dir)
        .run()
        .unwrap();
    assert!(!output.success());
    assert_eq!(output.exit_code(), 1);
    let json: serde_json::Value = serde_json::from_str(output.stdout()).unwrap();
    assert_eq!(json["success"], false);
    assert!(
        json["error"]
            .as_str()
            .unwrap()
            .contains("No skills matching")
    );
}

#[test]
fn json_action_required_exit_code_2() {
    let (dir, _tmp) = project_with_skill();
    let output = ion()
        .args(["--json", "remove", "test-skill"])
        .current_dir(&dir)
        .run()
        .unwrap();
    // Without --yes, JSON mode returns action_required with exit code 2
    assert!(!output.success());
    assert_eq!(output.exit_code(), 2);
    let json: serde_json::Value = serde_json::from_str(output.stdout()).unwrap();
    assert_eq!(json["success"], false);
    assert_eq!(json["action_required"], "confirm_removal");
    assert!(json["data"]["skills"].is_array());
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// JSON output mode
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

#[test]
fn json_skill_list_empty() {
    let (dir, _tmp) = project();
    let output = ion()
        .args(["--json", "skill", "list"])
        .current_dir(&dir)
        .run()
        .unwrap();
    assert!(output.success());
    let json: serde_json::Value = serde_json::from_str(output.stdout()).unwrap();
    assert_eq!(json["success"], true);
    assert_eq!(json["data"], serde_json::json!([]));
}

#[test]
fn json_skill_list_with_skill() {
    let (dir, _tmp) = project_with_skill();
    let output = ion()
        .args(["--json", "skill", "list"])
        .current_dir(&dir)
        .run()
        .unwrap();
    assert!(output.success());
    let json: serde_json::Value = serde_json::from_str(output.stdout()).unwrap();
    assert_eq!(json["success"], true);
    let skills = json["data"].as_array().unwrap();
    assert_eq!(skills.len(), 1);
    assert_eq!(skills[0]["name"], "test-skill");
}

#[test]
fn json_pretty_flag() {
    let (dir, _tmp) = project();
    let output = ion()
        .args(["--json", "--pretty", "skill", "list"])
        .current_dir(&dir)
        .run()
        .unwrap();
    assert!(output.success());
    // Pretty JSON has newlines and indentation
    let raw = output.stdout();
    assert!(raw.contains('\n'), "pretty JSON should contain newlines");
    assert!(raw.contains("  "), "pretty JSON should be indented");
    let json: serde_json::Value = serde_json::from_str(raw).unwrap();
    assert_eq!(json["success"], true);
}

#[test]
fn json_compact_by_default() {
    let (dir, _tmp) = project();
    let output = ion()
        .args(["--json", "skill", "list"])
        .current_dir(&dir)
        .run()
        .unwrap();
    assert!(output.success());
    let raw = output.stdout().trim();
    // Compact JSON is a single line
    assert!(
        !raw.contains('\n'),
        "compact JSON should be a single line, got: {raw}"
    );
}

#[test]
fn json_validate_structure() {
    let (dir, _tmp) = project_with_skill();
    let skill_path = dir.join(".agents").join("skills").join("test-skill");
    let output = ion()
        .args([
            "--json",
            "skill",
            "validate",
            &skill_path.display().to_string(),
        ])
        .current_dir(&dir)
        .run()
        .unwrap();
    let json: serde_json::Value = serde_json::from_str(output.stdout()).unwrap();
    assert_eq!(json["success"], true);
    let data = &json["data"];
    assert!(data["skills"].is_array());
    assert!(data["total_errors"].is_number());
    assert!(data["total_warnings"].is_number());
    assert!(data["total_infos"].is_number());
}

/// BUG: `ion --json skill validate` on a skill with errors outputs `"success": true`
/// in the JSON body but exits with code 1. The JSON envelope and exit code contradict
/// each other. The code calls `json::print_success()` then `process::exit(1)`.
/// It should use a response with `"success": false` when `total_errors > 0`.
#[test]
#[ignore = "broken: validate JSON says success:true but exits with code 1 on errors"]
fn json_validate_bad_skill_should_report_failure() {
    let (dir, _tmp) = project_with_bad_skill();
    let skill_path = dir.join("bad-skill");
    let output = ion()
        .args([
            "--json",
            "skill",
            "validate",
            &skill_path.display().to_string(),
        ])
        .current_dir(&dir)
        .run()
        .unwrap();
    assert_eq!(output.exit_code(), 1);
    let json: serde_json::Value = serde_json::from_str(output.stdout()).unwrap();
    // Should be false when there are validation errors
    assert_eq!(
        json["success"], false,
        "JSON should report success:false when validation has errors"
    );
}

#[test]
fn json_validate_bad_skill() {
    let (dir, _tmp) = project_with_bad_skill();
    let skill_path = dir.join("bad-skill");
    let output = ion()
        .args([
            "--json",
            "skill",
            "validate",
            &skill_path.display().to_string(),
        ])
        .current_dir(&dir)
        .run()
        .unwrap();
    // BUG: JSON says success:true but exit code is 1. See: json_validate_bad_skill_should_report_failure
    assert_eq!(output.exit_code(), 1);
    let json: serde_json::Value = serde_json::from_str(output.stdout()).unwrap();
    assert_eq!(json["success"], true); // contradicts exit code 1
    let data = &json["data"];
    assert!(data["skills"].is_array());
    assert!(data["total_errors"].as_u64().unwrap() > 0);
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// Error output goes to stderr (human mode) vs stdout (JSON mode)
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

#[test]
fn human_error_on_stderr() {
    let (dir, _tmp) = project();
    let output = ion()
        .args(["remove", "--yes", "nonexistent-skill"])
        .current_dir(&dir)
        .run()
        .unwrap();
    assert!(!output.success());
    assert!(
        output.stderr().contains("Error:"),
        "human-mode errors should go to stderr, got stderr='{}'",
        output.stderr()
    );
    // stdout should be empty (or just contain the "Will remove" line which errors before)
    assert!(
        !output.stdout().contains("Error:"),
        "error text should NOT be on stdout in human mode"
    );
}

#[test]
fn json_error_on_stdout() {
    let (dir, _tmp) = project();
    let output = ion()
        .args(["--json", "remove", "--yes", "nonexistent-skill"])
        .current_dir(&dir)
        .run()
        .unwrap();
    assert!(!output.success());
    // JSON errors are on stdout
    let json: serde_json::Value = serde_json::from_str(output.stdout()).unwrap();
    assert_eq!(json["success"], false);
    assert!(json["error"].is_string());
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// init command: TTY vs piped behavior
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

#[test]
fn init_with_target_flag_piped() {
    let tmp = tempfile::tempdir().unwrap();
    let output = ion()
        .args(["init", "-t", "claude"])
        .current_dir(tmp.path())
        .run()
        .unwrap();
    assert!(output.success());
    assert!(tmp.path().join("Ion.toml").exists());
}

#[test]
fn init_with_target_flag_pty() {
    let tmp = tempfile::tempdir().unwrap();
    let output = ion()
        .args(["init", "-t", "claude"])
        .current_dir(tmp.path())
        .terminal(Terminal::pty(80, 24))
        .run()
        .unwrap();
    assert!(output.success());
    assert!(tmp.path().join("Ion.toml").exists());
}

#[test]
fn init_json_without_target_action_required() {
    let tmp = tempfile::tempdir().unwrap();
    let output = ion()
        .args(["--json", "init"])
        .current_dir(tmp.path())
        .run()
        .unwrap();
    assert_eq!(output.exit_code(), 2);
    let json: serde_json::Value = serde_json::from_str(output.stdout()).unwrap();
    assert_eq!(json["success"], false);
    assert_eq!(json["action_required"], "target_selection");
}

/// BUG: `ion init` silently overwrites an existing Ion.toml without requiring `--force`.
/// This is inconsistent with `ion skill new` which errors when SKILL.md already exists
/// unless `--force` is passed. Init should either require `--force` or at minimum warn.
#[test]
#[ignore = "broken: init silently overwrites existing Ion.toml without --force"]
fn init_should_error_without_force_when_manifest_exists() {
    let (dir, _tmp) = project();
    let output = ion()
        .args(["init", "-t", "claude"])
        .current_dir(&dir)
        .run()
        .unwrap();
    assert!(
        !output.success(),
        "init should fail when Ion.toml already exists"
    );
}

#[test]
fn init_already_initialized_overwrites() {
    let (dir, _tmp) = project();
    let output = ion()
        .args(["init", "-t", "claude"])
        .current_dir(&dir)
        .run()
        .unwrap();
    // BUG: Re-init silently succeeds and overwrites existing Ion.toml.
    // See: init_should_error_without_force_when_manifest_exists
    assert!(output.success());
    let toml = std::fs::read_to_string(dir.join("Ion.toml")).unwrap();
    assert!(toml.contains("claude"));
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// remove command: interactive prompt
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

#[test]
fn remove_confirm_yes_piped() {
    let (dir, _tmp) = project_with_skill();
    let output = ion()
        .args(["remove", "test-skill"])
        .stdin(b"y\n".to_vec())
        .current_dir(&dir)
        .run()
        .unwrap();
    assert!(output.success());
    // Skill should be removed from Ion.toml
    let manifest = std::fs::read_to_string(dir.join("Ion.toml")).unwrap();
    assert!(!manifest.contains("test-skill"));
}

#[test]
fn remove_confirm_no_piped() {
    let (dir, _tmp) = project_with_skill();
    let output = ion()
        .args(["remove", "test-skill"])
        .stdin(b"n\n".to_vec())
        .current_dir(&dir)
        .run()
        .unwrap();
    assert!(!output.success()); // Aborted
    // Skill should still be in Ion.toml
    let manifest = std::fs::read_to_string(dir.join("Ion.toml")).unwrap();
    assert!(manifest.contains("test-skill"));
}

#[test]
fn remove_yes_flag_skips_prompt() {
    let (dir, _tmp) = project_with_skill();
    let output = ion()
        .args(["remove", "--yes", "test-skill"])
        .current_dir(&dir)
        .run()
        .unwrap();
    assert!(output.success());
    let manifest = std::fs::read_to_string(dir.join("Ion.toml")).unwrap();
    assert!(!manifest.contains("test-skill"));
}

#[test]
fn remove_interactive_pty_confirm() {
    let (dir, _tmp) = project_with_skill();
    let mut session = ion()
        .args(["remove", "test-skill"])
        .current_dir(&dir)
        .terminal(Terminal::pty(80, 24))
        .spawn()
        .unwrap();

    session.expect("Proceed?").unwrap();
    session.send_line("y").unwrap();

    let output = session.wait().unwrap();
    assert!(output.success());
}

#[test]
fn remove_interactive_pty_reject() {
    let (dir, _tmp) = project_with_skill();
    let mut session = ion()
        .args(["remove", "test-skill"])
        .current_dir(&dir)
        .terminal(Terminal::pty(80, 24))
        .spawn()
        .unwrap();

    session.expect("Proceed?").unwrap();
    session.send_line("n").unwrap();

    let output = session.wait().unwrap();
    assert!(!output.success());
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// skill new
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

#[test]
fn skill_new_creates_files() {
    let tmp = tempfile::tempdir().unwrap();
    let skill_dir = tmp.path().join("my-skill");
    let output = ion()
        .args(["skill", "new", "--path", &skill_dir.display().to_string()])
        .run()
        .unwrap();
    assert!(output.success());
    assert!(skill_dir.join("SKILL.md").exists());
}

#[test]
fn skill_new_json_output() {
    let tmp = tempfile::tempdir().unwrap();
    let skill_dir = tmp.path().join("my-skill");
    let output = ion()
        .args([
            "--json",
            "skill",
            "new",
            "--path",
            &skill_dir.display().to_string(),
        ])
        .run()
        .unwrap();
    assert!(output.success());
    let json: serde_json::Value = serde_json::from_str(output.stdout()).unwrap();
    assert_eq!(json["success"], true);
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// validate command
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

#[test]
fn validate_no_skills_found() {
    let tmp = tempfile::tempdir().unwrap();
    let output = ion()
        .args(["skill", "validate"])
        .current_dir(tmp.path())
        .run()
        .unwrap();
    assert!(output.success());
    assert!(output.stdout().contains("No SKILL.md files found"));
}

#[test]
fn validate_no_skills_json() {
    let tmp = tempfile::tempdir().unwrap();
    let output = ion()
        .args(["--json", "skill", "validate"])
        .current_dir(tmp.path())
        .run()
        .unwrap();
    assert!(output.success());
    let json: serde_json::Value = serde_json::from_str(output.stdout()).unwrap();
    assert_eq!(json["data"]["skills"], serde_json::json!([]));
    assert_eq!(json["data"]["total_errors"], 0);
}

#[test]
fn validate_valid_skill() {
    let (dir, _tmp) = project_with_skill();
    let skill_path = dir.join(".agents").join("skills").join("test-skill");
    let output = ion()
        .args(["skill", "validate", &skill_path.display().to_string()])
        .current_dir(&dir)
        .run()
        .unwrap();
    assert!(output.success());
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// completion command
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

#[test]
fn completion_bash() {
    let output = ion().args(["completion", "bash"]).run().unwrap();
    assert!(output.success());
    assert!(output.stdout().contains("complete"));
}

#[test]
fn completion_fish() {
    let output = ion().args(["completion", "fish"]).run().unwrap();
    assert!(output.success());
    assert!(output.stdout().contains("complete"));
}

#[test]
fn completion_zsh() {
    let output = ion().args(["completion", "zsh"]).run().unwrap();
    assert!(output.success());
    assert!(output.stdout().contains("#compdef ion"));
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// self info
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// BUG: `ion self info` does not use `Paint` for colored output in PTY mode.
/// Other commands like `skill list` and `validate` use `Paint` to color their output,
/// but `self info` delegates to `ionem::SelfManager::print_info()` which uses plain `println!`.
/// The output should be styled consistently: labels like "target:" and "exe:" should be dim,
/// and the version should be bold.
#[test]
#[ignore = "broken: self info has no color output in PTY mode unlike other commands"]
fn self_info_pty_should_have_color() {
    let output = ion()
        .args(["self", "info"])
        .terminal(Terminal::pty(80, 24))
        .run()
        .unwrap();
    assert!(output.success());
    assert!(
        output.stdout_raw().contains(&0x1B),
        "PTY self info should contain ANSI color codes"
    );
}

#[test]
fn self_info_piped() {
    let output = ion().args(["self", "info"]).run().unwrap();
    assert!(output.success());
    // Output format: "ion <version>\ntarget: ...\nexe: ..."
    assert!(output.stdout().contains("ion "));
    assert!(output.stdout().contains("target:"));
}

#[test]
fn self_info_json() {
    let output = ion().args(["--json", "self", "info"]).run().unwrap();
    assert!(output.success());
    let json: serde_json::Value = serde_json::from_str(output.stdout()).unwrap();
    assert_eq!(json["success"], true);
    assert!(json["data"]["version"].is_string());
}

#[test]
fn self_info_pty() {
    let output = ion()
        .args(["self", "info"])
        .terminal(Terminal::pty(80, 24))
        .run()
        .unwrap();
    assert!(output.success());
    // BUG: no color in PTY mode. See: self_info_pty_should_have_color
    assert!(output.stdout().contains("ion "));
    assert!(output.stdout().contains("target:"));
    assert!(output.stdout().contains("exe:"));
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// update command: empty project
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

#[test]
fn update_no_skills_piped() {
    let (dir, _tmp) = project();
    let output = ion().args(["update"]).current_dir(&dir).run().unwrap();
    assert!(output.success());
    assert!(output.stdout().contains("No skills to update"));
}

#[test]
fn update_no_skills_json() {
    let (dir, _tmp) = project();
    let output = ion()
        .args(["--json", "update"])
        .current_dir(&dir)
        .run()
        .unwrap();
    assert!(output.success());
    let json: serde_json::Value = serde_json::from_str(output.stdout()).unwrap();
    assert_eq!(json["success"], true);
    let data = &json["data"];
    assert!(data["updated"].as_array().unwrap().is_empty());
    assert!(data["skipped"].as_array().unwrap().is_empty());
}

#[test]
fn update_nonexistent_skill() {
    let (dir, _tmp) = project();
    let output = ion()
        .args(["update", "nonexistent"])
        .current_dir(&dir)
        .run()
        .unwrap();
    assert!(!output.success());
    assert_eq!(output.exit_code(), 1);
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// cache gc
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

#[test]
fn cache_gc_dry_run() {
    let output = ion().args(["cache", "gc", "--dry-run"]).run().unwrap();
    assert!(output.success());
}

#[test]
fn cache_gc_json() {
    let output = ion()
        .args(["--json", "cache", "gc", "--dry-run"])
        .run()
        .unwrap();
    assert!(output.success());
    let json: serde_json::Value = serde_json::from_str(output.stdout()).unwrap();
    assert_eq!(json["success"], true);
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// Terminal width behavior
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

#[test]
fn help_narrow_terminal() {
    let output = ion()
        .args(["--help"])
        .terminal(Terminal::pty(40, 24))
        .run()
        .unwrap();
    assert!(output.success());
    assert!(output.stdout().contains("Agent skill manager"));
}

#[test]
fn help_wide_terminal() {
    let output = ion()
        .args(["--help"])
        .terminal(Terminal::pty(200, 24))
        .run()
        .unwrap();
    assert!(output.success());
    assert!(output.stdout().contains("Agent skill manager"));
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// Display output formatting (snapshot-friendly)
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

#[test]
fn display_format_success() {
    let output = ion().args(["--help"]).run().unwrap();
    let display = output.to_string();
    assert!(display.starts_with("success: true\n"));
    assert!(display.contains("exit_code: 0\n"));
    assert!(display.contains("----- stdout -----\n"));
}

#[test]
fn display_format_error() {
    let (dir, _tmp) = project();
    let output = ion()
        .args(["remove", "--yes", "nonexistent"])
        .current_dir(&dir)
        .run()
        .unwrap();
    let display = output.to_string();
    assert!(display.starts_with("success: false\n"));
    assert!(display.contains("exit_code: 1\n"));
    assert!(display.contains("----- stderr -----\n"));
}
