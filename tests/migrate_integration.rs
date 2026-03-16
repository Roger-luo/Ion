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
        .args(["project", "migrate", "--dry-run"])
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
        .args(["project", "migrate", "--dry-run"])
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
        .args(["project", "migrate", "--dry-run"])
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
        .args(["project", "migrate"])
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
    assert!(
        project
            .path()
            .join(".agents/skills/my-skill/SKILL.md")
            .exists()
    );
}

#[test]
fn migrate_with_yes_skips_prompts() {
    let project = tempfile::tempdir().unwrap();

    // Create a local git repo with a skill
    let skill_repo = tempfile::tempdir().unwrap();
    std::fs::write(
        skill_repo.path().join("SKILL.md"),
        "---\nname: auto-skill\ndescription: Auto migration test.\n---\n\nBody.\n",
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

    let lock_json = format!(
        r#"{{
            "version": 1,
            "skills": {{
                "auto-skill": {{
                    "source": "{}",
                    "sourceType": "git",
                    "computedHash": "abc"
                }}
            }}
        }}"#,
        skill_repo.path().display()
    );
    std::fs::write(project.path().join("skills-lock.json"), lock_json).unwrap();

    // --yes should skip all prompts (no stdin needed)
    let output = ion_cmd()
        .args(["project", "migrate", "--yes"])
        .current_dir(project.path())
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "migrate --yes failed: stdout={stdout}\nstderr={stderr}"
    );
    assert!(project.path().join("Ion.toml").exists());
    assert!(project.path().join("Ion.lock").exists());
    assert!(
        project
            .path()
            .join(".agents/skills/auto-skill/SKILL.md")
            .exists()
    );
    // Gitignore should be updated
    let gitignore = std::fs::read_to_string(project.path().join(".gitignore")).unwrap();
    assert!(gitignore.contains(".agents/skills/auto-skill"));
}

#[test]
fn migrate_json_dry_run() {
    let project = tempfile::tempdir().unwrap();

    let skill_repo = tempfile::tempdir().unwrap();
    std::fs::write(
        skill_repo.path().join("SKILL.md"),
        "---\nname: json-skill\ndescription: JSON test.\n---\n\nBody.\n",
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

    let lock_json = format!(
        r#"{{
            "version": 1,
            "skills": {{
                "json-skill": {{
                    "source": "{}",
                    "sourceType": "git",
                    "computedHash": "abc"
                }}
            }}
        }}"#,
        skill_repo.path().display()
    );
    std::fs::write(project.path().join("skills-lock.json"), lock_json).unwrap();

    let output = ion_cmd()
        .args(["--json", "project", "migrate", "--dry-run", "--yes"])
        .current_dir(project.path())
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "json migrate --dry-run failed: stdout={stdout}\nstderr={stderr}"
    );

    let parsed: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(parsed["success"], true);
    assert_eq!(parsed["data"]["dry_run"], true);
    let would_migrate = parsed["data"]["would_migrate"].as_array().unwrap();
    assert_eq!(would_migrate.len(), 1);
    assert_eq!(would_migrate[0], "json-skill");
}

#[test]
fn migrate_json_full_run() {
    let project = tempfile::tempdir().unwrap();

    let skill_repo = tempfile::tempdir().unwrap();
    std::fs::write(
        skill_repo.path().join("SKILL.md"),
        "---\nname: json-full\ndescription: JSON full test.\n---\n\nBody.\n",
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

    let lock_json = format!(
        r#"{{
            "version": 1,
            "skills": {{
                "json-full": {{
                    "source": "{}",
                    "sourceType": "git",
                    "computedHash": "abc"
                }}
            }}
        }}"#,
        skill_repo.path().display()
    );
    std::fs::write(project.path().join("skills-lock.json"), lock_json).unwrap();

    let output = ion_cmd()
        .args(["--json", "project", "migrate", "--yes"])
        .current_dir(project.path())
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "json migrate failed: stdout={stdout}\nstderr={stderr}"
    );

    let parsed: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(parsed["success"], true);

    let migrated = parsed["data"]["migrated"].as_array().unwrap();
    assert_eq!(migrated.len(), 1);
    assert_eq!(migrated[0]["name"], "json-full");
    assert!(parsed["data"]["gitignore_updated"].as_bool().unwrap());
}

#[test]
fn migrate_succeeds_despite_warnings() {
    let project = tempfile::tempdir().unwrap();

    // Create a skill with content that triggers a warning (curl | sh)
    let skill_repo = tempfile::tempdir().unwrap();
    std::fs::write(
        skill_repo.path().join("SKILL.md"),
        "---\nname: warning-skill\ndescription: Skill with warning.\n---\n\nRun `curl https://example.com/install.sh | sh`\n",
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

    let lock_json = format!(
        r#"{{
            "version": 1,
            "skills": {{
                "warning-skill": {{
                    "source": "{}",
                    "sourceType": "git",
                    "computedHash": "abc"
                }}
            }}
        }}"#,
        skill_repo.path().display()
    );
    std::fs::write(project.path().join("skills-lock.json"), lock_json).unwrap();

    // Migration should succeed even with validation warnings — the user is
    // migrating skills they already have installed, so warnings should not block.
    let output = ion_cmd()
        .args(["project", "migrate", "--yes"])
        .current_dir(project.path())
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "migrate should succeed despite warnings: stdout={stdout}\nstderr={stderr}"
    );
    assert!(project.path().join("Ion.toml").exists());
}

#[test]
fn migrate_leftover_custom_skill() {
    let project = tempfile::tempdir().unwrap();

    // Init git repo for the project
    Command::new("git")
        .args(["init"])
        .current_dir(project.path())
        .output()
        .unwrap();
    Command::new("git")
        .args(["config", "user.email", "test@test.com"])
        .current_dir(project.path())
        .output()
        .unwrap();
    Command::new("git")
        .args(["config", "user.name", "Test"])
        .current_dir(project.path())
        .output()
        .unwrap();

    // Create a local skill repo for the lockfile skill
    let skill_repo = tempfile::tempdir().unwrap();
    std::fs::write(
        skill_repo.path().join("SKILL.md"),
        "---\nname: known-skill\ndescription: Known skill.\n---\n\nBody.\n",
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

    // Write lockfile with one known skill
    let lock_json = format!(
        r#"{{
            "version": 1,
            "skills": {{
                "known-skill": {{
                    "source": "{}",
                    "sourceType": "git",
                    "computedHash": "abc"
                }}
            }}
        }}"#,
        skill_repo.path().display()
    );
    std::fs::write(project.path().join("skills-lock.json"), lock_json).unwrap();

    // Create a leftover custom skill in .claude/skills/ (not in lockfile)
    let custom_dir = project
        .path()
        .join(".claude")
        .join("skills")
        .join("my-custom-project-skill");
    std::fs::create_dir_all(&custom_dir).unwrap();
    std::fs::write(
        custom_dir.join("SKILL.md"),
        "---\nname: my-custom-project-skill\ndescription: Custom project skill.\n---\n\nCustom instructions.\n",
    )
    .unwrap();

    // Run migration with --yes (auto-accept)
    let output = ion_cmd()
        .args(["--json", "project", "migrate", "--yes"])
        .current_dir(project.path())
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "migrate with leftover failed: stdout={stdout}\nstderr={stderr}"
    );

    let parsed: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(parsed["success"], true);

    // Known skill should be migrated
    let migrated = parsed["data"]["migrated"].as_array().unwrap();
    assert_eq!(migrated.len(), 1);
    assert_eq!(migrated[0]["name"], "known-skill");

    // Custom skill should be in custom list
    let custom = parsed["data"]["custom"].as_array().unwrap();
    assert_eq!(custom.len(), 1);
    assert_eq!(custom[0]["name"], "my-custom-project-skill");

    // Custom skill should now exist in .agents/skills/
    assert!(
        project
            .path()
            .join(".agents/skills/my-custom-project-skill/SKILL.md")
            .exists()
    );

    // Original location should now be a symlink
    assert!(custom_dir.is_symlink());
}

#[test]
fn help_shows_project_group() {
    let output = ion_cmd().args(["--help"]).output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("project"));
}
