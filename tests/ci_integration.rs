use std::process::Command;

fn ion_cmd() -> Command {
    Command::new(env!("CARGO_BIN_EXE_ion"))
}

#[test]
fn ci_help_is_exposed() {
    let output = ion_cmd().args(["ci", "--help"]).output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(output.status.success());
    assert!(stdout.contains("CI/CD"));
}

#[test]
fn init_bin_ci_creates_workflows() {
    let base = tempfile::tempdir().unwrap();
    let target = base.path().join("my-ci-skill");
    std::fs::create_dir(&target).unwrap();

    let output = ion_cmd()
        .args(["init", "--bin", "--ci", target.to_str().unwrap()])
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(output.status.success(), "stdout={stdout}\nstderr={stderr}");

    // Binary project files
    assert!(target.join("Cargo.toml").exists());
    assert!(target.join("src/main.rs").exists());
    assert!(target.join("SKILL.md").exists());

    // CI/CD files
    assert!(target.join(".github/workflows/ci.yml").exists());
    assert!(target.join(".github/workflows/release.yml").exists());
    assert!(target.join(".github/workflows/release-plz.yml").exists());
    assert!(target.join("release-plz.toml").exists());

    // release.yml should reference the binary name
    let release = std::fs::read_to_string(target.join(".github/workflows/release.yml")).unwrap();
    assert!(
        release.contains("my-ci-skill-${VERSION}"),
        "release.yml should use the binary name in asset packaging"
    );
    assert!(
        !release.contains("{name}"),
        "release.yml should not contain unsubstituted placeholders"
    );
}

#[test]
fn init_bin_without_ci_has_no_workflows() {
    let base = tempfile::tempdir().unwrap();
    let target = base.path().join("no-ci-skill");
    std::fs::create_dir(&target).unwrap();

    let output = ion_cmd()
        .args(["init", "--bin", target.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(output.status.success());

    assert!(!target.join(".github").exists());
    assert!(!target.join("release-plz.toml").exists());
}

#[test]
fn ci_standalone_requires_cargo_toml() {
    let dir = tempfile::tempdir().unwrap();

    let output = ion_cmd()
        .args(["ci"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Cargo.toml"));
}

#[test]
fn ci_standalone_creates_workflows() {
    let base = tempfile::tempdir().unwrap();
    let target = base.path().join("existing-skill");
    std::fs::create_dir(&target).unwrap();

    // First scaffold a binary project
    let output = ion_cmd()
        .args(["init", "--bin", target.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(output.status.success());

    // Now run `ion ci` in that project
    let output = ion_cmd()
        .args(["ci"])
        .current_dir(&target)
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(output.status.success(), "stdout={stdout}\nstderr={stderr}");

    assert!(target.join(".github/workflows/ci.yml").exists());
    assert!(target.join(".github/workflows/release.yml").exists());
    assert!(target.join(".github/workflows/release-plz.yml").exists());
    assert!(target.join("release-plz.toml").exists());

    // Verify the binary name was read from Cargo.toml
    let release = std::fs::read_to_string(target.join(".github/workflows/release.yml")).unwrap();
    assert!(release.contains("existing-skill-${VERSION}"));
}

#[test]
fn ci_standalone_errors_without_force_if_exists() {
    let base = tempfile::tempdir().unwrap();
    let target = base.path().join("dup-ci-skill");
    std::fs::create_dir(&target).unwrap();

    // Scaffold with CI
    let output = ion_cmd()
        .args(["init", "--bin", "--ci", target.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(output.status.success());

    // Running `ion ci` again should fail
    let output = ion_cmd()
        .args(["ci"])
        .current_dir(&target)
        .output()
        .unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("already exists"));
}

#[test]
fn ci_standalone_force_overwrites() {
    let base = tempfile::tempdir().unwrap();
    let target = base.path().join("force-ci-skill");
    std::fs::create_dir(&target).unwrap();

    // Scaffold with CI
    let output = ion_cmd()
        .args(["init", "--bin", "--ci", target.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(output.status.success());

    // Running `ion ci --force` should succeed
    let output = ion_cmd()
        .args(["ci", "--force"])
        .current_dir(&target)
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(output.status.success(), "stdout={stdout}\nstderr={stderr}");
}

#[test]
fn ci_flag_without_bin_errors() {
    let dir = tempfile::tempdir().unwrap();

    let output = ion_cmd()
        .args(["init", "--ci"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("--bin"));
}

#[test]
fn ci_json_output() {
    let base = tempfile::tempdir().unwrap();
    let target = base.path().join("json-ci-skill");
    std::fs::create_dir(&target).unwrap();

    // Scaffold project first
    let output = ion_cmd()
        .args(["init", "--bin", target.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(output.status.success());

    // Run ci with --json
    let output = ion_cmd()
        .args(["--json", "ci"])
        .current_dir(&target)
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(output.status.success(), "stdout={stdout}\nstderr={stderr}");

    let parsed: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(parsed["data"]["name"], "json-ci-skill");
    assert!(parsed["data"]["files"].is_array());
}
