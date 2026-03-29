use std::fs;
use std::path::Path;

use scenario::{Error, Project};

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
