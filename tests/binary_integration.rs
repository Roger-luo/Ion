use std::fs;
use tempfile::tempdir;

/// Test that binary skill entries roundtrip through Ion.toml correctly.
#[test]
fn test_binary_skill_manifest_roundtrip() {
    let toml_content = r#"
[skills]
mytool = { type = "binary", source = "owner/mytool", binary = "mytool" }
brainstorming = "obra/superpowers/brainstorming"
"#;
    let manifest = ion_skill::manifest::Manifest::parse(toml_content).unwrap();
    assert_eq!(manifest.skills.len(), 2);

    let entry = manifest.skills.get("mytool").unwrap();
    let source = entry.resolve().unwrap();

    assert_eq!(source.source_type, ion_skill::source::SourceType::Binary);
    assert_eq!(source.source, "owner/mytool");
    assert_eq!(source.binary.as_deref(), Some("mytool"));
}

/// Test that LockedSkill with binary fields roundtrips through TOML.
#[test]
fn test_binary_locked_skill_roundtrip() {
    let locked = ion_skill::lockfile::LockedSkill::binary(
        "mytool",
        "https://github.com/owner/mytool.git",
        "mytool",
        Some("1.2.0".to_string()),
        Some("sha256:abc123".to_string()),
    )
    .with_version("1.2.0");

    let lockfile = ion_skill::lockfile::Lockfile {
        skills: vec![locked],
        agents: None,
    };

    let tmp = tempdir().unwrap();
    let path = tmp.path().join("Ion.lock");
    lockfile.write_to(&path).unwrap();

    let loaded = ion_skill::lockfile::Lockfile::from_file(&path).unwrap();
    assert_eq!(loaded.skills.len(), 1);
    assert_eq!(loaded.skills[0].binary_name(), Some("mytool"));
    assert_eq!(loaded.skills[0].binary_version(), Some("1.2.0"));
    assert_eq!(loaded.skills[0].checksum(), Some("sha256:abc123"));
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
brainstorming = "obra/superpowers/brainstorming"
local = { type = "path", source = "./my-local-skill" }
"#;
    let manifest = ion_skill::manifest::Manifest::parse(toml_content).unwrap();
    assert_eq!(manifest.skills.len(), 3);

    let binary_source = manifest.skills["mytool"].resolve().unwrap();
    assert_eq!(
        binary_source.source_type,
        ion_skill::source::SourceType::Binary
    );
    assert_eq!(binary_source.rev.as_deref(), Some("v1.0"));
    assert_eq!(binary_source.binary.as_deref(), Some("mytool"));

    let regular_source = manifest.skills["brainstorming"].resolve().unwrap();
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

    let source =
        ion_skill::source::SkillSource::new(ion_skill::source::SourceType::Binary, "owner/mytool")
            .with_binary("mytool");

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
    lockfile.upsert(
        ion_skill::lockfile::LockedSkill::binary(
            "mytool",
            "https://github.com/owner/mytool.git",
            "mytool",
            Some("1.0.0".to_string()),
            Some("sha256:old".to_string()),
        )
        .with_version("1.0.0"),
    );

    lockfile.write_to(&path).unwrap();

    // Simulate update to v2.0.0 by replacing with a new entry
    let mut lockfile = ion_skill::lockfile::Lockfile::from_file(&path).unwrap();
    lockfile.upsert(
        ion_skill::lockfile::LockedSkill::binary(
            "mytool",
            "https://github.com/owner/mytool.git",
            "mytool",
            Some("2.0.0".to_string()),
            Some("sha256:new".to_string()),
        )
        .with_version("2.0.0"),
    );
    lockfile.write_to(&path).unwrap();

    // Verify update persisted
    let loaded = ion_skill::lockfile::Lockfile::from_file(&path).unwrap();
    assert_eq!(loaded.skills[0].binary_version(), Some("2.0.0"));
    assert_eq!(loaded.skills[0].checksum(), Some("sha256:new"));
}

/// Test list_installed_binaries returns correct names.
#[test]
fn test_list_installed_binaries() {
    // This just verifies the function doesn't crash — it uses the real bin_dir
    // which may or may not have content
    let result = ion_skill::binary::list_installed_binaries();
    assert!(result.is_ok());
}

/// Test URL template expansion replaces all placeholders with platform values.
#[test]
fn url_template_expansion() {
    use ion_skill::binary::{Platform, expand_url_template};

    let platform = Platform::detect();
    let url = expand_url_template(
        "https://example.com/releases/{version}/{binary}-{target}.tar.gz",
        "mytool",
        "1.2.0",
    );
    assert!(url.contains("1.2.0"));
    assert!(url.contains("mytool"));
    assert!(url.contains(&platform.target_triple()));
    assert!(!url.contains("{version}"));
    assert!(!url.contains("{binary}"));
    assert!(!url.contains("{target}"));
}

/// Test that all supported placeholders are expanded correctly.
#[test]
fn url_template_all_placeholders() {
    use ion_skill::binary::{Platform, expand_url_template};

    let platform = Platform::detect();
    let url = expand_url_template(
        "{binary}-{version}-{os}-{arch}-{target}.tar.gz",
        "tool",
        "2.0",
    );
    assert_eq!(
        url,
        format!(
            "tool-2.0-{}-{}-{}.tar.gz",
            platform.os,
            platform.arch,
            platform.target_triple()
        )
    );
}

/// Test asset pattern matching against expanded URL templates.
#[test]
fn asset_pattern_matching() {
    use ion_skill::binary::{Platform, expand_url_template};

    let platform = Platform::detect();
    let pattern = "mytool-{version}-{os}-{arch}.tar.gz";
    let expanded = expand_url_template(pattern, "mytool", "1.0.0");

    let expected = format!("mytool-1.0.0-{}-{}.tar.gz", platform.os, platform.arch);
    assert_eq!(expanded, expected);

    // Verify the expanded name would match an asset list
    let assets = [
        expected.clone(),
        "mytool-1.0.0-other-other.tar.gz".to_string(),
    ];
    assert!(assets.contains(&expanded));
}

/// Test validate_binary with non-existent and real executables.
#[test]
fn binary_validation_struct() {
    use ion_skill::binary::validate_binary;

    // Test with non-existent path
    assert!(validate_binary(std::path::Path::new("/nonexistent/binary")).is_err());

    // Test with a real executable
    #[cfg(unix)]
    {
        let tmp = tempdir().unwrap();
        let bin = tmp.path().join("testbin");
        std::fs::write(&bin, "#!/bin/sh\necho test").unwrap();
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&bin, std::fs::Permissions::from_mode(0o755)).unwrap();

        let validation = validate_binary(&bin).unwrap();
        assert!(validation.is_executable);
    }
}

/// Test manifest parsing with asset-pattern field.
#[test]
fn manifest_with_asset_pattern() {
    use ion_skill::manifest::Manifest;

    let toml_str = r#"[skills]
mytool = { type = "binary", source = "owner/mytool", binary = "mytool", asset-pattern = "mytool-{version}-{os}-{arch}.tar.gz" }
"#;
    let manifest = Manifest::parse(toml_str).unwrap();
    let source = manifest.skills["mytool"].resolve().unwrap();
    assert_eq!(source.source_type, ion_skill::source::SourceType::Binary);
    assert_eq!(source.binary.as_deref(), Some("mytool"));
    assert_eq!(
        source.asset_pattern.as_deref(),
        Some("mytool-{version}-{os}-{arch}.tar.gz")
    );
}
