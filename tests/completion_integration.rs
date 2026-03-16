use std::process::Command;

fn ion_cmd() -> Command {
    Command::new(env!("CARGO_BIN_EXE_ion"))
}

#[test]
fn completion_bash_produces_output() {
    let output = ion_cmd()
        .args(["completion", "bash"])
        .output()
        .expect("failed to run ion");
    assert!(output.status.success(), "exit code was not 0");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(!stdout.is_empty(), "completion output was empty");
    assert!(
        stdout.contains("ion"),
        "completion should reference the binary name"
    );
}

#[test]
fn completion_zsh_produces_output() {
    let output = ion_cmd()
        .args(["completion", "zsh"])
        .output()
        .expect("failed to run ion");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(!stdout.is_empty());
}

#[test]
fn completion_fish_produces_output() {
    let output = ion_cmd()
        .args(["completion", "fish"])
        .output()
        .expect("failed to run ion");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(!stdout.is_empty());
}

#[test]
fn completion_invalid_shell_fails() {
    let output = ion_cmd()
        .args(["completion", "nushell"])
        .output()
        .expect("failed to run ion");
    assert!(!output.status.success(), "should reject unknown shells");
}

#[test]
fn completion_missing_shell_fails() {
    let output = ion_cmd()
        .args(["completion"])
        .output()
        .expect("failed to run ion");
    assert!(!output.status.success(), "should require a shell argument");
}

#[test]
fn completion_help_shows_setup_instructions() {
    let output = ion_cmd()
        .args(["completion", "--help"])
        .output()
        .expect("failed to run ion");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Setup:"),
        "help should include setup section"
    );
    assert!(stdout.contains(".bashrc"), "help should show bash setup");
    assert!(stdout.contains(".zfunc"), "help should show zsh setup");
    assert!(
        stdout.contains("completions/ion.fish"),
        "help should show fish setup"
    );
}
