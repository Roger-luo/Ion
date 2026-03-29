use std::fs;
use std::path::Path;

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
