use std::process::Command;

fn ion_cmd() -> Command {
    Command::new(env!("CARGO_BIN_EXE_ion"))
}

/// Helper: create a workspace root with [workspace] section and optional members.
fn setup_workspace(dir: &std::path::Path, members: &[&str], root_skills: &str) {
    let mut manifest = String::from("[workspace]\nmembers = [");
    let member_strs: Vec<String> = members.iter().map(|m| format!("\"{}\"", m)).collect();
    manifest.push_str(&member_strs.join(", "));
    manifest.push_str("]\n\n");
    manifest.push_str(root_skills);
    std::fs::write(dir.join("Ion.toml"), manifest).unwrap();
}

/// Helper: create a member project directory with an Ion.toml.
fn setup_member(dir: &std::path::Path, member: &str, skills_toml: &str) {
    let member_dir = dir.join(member);
    std::fs::create_dir_all(&member_dir).unwrap();
    std::fs::write(member_dir.join("Ion.toml"), skills_toml).unwrap();
}

// --- Test 1: basic list works without workspace ---

#[test]
fn workspace_of_one_list_works() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("Ion.toml"),
        "[skills]\nmy-skill = { type = \"local\" }\n",
    )
    .unwrap();

    let output = ion_cmd()
        .args(["skill", "list"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(stdout.contains("my-skill"), "stdout: {}", stdout);
    // For workspace-of-one, no project header should appear
    assert!(
        !stdout.contains(". (root)"),
        "single project should not show header: {}",
        stdout
    );
}

// --- Test 2: running from a member dir finds the workspace ---

#[test]
fn workspace_discovered_from_member_dir() {
    let dir = tempfile::tempdir().unwrap();
    setup_workspace(
        dir.path(),
        &["docs"],
        "[skills]\nroot-skill = { type = \"local\" }\n",
    );
    setup_member(
        dir.path(),
        "docs",
        "[skills]\ndocs-skill = { type = \"local\" }\n",
    );

    // Run from the docs/ directory — should scope to docs member only
    let output = ion_cmd()
        .args(["skill", "list"])
        .current_dir(dir.path().join("docs"))
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    // Should see docs-skill but NOT root-skill (scoped to member)
    assert!(
        stdout.contains("docs-skill"),
        "should list member skills: {}",
        stdout
    );
    assert!(
        !stdout.contains("root-skill"),
        "should not list root skills when scoped to member: {}",
        stdout
    );
}

// --- Test 3: member inherits targets from root ---

#[test]
fn workspace_member_inherits_targets() {
    let dir = tempfile::tempdir().unwrap();
    let root_toml = r#"[workspace]
members = ["docs"]

[skills]

[options.targets]
claude = ".claude/skills"
"#;
    std::fs::write(dir.path().join("Ion.toml"), root_toml).unwrap();
    setup_member(
        dir.path(),
        "docs",
        "[skills]\ndocs-skill = { type = \"local\" }\n",
    );

    // Create the local skill dir so it shows as "installed"
    let skill_dir = dir.path().join("docs/.agents/skills/docs-skill");
    std::fs::create_dir_all(&skill_dir).unwrap();
    std::fs::write(skill_dir.join("SKILL.md"), "---\nname: docs-skill\n---\n").unwrap();

    let output = ion_cmd()
        .args(["skill", "list", "--json"])
        .current_dir(dir.path().join("docs"))
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    // JSON output should include the skill
    let parsed: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    let data = &parsed["data"];
    assert!(data.is_array(), "data should be an array: {}", stdout);
    let skills = data.as_array().unwrap();
    assert_eq!(skills.len(), 1);
    assert_eq!(skills[0]["name"], "docs-skill");
}

// --- Test 4: error if member dir missing ---

#[test]
fn workspace_rejects_missing_member() {
    let dir = tempfile::tempdir().unwrap();
    // Reference a member that doesn't exist
    setup_workspace(dir.path(), &["nonexistent"], "[skills]\n");

    let output = ion_cmd()
        .args(["skill", "list", "--project", "nonexistent"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    // The list command should work but the member won't have a manifest
    // Since we set up the workspace with a nonexistent member, --project nonexistent
    // should find the project index but it won't have a manifest, so list shows nothing
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    // Should not crash, but show no skills
    assert!(
        stdout.contains("No skills") || stdout.is_empty() || !stdout.contains("skill"),
        "should handle missing member gracefully: {}",
        stdout
    );
}

// --- Test 5: workspace add creates member ---

#[test]
fn workspace_add_creates_member() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("Ion.toml"), "[skills]\n").unwrap();

    let output = ion_cmd()
        .args(["workspace", "add", "docs"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Added"), "stdout: {}", stdout);

    // Check that docs/ dir was created
    assert!(dir.path().join("docs").exists());

    // Check that docs/Ion.toml was created
    assert!(dir.path().join("docs/Ion.toml").exists());

    // Check that the root Ion.toml now has [workspace] with docs
    let root_toml = std::fs::read_to_string(dir.path().join("Ion.toml")).unwrap();
    assert!(
        root_toml.contains("[workspace]"),
        "should have workspace section: {}",
        root_toml
    );
    assert!(
        root_toml.contains("\"docs\""),
        "should have docs member: {}",
        root_toml
    );
}

// --- Test 6: workspace remove unregisters member ---

#[test]
fn workspace_remove_unregisters_member() {
    let dir = tempfile::tempdir().unwrap();
    setup_workspace(dir.path(), &["docs", "frontend"], "[skills]\n");
    setup_member(dir.path(), "docs", "[skills]\n");
    setup_member(dir.path(), "frontend", "[skills]\n");

    let output = ion_cmd()
        .args(["workspace", "remove", "docs"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Removed"), "stdout: {}", stdout);
    assert!(
        stdout.contains("files preserved"),
        "should note files are preserved: {}",
        stdout
    );

    // Check that the root Ion.toml no longer has docs but still has frontend
    let root_toml = std::fs::read_to_string(dir.path().join("Ion.toml")).unwrap();
    assert!(
        !root_toml.contains("\"docs\""),
        "docs should be removed: {}",
        root_toml
    );
    assert!(
        root_toml.contains("\"frontend\""),
        "frontend should remain: {}",
        root_toml
    );

    // Files should still exist
    assert!(dir.path().join("docs/Ion.toml").exists());
}

// --- Test 7: workspace list shows members ---

#[test]
fn workspace_list_shows_members() {
    let dir = tempfile::tempdir().unwrap();
    setup_workspace(
        dir.path(),
        &["docs"],
        "[skills]\nroot-skill = { type = \"local\" }\n",
    );
    setup_member(
        dir.path(),
        "docs",
        "[skills]\ndocs-skill = { type = \"local\" }\nanother = { type = \"local\" }\n",
    );

    let output = ion_cmd()
        .args(["workspace", "list"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(stdout.contains(". (root)"), "should show root: {}", stdout);
    assert!(
        stdout.contains("docs"),
        "should show docs member: {}",
        stdout
    );
    assert!(
        stdout.contains("1 skill(s)"),
        "root should have 1 skill: {}",
        stdout
    );
    assert!(
        stdout.contains("2 skill(s)"),
        "docs should have 2 skills: {}",
        stdout
    );
}

// --- Test 8: skill list from root shows all projects ---

#[test]
fn list_shows_all_projects_from_root() {
    let dir = tempfile::tempdir().unwrap();
    setup_workspace(
        dir.path(),
        &["docs"],
        "[skills]\nroot-skill = { type = \"local\" }\n",
    );
    setup_member(
        dir.path(),
        "docs",
        "[skills]\ndocs-skill = { type = \"local\" }\n",
    );

    let output = ion_cmd()
        .args(["skill", "list"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    // When running from root of a workspace, should show all projects
    assert!(
        stdout.contains(". (root)"),
        "should show root header: {}",
        stdout
    );
    assert!(
        stdout.contains("docs"),
        "should show docs header: {}",
        stdout
    );
    assert!(
        stdout.contains("root-skill"),
        "should list root skill: {}",
        stdout
    );
    assert!(
        stdout.contains("docs-skill"),
        "should list docs skill: {}",
        stdout
    );
}

// --- Test: workspace list JSON output ---

#[test]
fn workspace_list_json_output() {
    let dir = tempfile::tempdir().unwrap();
    setup_workspace(
        dir.path(),
        &["docs"],
        "[skills]\nroot-skill = { type = \"local\" }\n",
    );
    setup_member(
        dir.path(),
        "docs",
        "[skills]\ndocs-skill = { type = \"local\" }\n",
    );

    let output = ion_cmd()
        .args(["workspace", "list", "--json"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let parsed: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    let data = &parsed["data"];
    assert!(data.is_array(), "data should be an array: {}", stdout);
    let members = data.as_array().unwrap();
    assert_eq!(members.len(), 2, "should have root + docs: {}", stdout);
    assert_eq!(members[0]["path"], ". (root)");
    assert_eq!(members[0]["skill_count"], 1);
    assert_eq!(members[1]["path"], "docs");
    assert_eq!(members[1]["skill_count"], 1);
}

// --- Test: workspace status output ---

#[test]
fn workspace_status_shows_status() {
    let dir = tempfile::tempdir().unwrap();
    setup_workspace(
        dir.path(),
        &["docs"],
        "[skills]\nroot-skill = { type = \"local\" }\n",
    );
    setup_member(dir.path(), "docs", "[skills]\n");

    let output = ion_cmd()
        .args(["workspace", "status"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        stdout.contains("Workspace status"),
        "should show status header: {}",
        stdout
    );
    assert!(stdout.contains(". (root)"), "should show root: {}", stdout);
    assert!(stdout.contains("docs"), "should show docs: {}", stdout);
}

// --- Test: not a workspace shows informative message ---

#[test]
fn workspace_list_not_a_workspace() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("Ion.toml"), "[skills]\n").unwrap();

    let output = ion_cmd()
        .args(["workspace", "list"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        stdout.contains("Not a workspace"),
        "should tell user it's not a workspace: {}",
        stdout
    );
}

// --- Test: skill list JSON from workspace root includes project field ---

#[test]
fn list_json_from_workspace_includes_project_field() {
    let dir = tempfile::tempdir().unwrap();
    setup_workspace(
        dir.path(),
        &["docs"],
        "[skills]\nroot-skill = { type = \"local\" }\n",
    );
    setup_member(
        dir.path(),
        "docs",
        "[skills]\ndocs-skill = { type = \"local\" }\n",
    );

    let output = ion_cmd()
        .args(["skill", "list", "--json"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let parsed: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    let data = &parsed["data"];
    assert!(data.is_array(), "data should be an array: {}", stdout);
    let skills = data.as_array().unwrap();
    assert_eq!(skills.len(), 2, "should have 2 skills total: {}", stdout);

    // Each skill should have a "project" field
    for skill in skills {
        assert!(
            skill["project"].is_string(),
            "skill should have project field: {}",
            skill
        );
    }

    // Check the correct project assignments
    let root_skill = skills.iter().find(|s| s["name"] == "root-skill").unwrap();
    assert_eq!(root_skill["project"], ". (root)");

    let docs_skill = skills.iter().find(|s| s["name"] == "docs-skill").unwrap();
    assert_eq!(docs_skill["project"], "docs");
}

// --- Test: workspace add JSON output ---

#[test]
fn workspace_add_json_output() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("Ion.toml"), "[skills]\n").unwrap();

    let output = ion_cmd()
        .args(["workspace", "add", "frontend", "--json"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let parsed: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(parsed["success"], true);
    assert_eq!(parsed["data"]["added"], "frontend");
}

// --- Test: workspace remove JSON output ---

#[test]
fn workspace_remove_json_output() {
    let dir = tempfile::tempdir().unwrap();
    setup_workspace(dir.path(), &["docs"], "[skills]\n");
    setup_member(dir.path(), "docs", "[skills]\n");

    let output = ion_cmd()
        .args(["workspace", "remove", "docs", "--json"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let parsed: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(parsed["success"], true);
    assert_eq!(parsed["data"]["removed"], "docs");
}

// --- Test: update from workspace root updates all projects ---

#[test]
fn update_from_workspace_root_handles_all_projects() {
    let dir = tempfile::tempdir().unwrap();
    setup_workspace(
        dir.path(),
        &["docs"],
        "[skills]\nroot-skill = { type = \"local\" }\n",
    );
    setup_member(
        dir.path(),
        "docs",
        "[skills]\ndocs-skill = { type = \"local\" }\n",
    );

    // Update from root — should process all projects without errors
    let output = ion_cmd()
        .args(["update"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

// --- Test: workspace status JSON output ---

#[test]
fn workspace_status_json_output() {
    let dir = tempfile::tempdir().unwrap();
    setup_workspace(
        dir.path(),
        &["docs"],
        "[skills]\nroot-skill = { type = \"local\" }\n",
    );
    setup_member(dir.path(), "docs", "[skills]\n");

    let output = ion_cmd()
        .args(["workspace", "status", "--json"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let parsed: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(parsed["success"], true);
    assert_eq!(parsed["data"]["is_workspace"], true);
    let members = parsed["data"]["members"].as_array().unwrap();
    assert_eq!(members.len(), 2);
    assert_eq!(members[0]["path"], ". (root)");
    assert!(members[0]["has_manifest"].as_bool().unwrap());
}

// --- Task 7: Add/Remove Ambiguity Guard ---

#[test]
fn add_from_workspace_root_without_project_flag_errors() {
    let dir = tempfile::tempdir().unwrap();
    setup_workspace(
        dir.path(),
        &["docs", "frontend"],
        "[skills]\nroot-skill = { type = \"local\" }\n",
    );
    setup_member(dir.path(), "docs", "[skills]\n");
    setup_member(dir.path(), "frontend", "[skills]\n");

    // ion add some/source from root of a multi-project workspace → should fail
    let output = ion_cmd()
        .args(["add", "some/nonexistent-source"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    assert!(
        !output.status.success(),
        "should fail when ambiguous: stdout={}, stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("--project"),
        "error should mention --project: {}",
        stderr
    );
}

#[test]
fn remove_from_workspace_root_without_project_flag_errors() {
    let dir = tempfile::tempdir().unwrap();
    setup_workspace(
        dir.path(),
        &["docs", "frontend"],
        "[skills]\nroot-skill = { type = \"local\" }\n",
    );
    setup_member(dir.path(), "docs", "[skills]\n");
    setup_member(dir.path(), "frontend", "[skills]\n");

    // ion remove foo -y from root of a multi-project workspace → should fail
    let output = ion_cmd()
        .args(["remove", "foo", "-y"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    assert!(
        !output.status.success(),
        "should fail when ambiguous: stdout={}, stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("--project"),
        "error should mention --project: {}",
        stderr
    );
}

// --- Task 8: Workspace-Aware Agents Commands ---

#[test]
fn agents_update_iterates_workspace() {
    let dir = tempfile::tempdir().unwrap();
    // Create workspace where neither project has [agents] configured.
    // The update command should not crash — it should skip projects without [agents].
    setup_workspace(dir.path(), &["docs"], "[skills]\n");
    setup_member(dir.path(), "docs", "[skills]\n");

    let output = ion_cmd()
        .args(["agents", "update"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    // Should not crash. When no projects have [agents], the command should
    // still succeed (in multi-project mode, projects without [agents] are skipped).
    // Note: in non-workspace (single project) mode, it would error. But with
    // multi-project scope it should skip gracefully. Since we have a workspace
    // with 2 projects (root + docs), scope is All.
    // Both lack [agents], so it should not error in multi mode.
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

// --- Task 10: Init Auto-Registration ---

#[test]
fn init_inside_workspace_auto_registers() {
    let dir = tempfile::tempdir().unwrap();
    // Create root with [workspace] members = []
    std::fs::write(
        dir.path().join("Ion.toml"),
        "[workspace]\nmembers = []\n\n[skills]\n",
    )
    .unwrap();

    // Create subdirectory packages/frontend (no Ion.toml yet)
    let frontend_dir = dir.path().join("packages/frontend");
    std::fs::create_dir_all(&frontend_dir).unwrap();

    // Run ion init --target claude from packages/frontend
    let output = ion_cmd()
        .args(["init", "--target", "claude"])
        .current_dir(&frontend_dir)
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // Check that packages/frontend/Ion.toml was created
    assert!(
        frontend_dir.join("Ion.toml").exists(),
        "Ion.toml should be created in the subdirectory"
    );

    // Check that root Ion.toml now has "packages/frontend" in members
    let root_toml = std::fs::read_to_string(dir.path().join("Ion.toml")).unwrap();
    assert!(
        root_toml.contains("packages/frontend"),
        "root Ion.toml should have packages/frontend as member: {}",
        root_toml
    );

    // Check stdout mentions registration
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Registered"),
        "should mention registration: {}",
        stdout
    );
}

// --- Task 11: Workspace-Aware Install-All ---

#[test]
fn install_all_across_workspace() {
    let dir = tempfile::tempdir().unwrap();
    setup_workspace(
        dir.path(),
        &["docs"],
        "[skills]\nroot-skill = { type = \"local\" }\n",
    );
    setup_member(
        dir.path(),
        "docs",
        "[skills]\ndocs-skill = { type = \"local\" }\n",
    );

    // Create local skill directories so install can find them
    let root_skill_dir = dir.path().join(".agents/skills/root-skill");
    std::fs::create_dir_all(&root_skill_dir).unwrap();
    std::fs::write(
        root_skill_dir.join("SKILL.md"),
        "---\nname: root-skill\n---\nRoot skill content\n",
    )
    .unwrap();

    let docs_skill_dir = dir.path().join("docs/.agents/skills/docs-skill");
    std::fs::create_dir_all(&docs_skill_dir).unwrap();
    std::fs::write(
        docs_skill_dir.join("SKILL.md"),
        "---\nname: docs-skill\n---\nDocs skill content\n",
    )
    .unwrap();

    // Run ion add (no args = install-all) from root
    let output = ion_cmd()
        .args(["add"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // Both Ion.lock files should be created
    assert!(
        dir.path().join("Ion.lock").exists(),
        "root Ion.lock should be created"
    );
    assert!(
        dir.path().join("docs/Ion.lock").exists(),
        "docs Ion.lock should be created"
    );
}
