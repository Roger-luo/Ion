use std::process::Command;

fn ion_cmd() -> Command {
    Command::new(env!("CARGO_BIN_EXE_ion"))
}

#[test]
fn migrate_from_lockfile_dry_run() {
    let project = tempfile::tempdir().unwrap();

    // Create a local skill repo to serve as the source
    let skill_repo = tempfile::tempdir().unwrap();
    let skill_dir = skill_repo.path().join("test-migrate");
    std::fs::create_dir(&skill_dir).unwrap();
    std::fs::write(
        skill_dir.join("SKILL.md"),
        "---\nname: test-migrate\ndescription: Migration test skill.\n---\n\nBody.\n",
    )
    .unwrap();

    // Init a git repo so it can be cloned
    Command::new("git")
        .args(["init"])
        .current_dir(skill_repo.path())
        .output()
        .unwrap();
    Command::new("git")
        .args(["add", "."])
        .current_dir(skill_repo.path())
        .output()
        .unwrap();
    Command::new("git")
        .args(["commit", "-m", "init"])
        .current_dir(skill_repo.path())
        .output()
        .unwrap();

    // Write skills-lock.json pointing at the local git repo
    let lock_json = format!(
        r#"{{
            "version": 1,
            "skills": {{
                "test-migrate": {{
                    "source": "{}",
                    "sourceType": "git",
                    "computedHash": "abc123"
                }}
            }}
        }}"#,
        skill_repo.path().display()
    );
    std::fs::write(project.path().join("skills-lock.json"), lock_json).unwrap();

    // Run dry-run migrate — should not write files
    let output = ion_cmd()
        .args(["migrate", "--dry-run"])
        .current_dir(project.path())
        .stdin(std::process::Stdio::piped())
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "migrate --dry-run failed: stdout={stdout}\nstderr={stderr}"
    );
    assert!(stdout.contains("skills-lock.json"));
    assert!(stdout.contains("Dry run"));
    assert!(!project.path().join("Ion.toml").exists());
    assert!(!project.path().join("Ion.lock").exists());
}

#[test]
fn migrate_from_directory_scan_dry_run() {
    let project = tempfile::tempdir().unwrap();

    // Create .agents/skills/dir-skill/SKILL.md
    let skill_dir = project
        .path()
        .join(".agents")
        .join("skills")
        .join("dir-skill");
    std::fs::create_dir_all(&skill_dir).unwrap();
    std::fs::write(
        skill_dir.join("SKILL.md"),
        "---\nname: dir-skill\ndescription: Directory scan test.\nmetadata:\n  version: \"1.0\"\n---\n\nBody.\n",
    )
    .unwrap();

    // Run dry-run migrate (no skills-lock.json, falls back to dir scan)
    // Pipe empty stdin so prompts for source get skipped
    let output = ion_cmd()
        .args(["migrate", "--dry-run"])
        .current_dir(project.path())
        .stdin(std::process::Stdio::piped())
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "migrate --dry-run failed: stdout={stdout}\nstderr={stderr}"
    );
    assert!(stdout.contains("scanning directories"));
    assert!(stdout.contains("1 skills"));
}

#[test]
fn migrate_no_skills_found() {
    let project = tempfile::tempdir().unwrap();

    let output = ion_cmd()
        .args(["migrate", "--dry-run"])
        .current_dir(project.path())
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(output.status.success());
    assert!(stdout.contains("No skills found"));
}

#[test]
fn migrate_with_local_git_repo() {
    let project = tempfile::tempdir().unwrap();

    // Create a local git repo with a skill at its root
    let skill_repo = tempfile::tempdir().unwrap();
    std::fs::write(
        skill_repo.path().join("SKILL.md"),
        "---\nname: my-skill\ndescription: A local repo skill.\n---\n\nDo things.\n",
    )
    .unwrap();

    Command::new("git")
        .args(["init"])
        .current_dir(skill_repo.path())
        .output()
        .unwrap();
    Command::new("git")
        .args(["add", "."])
        .current_dir(skill_repo.path())
        .output()
        .unwrap();
    Command::new("git")
        .args(["commit", "-m", "init"])
        .current_dir(skill_repo.path())
        .output()
        .unwrap();

    // Write skills-lock.json with git sourceType pointing at local repo
    let lock_json = format!(
        r#"{{
            "version": 1,
            "skills": {{
                "my-skill": {{
                    "source": "{}",
                    "sourceType": "git",
                    "computedHash": "abc"
                }}
            }}
        }}"#,
        skill_repo.path().display()
    );
    std::fs::write(project.path().join("skills-lock.json"), lock_json).unwrap();

    // Provide empty rev (just press Enter) via stdin
    let mut child = Command::new(env!("CARGO_BIN_EXE_ion"))
        .args(["migrate"])
        .current_dir(project.path())
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();

    // Write a newline to skip rev pinning prompt
    {
        use std::io::Write;
        let stdin = child.stdin.as_mut().unwrap();
        stdin.write_all(b"\n").unwrap();
    }

    let output = child.wait_with_output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "migrate failed: stdout={stdout}\nstderr={stderr}"
    );
    assert!(project.path().join("Ion.toml").exists());
    assert!(project.path().join("Ion.lock").exists());
    assert!(project
        .path()
        .join(".agents/skills/my-skill/SKILL.md")
        .exists());
}

#[test]
fn help_shows_migrate() {
    let output = ion_cmd().args(["--help"]).output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("migrate"));
}
