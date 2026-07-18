use std::process::Command;

fn ion_cmd() -> Command {
    Command::new(env!("CARGO_BIN_EXE_ion"))
}

fn write_skill(path: &std::path::Path, name: &str, body: &str) {
    std::fs::create_dir_all(path).unwrap();
    std::fs::write(
        path.join("SKILL.md"),
        format!("---\nname: {name}\ndescription: Integration test skill.\n---\n\n{body}\n"),
    )
    .unwrap();
}

#[test]
fn validate_help_is_exposed() {
    let output = ion_cmd().args(["validate", "--help"]).output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(output.status.success());
    assert!(stdout.contains("Validate local skill definitions"));
}

#[test]
fn validate_default_scans_current_dir_recursively() {
    let project = tempfile::tempdir().unwrap();
    let skill_a = project.path().join("skills/a");
    let skill_b = project.path().join("tools/b");
    write_skill(&skill_a, "skill-a", "Safe body");
    write_skill(&skill_b, "skill-b", "Safe body");

    let output = ion_cmd()
        .args(["skill", "validate"])
        .current_dir(project.path())
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(output.status.success(), "stdout={stdout}\nstderr={stderr}");
    assert!(stdout.contains("Validating 2 skill(s)"));
    assert!(stdout.contains("skills/a/SKILL.md") || stdout.contains("tools/b/SKILL.md"));
}

#[test]
fn validate_single_skill_path() {
    let project = tempfile::tempdir().unwrap();
    let one = project.path().join("one");
    let two = project.path().join("two");
    write_skill(&one, "skill-one", "Safe body");
    write_skill(&two, "skill-two", "Hidden \u{200B} marker");

    let output = ion_cmd()
        .args(["skill", "validate", one.to_str().unwrap()])
        .current_dir(project.path())
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(output.status.success(), "stdout={stdout}\nstderr={stderr}");
    assert!(stdout.contains("one/SKILL.md"));
    assert!(!stdout.contains("two/SKILL.md"));
}

#[test]
fn validate_returns_nonzero_when_any_error_exists() {
    let project = tempfile::tempdir().unwrap();
    let bad = project.path().join("bad");
    write_skill(&bad, "skill-bad", "Hidden \u{200B} marker");

    let output = ion_cmd()
        .args(["skill", "validate", project.path().to_str().unwrap()])
        .current_dir(project.path())
        .output()
        .unwrap();

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(!output.status.success());
    assert!(stderr.contains("Validation failed"));
}

#[test]
fn validate_ignores_tool_names_in_prose() {
    // Tool names like Agent/Read/Write are common English words. Using them in
    // prose (not inline code) must not trigger the tool-declaration warning,
    // even when allowed-tools omits them.
    let project = tempfile::tempdir().unwrap();
    let skill = project.path().join("prose");
    write_skill(
        &skill,
        "prose-skill",
        "**Agent self-sufficiency**: Read the output and Write your notes, then Edit.",
    );

    let output = ion_cmd()
        .args(["skill", "validate", skill.to_str().unwrap()])
        .current_dir(project.path())
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(output.status.success(), "stdout={stdout}\nstderr={stderr}");
    assert!(
        !stdout.contains("tool-declaration"),
        "prose tool-words should not warn: {stdout}"
    );
    assert!(stdout.contains("0 warning(s)"), "stdout={stdout}");
}

#[test]
fn validate_flags_undeclared_tool_in_inline_code() {
    // A genuine reference (inline code) to a tool that isn't declared should
    // still surface the tool-declaration warning.
    let project = tempfile::tempdir().unwrap();
    let skill = project.path().join("coded");
    write_skill(
        &skill,
        "coded-skill",
        "Spawn a subagent with the `Agent` tool to fan out.",
    );

    let output = ion_cmd()
        .args(["skill", "validate", skill.to_str().unwrap()])
        .current_dir(project.path())
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("tool-declaration"), "stdout={stdout}");
    assert!(stdout.contains("Agent"), "stdout={stdout}");
}
