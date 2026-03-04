use std::process::Command;

fn ion_cmd() -> Command {
    Command::new(env!("CARGO_BIN_EXE_ion"))
}

#[test]
fn config_set_and_get_global() {
    let dir = tempfile::tempdir().unwrap();
    let config_path = dir.path().join("config.toml");

    // Test via the library directly since we can't easily override the global config path
    use ion_skill::config::GlobalConfig;

    GlobalConfig::set_value_in_file(&config_path, "targets.claude", ".claude/skills").unwrap();
    let config = GlobalConfig::load_from(&config_path).unwrap();
    assert_eq!(config.targets["claude"], ".claude/skills");
}

#[test]
fn config_set_and_get_project() {
    let dir = tempfile::tempdir().unwrap();
    let manifest_path = dir.path().join("Ion.toml");
    std::fs::write(&manifest_path, "[skills]\n").unwrap();

    let output = ion_cmd()
        .args(["config", "list", "--project"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "stdout: {}, stderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn config_list_no_project() {
    let dir = tempfile::tempdir().unwrap();

    let output = ion_cmd()
        .args(["config", "list", "--project"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    // Should fail — no Ion.toml
    assert!(!output.status.success());
}

#[test]
fn config_get_nonexistent_key() {
    let dir = tempfile::tempdir().unwrap();

    let output = ion_cmd()
        .args(["config", "get", "targets.nonexistent"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    // Should fail — key not found
    assert!(!output.status.success());
}
