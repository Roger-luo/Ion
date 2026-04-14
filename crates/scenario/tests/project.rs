use std::fs;
use std::path::Path;
use std::process::Command;

use scenario::{Error, Project, Scenario};

// ── Manifest parsing ───────────────────────────────────────────────

#[test]
fn parse_manifest_full() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/with-manifest");
    let manifest = scenario::manifest::TemplateManifest::from_dir(&dir).unwrap();

    assert!(manifest.variables.contains_key("name"));
    assert_eq!(
        manifest.variables["name"].default.as_deref(),
        Some("test-skill")
    );
    assert!(manifest.variables.contains_key("description"));
    assert!(manifest.variables["description"].default.is_none());

    assert_eq!(manifest.files.optional, vec!["Ion.lock".to_string()]);

    assert_eq!(
        manifest
            .files
            .mappings
            .get("skills/SKILL.md")
            .map(|s| s.as_str()),
        Some(".agents/skills/{{name}}/SKILL.md")
    );

    assert_eq!(
        manifest
            .files
            .symlinks
            .get(".claude/skills/{{name}}")
            .map(|s| s.as_str()),
        Some("../../.agents/skills/{{name}}")
    );
}

#[test]
fn parse_manifest_minimal() {
    let tmp = tempfile::tempdir().unwrap();
    fs::write(tmp.path().join("template.toml"), "").unwrap();
    let manifest = scenario::manifest::TemplateManifest::from_dir(tmp.path()).unwrap();

    assert!(manifest.variables.is_empty());
    assert!(manifest.files.optional.is_empty());
    assert!(manifest.files.mappings.is_empty());
    assert!(manifest.files.symlinks.is_empty());
}

#[test]
fn parse_manifest_missing_file() {
    let tmp = tempfile::tempdir().unwrap();
    let result = scenario::manifest::TemplateManifest::from_dir(tmp.path());
    assert!(result.is_err());
}

// ── Empty project ──────────────────────────────────────────────────

#[test]
fn empty_project_creates_tempdir() {
    let project = Project::empty().build().unwrap();
    assert!(project.path().exists());
    assert!(project.path().is_dir());
}

#[test]
fn empty_project_with_file() {
    let project = Project::empty()
        .file("config.toml", "[settings]\nkey = \"value\"")
        .build()
        .unwrap();
    let content = fs::read_to_string(project.path().join("config.toml")).unwrap();
    assert_eq!(content, "[settings]\nkey = \"value\"");
}

#[test]
fn empty_project_with_nested_file() {
    let project = Project::empty().file("a/b/c.txt", "deep").build().unwrap();
    let content = fs::read_to_string(project.path().join("a/b/c.txt")).unwrap();
    assert_eq!(content, "deep");
}

#[test]
fn empty_project_with_dir() {
    let project = Project::empty().dir("empty-dir").build().unwrap();
    assert!(project.path().join("empty-dir").is_dir());
}

#[test]
fn setup_git_initializes_a_repository() {
    let project = Project::empty().setup_git().build().unwrap();

    assert!(project.path().join(".git").is_dir());

    let status = Command::new("git")
        .args([
            "-C",
            project.path().to_str().unwrap(),
            "rev-parse",
            "--is-inside-work-tree",
        ])
        .output()
        .unwrap();
    assert!(status.status.success());
    assert_eq!(String::from_utf8(status.stdout).unwrap().trim(), "true");
}

#[test]
fn initial_commit_creates_head_commit() {
    let project = Project::empty()
        .file("README.md", "hello\n")
        .initial_commit("initial commit")
        .build()
        .unwrap();

    let head = Command::new("git")
        .args([
            "-C",
            project.path().to_str().unwrap(),
            "rev-parse",
            "--verify",
            "HEAD",
        ])
        .output()
        .unwrap();
    assert!(head.status.success());
    assert_eq!(String::from_utf8(head.stdout).unwrap().trim().len(), 40);

    let message = Command::new("git")
        .args([
            "-C",
            project.path().to_str().unwrap(),
            "log",
            "-1",
            "--format=%s",
            "HEAD",
        ])
        .output()
        .unwrap();
    assert!(message.status.success());
    assert_eq!(
        String::from_utf8(message.stdout).unwrap().trim(),
        "initial commit"
    );

    let author = Command::new("git")
        .args([
            "-C",
            project.path().to_str().unwrap(),
            "log",
            "-1",
            "--format=%an <%ae>",
            "HEAD",
        ])
        .output()
        .unwrap();
    assert!(author.status.success());
    assert_eq!(
        String::from_utf8(author.stdout).unwrap().trim(),
        "Scenario Test <scenario@example.com>"
    );
}

#[test]
fn git_setup_actions_are_order_independent() {
    let project = Project::empty()
        .file("README.md", "hello\n")
        .initial_commit("out of order")
        .git_user("Ordered User", "ordered@example.com")
        .setup_git()
        .build()
        .unwrap();

    let head = Command::new("git")
        .args([
            "-C",
            project.path().to_str().unwrap(),
            "rev-parse",
            "--verify",
            "HEAD",
        ])
        .output()
        .unwrap();
    assert!(head.status.success());

    let author = Command::new("git")
        .args([
            "-C",
            project.path().to_str().unwrap(),
            "log",
            "-1",
            "--format=%an <%ae>",
            "HEAD",
        ])
        .output()
        .unwrap();
    assert!(author.status.success());
    assert_eq!(
        String::from_utf8(author.stdout).unwrap().trim(),
        "Ordered User <ordered@example.com>"
    );
}

#[test]
fn build_in_initial_commit_only_tracks_builder_created_content() {
    let target = tempfile::tempdir().unwrap();
    fs::write(target.path().join("preexisting.txt"), "leave me alone\n").unwrap();

    let project = Project::empty()
        .file("managed.txt", "tracked\n")
        .initial_commit("scoped commit")
        .build_in(target.path())
        .unwrap();

    let tracked = Command::new("git")
        .args([
            "-C",
            project.path().to_str().unwrap(),
            "ls-tree",
            "--name-only",
            "-r",
            "HEAD",
        ])
        .output()
        .unwrap();
    assert!(tracked.status.success());
    let tracked = String::from_utf8(tracked.stdout).unwrap();
    assert!(tracked.lines().any(|line| line == "managed.txt"));
    assert!(!tracked.lines().any(|line| line == "preexisting.txt"));

    let status = Command::new("git")
        .args(["-C", project.path().to_str().unwrap(), "status", "--short"])
        .output()
        .unwrap();
    assert!(status.status.success());
    let status = String::from_utf8(status.stdout).unwrap();
    assert!(status.lines().any(|line| line == "?? preexisting.txt"));
    assert!(!status.lines().any(|line| line.contains("managed.txt")));
}

#[test]
fn build_in_initial_commit_does_not_stage_preexisting_nested_files_under_builder_dirs() {
    let target = tempfile::tempdir().unwrap();
    fs::create_dir_all(target.path().join("managed")).unwrap();
    fs::write(
        target.path().join("managed/preexisting.txt"),
        "leave nested alone\n",
    )
    .unwrap();

    let project = Project::empty()
        .dir("managed")
        .file("managed/created.txt", "tracked\n")
        .initial_commit("nested scope")
        .build_in(target.path())
        .unwrap();

    let tracked = Command::new("git")
        .args([
            "-C",
            project.path().to_str().unwrap(),
            "ls-tree",
            "--name-only",
            "-r",
            "HEAD",
        ])
        .output()
        .unwrap();
    assert!(tracked.status.success());
    let tracked = String::from_utf8(tracked.stdout).unwrap();
    assert!(tracked.lines().any(|line| line == "managed/created.txt"));
    assert!(
        !tracked
            .lines()
            .any(|line| line == "managed/preexisting.txt")
    );

    let status = Command::new("git")
        .args(["-C", project.path().to_str().unwrap(), "status", "--short"])
        .output()
        .unwrap();
    assert!(status.status.success());
    let status = String::from_utf8(status.stdout).unwrap();
    assert!(
        status
            .lines()
            .any(|line| line == "?? managed/preexisting.txt")
    );
    assert!(
        !status
            .lines()
            .any(|line| line.contains("managed/created.txt"))
    );
}

#[test]
fn project_setup_failure_is_reported() {
    let result = Project::empty()
        .setup_git()
        .initial_commit("initial commit")
        .build();

    match result {
        Err(Error::ProjectSetup { step, source }) => {
            assert_eq!(step, "initial_commit");
            let message = source.to_string();
            assert!(message.starts_with("git ["));
            assert!(
                message.contains("git [\"add\", \"--\"]")
                    || message.contains("git [\"commit\", \"-m\", \"initial commit\"]")
            );
        }
        other => panic!("expected ProjectSetup error, got: {other:?}"),
    }
}

#[test]
fn empty_project_cleanup_on_drop() {
    let path;
    {
        let project = Project::empty()
            .file("tmp.txt", "gone soon")
            .build()
            .unwrap();
        path = project.path().to_path_buf();
        assert!(path.exists());
    }
    assert!(!path.exists());
}

// ── Template: basic rendering ──────────────────────────────────────

#[test]
fn template_basic_rendering() {
    let fixtures = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/basic");
    let project = Project::from_template(&fixtures)
        .var("greeting", "Hello")
        .build()
        .unwrap();
    let content = fs::read_to_string(project.path().join("greeting.txt")).unwrap();
    assert_eq!(content, "Hello, world!\n");
}

#[test]
fn template_override_default() {
    let fixtures = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/basic");
    let project = Project::from_template(&fixtures)
        .var("greeting", "Hi")
        .var("name", "Rust")
        .build()
        .unwrap();
    let content = fs::read_to_string(project.path().join("greeting.txt")).unwrap();
    assert_eq!(content, "Hi, Rust!\n");
}

#[test]
fn template_missing_required_var() {
    let fixtures = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/basic");
    let result = Project::from_template(&fixtures).build();
    match result {
        Err(Error::MissingVariable { names }) => {
            assert_eq!(names, vec!["greeting"]);
        }
        other => panic!("expected MissingVariable error, got: {other:?}"),
    }
}

#[test]
fn template_unknown_var() {
    let fixtures = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/basic");
    let result = Project::from_template(&fixtures)
        .var("greeting", "Hello")
        .var("typo_var", "oops")
        .build();
    match result {
        Err(Error::UnknownVariable { name }) => {
            assert_eq!(name, "typo_var");
        }
        other => panic!("expected UnknownVariable error, got: {other:?}"),
    }
}

#[test]
fn template_not_found() {
    let result = Project::from_template("/nonexistent/path").build();
    assert!(matches!(result, Err(Error::TemplateNotFound { .. })));
}

#[test]
fn template_excludes_manifest_from_output() {
    let fixtures = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/basic");
    let project = Project::from_template(&fixtures)
        .var("greeting", "Hello")
        .build()
        .unwrap();
    assert!(!project.path().join("template.toml").exists());
}

// ── File filtering ─────────────────────────────────────────────────

#[test]
fn optional_files_excluded_by_default() {
    let fixtures = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/with-optional");
    let project = Project::from_template(&fixtures).build().unwrap();

    assert!(project.path().join("config.txt").exists());
    assert!(!project.path().join("lockfile.txt").exists());
    assert!(!project.path().join("extra/data.txt").exists());
}

#[test]
fn include_brings_back_optional() {
    let fixtures = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/with-optional");
    let project = Project::from_template(&fixtures)
        .include("lockfile.txt")
        .build()
        .unwrap();

    assert!(project.path().join("config.txt").exists());
    assert!(project.path().join("lockfile.txt").exists());
    assert!(!project.path().join("extra/data.txt").exists());
}

#[test]
fn include_dir_prefix_brings_back_all() {
    let fixtures = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/with-optional");
    let project = Project::from_template(&fixtures)
        .include("extra")
        .build()
        .unwrap();

    assert!(project.path().join("extra/data.txt").exists());
}

#[test]
fn exclude_removes_non_optional() {
    let fixtures = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/with-optional");
    let project = Project::from_template(&fixtures)
        .exclude("config.txt")
        .build()
        .unwrap();

    assert!(!project.path().join("config.txt").exists());
    assert!(!project.path().join("lockfile.txt").exists());
}

// ── Path mappings ──────────────────────────────────────────────────

#[test]
fn mapping_routes_source_to_rendered_dest() {
    let fixtures = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/with-mappings");
    let project = Project::from_template(&fixtures)
        .var("name", "my-skill")
        .build()
        .unwrap();

    // Source file should NOT exist at its natural path
    assert!(!project.path().join("skill.md").exists());

    // It should exist at the mapped, rendered path
    let content =
        fs::read_to_string(project.path().join(".agents/skills/my-skill/SKILL.md")).unwrap();
    assert!(content.contains("name: my-skill"));
    assert!(content.contains("Body of my-skill"));

    // Unmapped files keep their natural path
    assert!(project.path().join("config.txt").exists());
}

// ── Overrides ──────────────────────────────────────────────────────

#[test]
fn override_replaces_template_content() {
    let fixtures = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/basic");
    let project = Project::from_template(&fixtures)
        .var("greeting", "Yo")
        .override_file("greeting.txt", "Custom: {{ name }} says {{ greeting }}\n")
        .build()
        .unwrap();

    let content = fs::read_to_string(project.path().join("greeting.txt")).unwrap();
    assert_eq!(content, "Custom: world says Yo\n");
}

#[test]
fn extra_file_added_verbatim() {
    let fixtures = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/basic");
    let project = Project::from_template(&fixtures)
        .var("greeting", "Hi")
        .file("extra.txt", "{{ not rendered }}")
        .build()
        .unwrap();

    // Extra files are written verbatim — no template rendering
    let content = fs::read_to_string(project.path().join("extra.txt")).unwrap();
    assert_eq!(content, "{{ not rendered }}");
}

// ── Symlinks ───────────────────────────────────────────────────────

#[test]
fn symlink_created_with_rendered_paths() {
    let fixtures = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/with-symlinks");
    let project = Project::from_template(&fixtures)
        .var("name", "my-skill")
        .build()
        .unwrap();

    let link_path = project.path().join(".targets/my-skill");
    assert!(link_path.symlink_metadata().unwrap().is_symlink());

    // The symlink should resolve to the real skill directory
    let resolved = fs::read_to_string(link_path.join("readme.md")).unwrap();
    assert!(resolved.contains("Skill: my-skill"));
}

#[test]
fn symlink_missing_target_errors() {
    let tmp = tempfile::tempdir().unwrap();
    let template_dir = tmp.path().join("template");
    fs::create_dir_all(&template_dir).unwrap();
    fs::write(
        template_dir.join("template.toml"),
        r#"
[variables]

[files.symlinks]
"link" = "nonexistent-target"
"#,
    )
    .unwrap();

    let result = Project::from_template(&template_dir).build();
    assert!(matches!(result, Err(Error::SymlinkTarget { .. })));
}

// ── build_in ───────────────────────────────────────────────────────

#[test]
fn build_in_populates_existing_dir() {
    let target = tempfile::tempdir().unwrap();
    let fixtures = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/basic");
    let project = Project::from_template(&fixtures)
        .var("greeting", "Hello")
        .build_in(target.path())
        .unwrap();

    assert_eq!(project.path(), target.path());
    let content = fs::read_to_string(target.path().join("greeting.txt")).unwrap();
    assert_eq!(content, "Hello, world!\n");
}

#[test]
fn build_in_does_not_cleanup_on_drop() {
    let target = tempfile::tempdir().unwrap();
    let target_path = target.path().to_path_buf();

    {
        let _project = Project::empty()
            .file("test.txt", "data")
            .build_in(&target_path)
            .unwrap();
    }

    // Directory still exists after Project is dropped
    assert!(target_path.join("test.txt").exists());
}

// ── Scenario integration ───────────────────────────────────────────

#[test]
fn scenario_project_sets_current_dir() {
    let project = Project::empty()
        .file("marker.txt", "found it")
        .build()
        .unwrap();

    let output = Scenario::new("cat")
        .arg("marker.txt")
        .project(&project)
        .run()
        .unwrap();

    assert!(output.success());
    assert!(output.stdout().contains("found it"));
}

// ── Error cases ────────────────────────────────────────────────────

#[test]
fn template_render_error_on_bad_syntax() {
    let tmp = tempfile::tempdir().unwrap();
    let template_dir = tmp.path().join("template");
    fs::create_dir_all(&template_dir).unwrap();
    fs::write(template_dir.join("template.toml"), "[variables]\n").unwrap();
    fs::write(template_dir.join("bad.txt"), "{{ unclosed").unwrap();

    let result = Project::from_template(&template_dir).build();
    assert!(matches!(result, Err(Error::TemplateRender { .. })));
}

#[test]
fn malformed_manifest_toml() {
    let tmp = tempfile::tempdir().unwrap();
    let template_dir = tmp.path().join("template");
    fs::create_dir_all(&template_dir).unwrap();
    fs::write(template_dir.join("template.toml"), "[invalid\nbroken").unwrap();

    let result = Project::from_template(&template_dir).build();
    assert!(matches!(result, Err(Error::ManifestParse { .. })));
}
