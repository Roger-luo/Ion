use std::fs;
use std::path::Path;

use scenario::Project;

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
