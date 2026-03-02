use std::process::Command;

fn ion_cmd() -> Command {
    Command::new(env!("CARGO_BIN_EXE_ion"))
}

#[test]
fn search_shows_help() {
    let output = ion_cmd()
        .args(["search", "--help"])
        .output()
        .expect("failed to run ion");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Search for skills"));
    assert!(stdout.contains("--agent"));
    assert!(stdout.contains("--interactive"));
    assert!(stdout.contains("--source"));
    assert!(stdout.contains("--limit"));
}

#[test]
fn search_no_registries_falls_through_gracefully() {
    let output = ion_cmd()
        .args(["search", "nonexistent-skill-xyz-12345"])
        .output()
        .expect("failed to run ion");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(!stderr.contains("panicked"));
}

#[test]
fn search_unknown_source_errors() {
    let output = ion_cmd()
        .args(["search", "test", "--source", "nonexistent"])
        .output()
        .expect("failed to run ion");
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Unknown source"));
}
