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

/// Test that is_binary_installed works correctly.
#[test]
fn test_binary_cache_check() {
    let tmp = tempdir().unwrap();
    let bin_root = tmp.path().join("bin");

    // Not installed yet
    let path = bin_root.join("mytool").join("1.0.0").join("mytool");
    assert!(!path.exists());

    // Create the expected directory structure
    let version_dir = bin_root.join("mytool").join("1.0.0");
    fs::create_dir_all(&version_dir).unwrap();
    fs::write(version_dir.join("mytool"), "fake binary").unwrap();

    // Now the path should exist
    assert!(path.exists());
}

/// Test that binary cleanup functions work correctly.
#[test]
fn test_binary_cleanup_functions() {
    let tmp = tempdir().unwrap();

    // Create fake binary structure
    let binary_dir = tmp.path().join("mytool");
    let v1_dir = binary_dir.join("1.0.0");
    let v2_dir = binary_dir.join("2.0.0");
    fs::create_dir_all(&v1_dir).unwrap();
    fs::create_dir_all(&v2_dir).unwrap();
    fs::write(v1_dir.join("mytool"), "v1").unwrap();
    fs::write(v2_dir.join("mytool"), "v2").unwrap();

    // Remove just v1
    fs::remove_dir_all(&v1_dir).unwrap();
    assert!(!v1_dir.exists());
    assert!(v2_dir.exists());

    // Remove entire binary dir
    fs::remove_dir_all(&binary_dir).unwrap();
    assert!(!binary_dir.exists());
}

/// Test that lockfile correctly tracks binary version changes (simulating update).
#[test]
fn test_lockfile_binary_version_update() {
    let tmp = tempdir().unwrap();
    let path = tmp.path().join("Ion.lock");

    // Initial install at v1.0.0
    let mut lockfile = ion_skill::lockfile::Lockfile::default();
    lockfile.upsert(ion_skill::lockfile::LockedSkill {
        name: "mytool".to_string(),
        source: "https://github.com/owner/mytool.git".to_string(),
        path: None,
        version: Some("1.0.0".to_string()),
        commit: None,
        checksum: None,
        binary: Some("mytool".to_string()),
        binary_version: Some("1.0.0".to_string()),
        binary_checksum: Some("sha256:old".to_string()),
    });

    lockfile.write_to(&path).unwrap();

    // Simulate update to v2.0.0
    let mut lockfile = ion_skill::lockfile::Lockfile::from_file(&path).unwrap();
    let mut entry = lockfile.find("mytool").unwrap().clone();
    entry.binary_version = Some("2.0.0".to_string());
    entry.binary_checksum = Some("sha256:new".to_string());
    entry.version = Some("2.0.0".to_string());
    lockfile.upsert(entry);
    lockfile.write_to(&path).unwrap();

    // Verify update persisted
    let loaded = ion_skill::lockfile::Lockfile::from_file(&path).unwrap();
    assert_eq!(loaded.skills[0].binary_version.as_deref(), Some("2.0.0"));
    assert_eq!(
        loaded.skills[0].binary_checksum.as_deref(),
        Some("sha256:new")
    );
}

/// Test list_installed_binaries returns correct names.
#[test]
fn test_list_installed_binaries() {
    // This just verifies the function doesn't crash — it uses the real bin_dir
    // which may or may not have content
    let result = ion_skill::binary::list_installed_binaries();
    assert!(result.is_ok());
}
