use std::process::Command;

fn ion_cmd() -> Command {
    Command::new(env!("CARGO_BIN_EXE_ion"))
}

/// Create a bare-bones git repo at `path` with a valid SKILL.md and return the HEAD commit SHA.
fn create_upstream_repo(path: &std::path::Path, skill_name: &str) -> String {
    std::fs::create_dir_all(path).unwrap();

    Command::new("git")
        .args(["init"])
        .current_dir(path)
        .output()
        .unwrap();
    Command::new("git")
        .args(["config", "user.email", "test@test.com"])
        .current_dir(path)
        .output()
        .unwrap();
    Command::new("git")
        .args(["config", "user.name", "Test"])
        .current_dir(path)
        .output()
        .unwrap();

    std::fs::write(
        path.join("SKILL.md"),
        format!(
            "---\nname: {skill_name}\ndescription: A test skill for updates.\n---\n\n# Test\n\nBody text here.\n"
        ),
    )
    .unwrap();

    Command::new("git")
        .args(["add", "."])
        .current_dir(path)
        .output()
        .unwrap();
    Command::new("git")
        .args(["commit", "-m", "initial"])
        .current_dir(path)
        .output()
        .unwrap();

    get_head_sha(path)
}

/// Get the HEAD commit SHA of a git repo.
fn get_head_sha(repo: &std::path::Path) -> String {
    let output = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(repo)
        .output()
        .unwrap();
    String::from_utf8_lossy(&output.stdout).trim().to_string()
}

/// Add a new commit to an upstream repo by updating SKILL.md.
fn push_upstream_commit(repo: &std::path::Path, skill_name: &str, body: &str) {
    std::fs::write(
        repo.join("SKILL.md"),
        format!(
            "---\nname: {skill_name}\ndescription: A test skill for updates.\n---\n\n# Test\n\n{body}\n"
        ),
    )
    .unwrap();

    Command::new("git")
        .args(["add", "."])
        .current_dir(repo)
        .output()
        .unwrap();
    Command::new("git")
        .args(["commit", "-m", "update content"])
        .current_dir(repo)
        .output()
        .unwrap();
}

/// Read the Ion.lock file and parse it.
fn read_lockfile(project: &std::path::Path) -> ion_skill::lockfile::Lockfile {
    ion_skill::lockfile::Lockfile::from_file(&project.join("Ion.lock")).unwrap()
}

#[test]
fn update_git_skill_pulls_latest_commit() {
    let tmp = tempfile::tempdir().unwrap();
    let upstream = tmp.path().join("upstream");
    let project = tmp.path().join("project");
    std::fs::create_dir_all(&project).unwrap();

    // 1. Create upstream git repo with valid SKILL.md
    create_upstream_repo(&upstream, "test-skill");

    // 2. Write Ion.toml pointing to the local git repo
    std::fs::write(
        project.join("Ion.toml"),
        format!(
            "[skills]\ntest-skill = {{ type = \"git\", source = \"{}\" }}\n",
            upstream.display()
        ),
    )
    .unwrap();

    // 3. Run `ion install` to install the skill
    let output = ion_cmd()
        .args(["add"])
        .current_dir(&project)
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "install failed: stdout={stdout}\nstderr={stderr}"
    );

    // 4. Read lockfile to get the original commit
    let lock_before = read_lockfile(&project);
    let skill_before = lock_before.find("test-skill").expect("skill in lockfile");
    let commit_before = skill_before
        .commit()
        .expect("commit in lockfile")
        .to_string();

    // 5. Make a new commit upstream
    push_upstream_commit(&upstream, "test-skill", "Updated body text.");

    // Verify upstream actually advanced
    let new_upstream_sha = get_head_sha(&upstream);
    assert_ne!(
        commit_before, new_upstream_sha,
        "upstream should have a new commit"
    );

    // 6. Run `ion update`
    let output = ion_cmd()
        .args(["update"])
        .current_dir(&project)
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "update failed: stdout={stdout}\nstderr={stderr}"
    );

    // 7. Read lockfile again — commit should have changed
    let lock_after = read_lockfile(&project);
    let skill_after = lock_after
        .find("test-skill")
        .expect("skill in lockfile after update");
    let commit_after = skill_after
        .commit()
        .expect("commit after update")
        .to_string();

    assert_ne!(
        commit_before, commit_after,
        "lockfile commit should have changed after update"
    );
    assert_eq!(
        commit_after, new_upstream_sha,
        "lockfile commit should match upstream HEAD"
    );
}

#[test]
fn update_skips_pinned_git_skill() {
    let tmp = tempfile::tempdir().unwrap();
    let upstream = tmp.path().join("upstream");
    let project = tmp.path().join("project");
    std::fs::create_dir_all(&project).unwrap();

    // 1. Create upstream git repo
    let initial_sha = create_upstream_repo(&upstream, "pinned-skill");

    // 2. Write Ion.toml with a pinned rev
    std::fs::write(
        project.join("Ion.toml"),
        format!(
            "[skills]\npinned-skill = {{ type = \"git\", source = \"{}\", rev = \"{}\" }}\n",
            upstream.display(),
            initial_sha,
        ),
    )
    .unwrap();

    // 3. Run `ion install` to install from manifest
    let output = ion_cmd()
        .args(["add"])
        .current_dir(&project)
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "install failed: stdout={stdout}\nstderr={stderr}"
    );

    // Read lockfile to confirm initial state
    let lock_before = read_lockfile(&project);
    let skill_before = lock_before.find("pinned-skill").expect("skill in lockfile");
    let commit_before = skill_before
        .commit()
        .expect("commit in lockfile")
        .to_string();

    // 4. Make new commit upstream
    push_upstream_commit(
        &upstream,
        "pinned-skill",
        "New content that should be ignored.",
    );

    // 5. Run `ion update`
    let output = ion_cmd()
        .args(["update"])
        .current_dir(&project)
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "update failed: stdout={stdout}\nstderr={stderr}"
    );

    // 6. Verify stdout mentions "skipped" or "pinned"
    assert!(
        stdout.contains("skipped") || stdout.contains("pinned"),
        "expected 'skipped' or 'pinned' in output, got: {stdout}"
    );

    // 7. Verify lockfile commit hasn't changed
    let lock_after = read_lockfile(&project);
    let skill_after = lock_after
        .find("pinned-skill")
        .expect("skill in lockfile after update");
    let commit_after = skill_after
        .commit()
        .expect("commit after update")
        .to_string();

    assert_eq!(
        commit_before, commit_after,
        "pinned skill commit should not change after update"
    );
}

#[test]
fn update_preserves_old_version_on_validation_failure() {
    let tmp = tempfile::tempdir().unwrap();
    let upstream = tmp.path().join("upstream");
    let project = tmp.path().join("project");
    std::fs::create_dir_all(&project).unwrap();

    // 1. Create upstream with valid SKILL.md
    create_upstream_repo(&upstream, "fail-skill");

    // Write Ion.toml pointing to the local git repo
    std::fs::write(
        project.join("Ion.toml"),
        format!(
            "[skills]\nfail-skill = {{ type = \"git\", source = \"{}\" }}\n",
            upstream.display()
        ),
    )
    .unwrap();

    // Install via `ion install`
    let output = ion_cmd()
        .args(["add"])
        .current_dir(&project)
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "install failed: stdout={stdout}\nstderr={stderr}"
    );

    // 2. Read Ion.lock (before)
    let lock_before = read_lockfile(&project);
    let skill_before = lock_before.find("fail-skill").expect("skill in lockfile");
    let commit_before = skill_before
        .commit()
        .expect("commit in lockfile")
        .to_string();

    // 3. Push invalid update upstream (zero-width space triggers security validator)
    std::fs::write(
        upstream.join("SKILL.md"),
        "---\nname: fail-skill\ndescription: Now has injection.\n---\n\nHidden instruction \u{200B} marker.\n",
    )
    .unwrap();

    Command::new("git")
        .args(["add", "."])
        .current_dir(&upstream)
        .output()
        .unwrap();
    Command::new("git")
        .args(["commit", "-m", "add injection"])
        .current_dir(&upstream)
        .output()
        .unwrap();

    // Verify upstream advanced
    let new_upstream_sha = get_head_sha(&upstream);
    assert_ne!(commit_before, new_upstream_sha);

    // 4. Run `ion update` — should succeed overall but fail for this skill
    let output = ion_cmd()
        .args(["update"])
        .current_dir(&project)
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    // The command itself should not crash (exit 0), but the skill update should fail
    // Note: ion update may still exit 0 even with individual skill failures
    eprintln!("update stdout: {stdout}");
    eprintln!("update stderr: {stderr}");

    // 5. Read Ion.lock (after) — should be unchanged
    let lock_after = read_lockfile(&project);
    let skill_after = lock_after
        .find("fail-skill")
        .expect("skill in lockfile after update");
    let commit_after = skill_after
        .commit()
        .expect("commit after update")
        .to_string();

    assert_eq!(
        commit_before, commit_after,
        "lockfile commit should be unchanged when validation fails"
    );
}
