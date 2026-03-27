use std::process::Command;

fn ion_cmd() -> Command {
    Command::new(env!("CARGO_BIN_EXE_ion"))
}

/// Helper: write a minimal AGENTS.md containing the managed section markers.
fn write_managed_agents_md(dir: &std::path::Path, inner: &str) {
    let content = format!(
        "<!-- ion:managed:begin -->\n{inner}\n<!-- ion:managed:end -->\n\n\
         ## Project-Specific Notes\n\nMy project stuff.\n"
    );
    std::fs::write(dir.join("AGENTS.md"), content).unwrap();
}

// ── ion agents fetch ──────────────────────────────────────────────────────────

#[test]
fn agents_fetch_requires_url_or_config() {
    let dir = tempfile::tempdir().unwrap();

    let output = ion_cmd()
        .args(["agents", "fetch"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !output.status.success(),
        "should fail when no URL is given and none configured"
    );
    assert!(
        stderr.contains("URL") || stderr.contains("url"),
        "error should mention URL: {stderr}"
    );
}

#[test]
fn agents_fetch_with_invalid_url_fails_gracefully() {
    let dir = tempfile::tempdir().unwrap();

    let output = ion_cmd()
        .args([
            "agents",
            "fetch",
            "https://this-domain-does-not-exist.invalid/AGENTS.md",
        ])
        .current_dir(dir.path())
        .output()
        .unwrap();

    // Should fail (HTTP error or connection error) but not panic
    assert!(!output.status.success(), "should fail with invalid URL");
}

#[test]
fn agents_fetch_reads_url_from_manifest() {
    let dir = tempfile::tempdir().unwrap();

    // Write Ion.toml with an agents-md-url that will fail to connect
    // (we just need to confirm that the URL is *read* from the config)
    std::fs::write(
        dir.path().join("Ion.toml"),
        "[skills]\n\n[options]\nagents-md-url = \"https://does-not-exist.invalid/AGENTS.md\"\n",
    )
    .unwrap();

    let output = ion_cmd()
        .args(["agents", "fetch"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    let stderr = String::from_utf8_lossy(&output.stderr);
    // It should attempt the fetch (HTTP error), NOT the "no URL" config error
    assert!(
        !stderr.contains("No AGENTS.md URL specified"),
        "should have loaded URL from Ion.toml and attempted the fetch, not shown 'no URL' error, stderr: {stderr}"
    );
}

/// Verifies that the `--force` flag is accepted by the CLI parser.
/// A network error is expected (invalid URL); we only check that there is no
/// "unexpected argument" error from clap.
#[test]
fn agents_fetch_force_flag_is_accepted() {
    let dir = tempfile::tempdir().unwrap();

    std::fs::write(dir.path().join("AGENTS.md"), "# My notes\n").unwrap();

    let output = ion_cmd()
        .args([
            "agents",
            "fetch",
            "--force",
            "https://does-not-exist.invalid/AGENTS.md",
        ])
        .current_dir(dir.path())
        .output()
        .unwrap();

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("unexpected argument"),
        "force flag should be accepted by the CLI parser: {stderr}"
    );
}

// ── ion agents update ─────────────────────────────────────────────────────────

#[test]
fn agents_update_requires_configured_url() {
    let dir = tempfile::tempdir().unwrap();

    let output = ion_cmd()
        .args(["agents", "update"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !output.status.success(),
        "should fail when no URL is configured"
    );
    assert!(
        stderr.contains("No AGENTS.md URL configured"),
        "error should mention configuration: {stderr}"
    );
}

#[test]
fn agents_update_reads_url_from_manifest() {
    let dir = tempfile::tempdir().unwrap();

    std::fs::write(
        dir.path().join("Ion.toml"),
        "[skills]\n\n[options]\nagents-md-url = \"https://does-not-exist.invalid/AGENTS.md\"\n",
    )
    .unwrap();

    write_managed_agents_md(dir.path(), "old org content");

    let output = ion_cmd()
        .args(["agents", "update"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    let stderr = String::from_utf8_lossy(&output.stderr);
    // It should attempt the fetch (network error), NOT the "no URL" config error
    assert!(
        !stderr.contains("No AGENTS.md URL configured"),
        "should have loaded URL from Ion.toml and attempted the fetch, not shown 'no URL configured' error, stderr: {stderr}"
    );
}

// ── library-level tests ───────────────────────────────────────────────────────

#[test]
fn agents_md_write_new_creates_managed_markers() {
    use ion_skill::agents_md;

    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("AGENTS.md");
    agents_md::write_new("# Org Standard", &path, false).unwrap();

    let content = std::fs::read_to_string(&path).unwrap();
    assert!(content.contains(agents_md::MANAGED_BEGIN));
    assert!(content.contains(agents_md::MANAGED_END));
    assert!(content.contains("# Org Standard"));
    assert!(content.contains("Project-Specific Notes"));
}

#[test]
fn agents_md_update_managed_preserves_project_content() {
    use ion_skill::agents_md;

    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("AGENTS.md");

    write_managed_agents_md(dir.path(), "old org content");

    let result = agents_md::update_managed("new org content", &path).unwrap();
    assert_eq!(result, agents_md::WriteResult::Updated);

    let content = std::fs::read_to_string(&path).unwrap();
    assert!(
        content.contains("new org content"),
        "managed content updated"
    );
    assert!(
        !content.contains("old org content"),
        "old managed content removed"
    );
    assert!(
        content.contains("My project stuff."),
        "project content preserved"
    );
}

#[test]
fn agents_md_resolve_url_github_shorthand() {
    use ion_skill::agents_md;

    let url = agents_md::resolve_url("myorg/myrepo/AGENTS.md");
    assert_eq!(
        url,
        "https://raw.githubusercontent.com/myorg/myrepo/HEAD/AGENTS.md"
    );
}

#[test]
fn manifest_options_agents_md_url_roundtrip() {
    use ion_skill::manifest::Manifest;

    let toml = "[skills]\n\n[options]\nagents-md-url = \"https://example.com/AGENTS.md\"\n";
    let manifest = Manifest::parse(toml).unwrap();
    assert_eq!(
        manifest.options.agents_md_url.as_deref(),
        Some("https://example.com/AGENTS.md")
    );
    assert_eq!(
        manifest.options.get_value("agents-md-url").as_deref(),
        Some("https://example.com/AGENTS.md")
    );
    let values = manifest.options.list_values();
    assert!(
        values
            .iter()
            .any(|(k, v)| k == "agents-md-url" && v == "https://example.com/AGENTS.md")
    );
}

#[test]
fn manifest_writer_write_agents_md_url() {
    use ion_skill::manifest::Manifest;
    use ion_skill::manifest_writer;

    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("Ion.toml");
    std::fs::write(&path, "[skills]\n").unwrap();

    manifest_writer::write_agents_md_url(&path, "https://example.com/AGENTS.md").unwrap();

    let content = std::fs::read_to_string(&path).unwrap();
    assert!(content.contains("agents-md-url"));
    assert!(content.contains("https://example.com/AGENTS.md"));

    // Verify it parses back correctly
    let manifest = Manifest::parse(&content).unwrap();
    assert_eq!(
        manifest.options.agents_md_url.as_deref(),
        Some("https://example.com/AGENTS.md")
    );
}

#[test]
fn global_config_agents_md_url() {
    use ion_skill::config::GlobalConfig;

    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.toml");
    std::fs::write(
        &path,
        "[agents]\nmd-url = \"https://example.com/AGENTS.md\"\n",
    )
    .unwrap();

    let config = GlobalConfig::load_from(&path).unwrap();
    assert_eq!(
        config.agents.md_url.as_deref(),
        Some("https://example.com/AGENTS.md")
    );
    assert_eq!(
        config.get_value("agents.md-url").as_deref(),
        Some("https://example.com/AGENTS.md")
    );
}
