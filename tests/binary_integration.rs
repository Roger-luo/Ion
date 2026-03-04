use std::fs;
use tempfile::tempdir;

/// Test that binary skill entries roundtrip through Ion.toml correctly.
#[test]
fn test_binary_skill_manifest_roundtrip() {
    let toml_content = r#"
[skills]
mytool = { type = "binary", source = "owner/mytool", binary = "mytool" }
brainstorming = "anthropics/skills/brainstorming"
"#;
    let manifest = ion_skill::manifest::Manifest::parse(toml_content).unwrap();
    assert_eq!(manifest.skills.len(), 2);

    let entry = manifest.skills.get("mytool").unwrap();
    let source = ion_skill::manifest::Manifest::resolve_entry(entry).unwrap();

    assert_eq!(source.source_type, ion_skill::source::SourceType::Binary);
    assert_eq!(source.source, "owner/mytool");
    assert_eq!(source.binary.as_deref(), Some("mytool"));
}

/// Test that LockedSkill with binary fields roundtrips through TOML.
#[test]
fn test_binary_locked_skill_roundtrip() {
    let locked = ion_skill::lockfile::LockedSkill {
        name: "mytool".to_string(),
        source: "https://github.com/owner/mytool.git".to_string(),
        path: None,
        version: Some("1.2.0".to_string()),
        commit: None,
        checksum: None,
        binary: Some("mytool".to_string()),
        binary_version: Some("1.2.0".to_string()),
        binary_checksum: Some("sha256:abc123".to_string()),
    };

    let lockfile = ion_skill::lockfile::Lockfile {
        skills: vec![locked],
    };

    let tmp = tempdir().unwrap();
    let path = tmp.path().join("Ion.lock");
    lockfile.write_to(&path).unwrap();

    let loaded = ion_skill::lockfile::Lockfile::from_file(&path).unwrap();
    assert_eq!(loaded.skills.len(), 1);
    assert_eq!(loaded.skills[0].binary.as_deref(), Some("mytool"));
    assert_eq!(loaded.skills[0].binary_version.as_deref(), Some("1.2.0"));
    assert_eq!(
        loaded.skills[0].binary_checksum.as_deref(),
        Some("sha256:abc123")
    );
}

/// Test platform detection produces valid values.
#[test]
fn test_platform_detection_produces_valid_triple() {
    let platform = ion_skill::binary::Platform::detect();
    let triple = platform.target_triple();
    assert!(
        triple.contains("darwin") || triple.contains("linux") || triple.contains("windows"),
        "Unexpected triple: {}",
        triple
    );
}

/// Test that a manifest with mixed binary and regular skills parses correctly.
#[test]
fn test_mixed_manifest_binary_and_regular() {
    let toml_content = r#"
[skills]
mytool = { type = "binary", source = "owner/mytool", binary = "mytool", rev = "v1.0" }
brainstorming = "anthropics/skills/brainstorming"
local = { type = "path", source = "./my-local-skill" }
"#;
    let manifest = ion_skill::manifest::Manifest::parse(toml_content).unwrap();
    assert_eq!(manifest.skills.len(), 3);

    let binary_source =
        ion_skill::manifest::Manifest::resolve_entry(&manifest.skills["mytool"]).unwrap();
    assert_eq!(
        binary_source.source_type,
        ion_skill::source::SourceType::Binary
    );
    assert_eq!(binary_source.rev.as_deref(), Some("v1.0"));
    assert_eq!(binary_source.binary.as_deref(), Some("mytool"));

    let regular_source =
        ion_skill::manifest::Manifest::resolve_entry(&manifest.skills["brainstorming"]).unwrap();
    assert_eq!(
        regular_source.source_type,
        ion_skill::source::SourceType::Github
    );
    assert!(regular_source.binary.is_none());
}

/// Test manifest_writer handles binary skills in Ion.toml.
#[test]
fn test_manifest_writer_binary_skill() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("Ion.toml");
    fs::write(&path, "[skills]\n").unwrap();

    let source = ion_skill::source::SkillSource {
        source_type: ion_skill::source::SourceType::Binary,
        source: "owner/mytool".to_string(),
        path: None,
        rev: None,
        version: None,
        binary: Some("mytool".to_string()),
    };

    ion_skill::manifest_writer::add_skill(&path, "mytool", &source).unwrap();

    let content = fs::read_to_string(&path).unwrap();
    assert!(
        content.contains("binary"),
        "Should contain binary field: {}",
        content
    );
    assert!(
        content.contains("mytool"),
        "Should contain binary name: {}",
        content
    );
}
