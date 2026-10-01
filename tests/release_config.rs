use toml_edit::DocumentMut;

#[test]
fn unpublished_packages_use_existing_git_releases_as_baseline() {
    let config = include_str!("../release-plz.toml")
        .parse::<DocumentMut>()
        .unwrap();
    let workspace = &config["workspace"];
    let overrides = config["package"].as_array_of_tables().unwrap();
    for name in ["ion", "ion-skill", "scenario", "ionem"] {
        let package = overrides
            .iter()
            .find(|p| p.get("name").and_then(|v| v.as_str()) == Some(name));
        let setting = |key: &str| {
            package
                .and_then(|p| p.get(key))
                .and_then(|v| v.as_bool())
                .or_else(|| workspace.get(key).and_then(|v| v.as_bool()))
                .unwrap_or(false)
        };
        let publish = setting("publish");
        let git_only = setting("git_only");
        assert_ne!(
            publish, git_only,
            "{name} must use exactly one release baseline: registry for published crates, git tags otherwise"
        );
        assert_eq!(
            publish,
            name == "ionem",
            "only ionem is published to crates.io"
        );
    }
}

#[test]
fn release_notes_identify_the_version_before_grouping_changes() {
    let config = include_str!("../release-plz.toml")
        .parse::<DocumentMut>()
        .unwrap();
    let body = config["changelog"]["body"].as_str().unwrap();
    let heading = body
        .lines()
        .find(|line| line.starts_with("## "))
        .expect("release notes need a level-two version heading");
    assert!(
        heading.contains("{{ version }}"),
        "release notes must identify the package version"
    );
    assert!(body.find(heading).unwrap() < body.find("{% for group").unwrap());
}

#[test]
fn release_tags_keep_the_workspace_package_prefix() {
    let config = include_str!("../release-plz.toml")
        .parse::<DocumentMut>()
        .unwrap();
    let template = config["workspace"]
        .get("git_tag_name")
        .and_then(|v| v.as_str())
        .expect("explicit tag names must survive selecting a single package");
    for (name, version) in [
        ("ion", "0.6.6"),
        ("ion-skill", "0.1.0"),
        ("ionem", "0.2.1"),
        ("scenario", "0.2.1"),
    ] {
        let tag = template
            .replace("{{ package }}", name)
            .replace("{{ version }}", version);
        assert_eq!(tag, format!("{name}-v{version}"));
    }
}
