use std::fs;
use std::process::Command;

fn fixture() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    fs::create_dir(dir.path().join("project")).unwrap();
    fs::write(
        dir.path().join("project/Ion.toml"),
        "[options.targets]\nclaude = \".claude/skills\"\n[skills]\n",
    )
    .unwrap();
    for (path, name) in [
        ("skills/pdf", "pdf"),
        ("skills/pptx", "pptx"),
        ("skills/unselected", "unselected"),
        ("template", "template"),
    ] {
        let skill = dir.path().join("collection").join(path);
        fs::create_dir_all(&skill).unwrap();
        fs::write(
            skill.join("SKILL.md"),
            format!("---\nname: {name}\ndescription: Process documents when requested by the user.\n---\n# Instructions\n\nProcess the requested documents.\n"),
        )
        .unwrap();
    }
    dir
}

fn add(dir: &tempfile::TempDir, selection: &str) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_ion"))
        .args([
            "add",
            "../collection",
            "--skills",
            selection,
            "--allow-warnings",
            "--json",
        ])
        .current_dir(dir.path().join("project"))
        .env("HOME", dir.path())
        .env("XDG_CONFIG_HOME", dir.path().join("config"))
        .env("XDG_DATA_HOME", dir.path().join("data"))
        .output()
        .unwrap()
}

#[test]
fn collection_selection_matches_deployment_and_registration() {
    let dir = fixture();
    let output = add(&dir, "pdf,pptx");
    assert!(
        output.status.success(),
        "{} {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let mut installed: Vec<_> = json["data"]["installed"]
        .as_array()
        .unwrap()
        .iter()
        .map(|n| n.as_str().unwrap())
        .collect();
    installed.sort();
    assert_eq!(installed, ["pdf", "pptx"]);
    let project = dir.path().join("project");
    for parent in [".agents/skills", ".claude/skills"] {
        let mut names: Vec<_> = fs::read_dir(project.join(parent))
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        names.retain(|name| name != "ion-cli");
        names.sort();
        assert_eq!(names, ["pdf", "pptx"], "unexpected deployment in {parent}");
    }
    let manifest = ion_skill::manifest::Manifest::from_file(&project.join("Ion.toml")).unwrap();
    let mut names: Vec<_> = manifest.skills.keys().map(String::as_str).collect();
    names.retain(|name| *name != "ion-cli");
    names.sort();
    assert_eq!(names, ["pdf", "pptx"]);
    let lock = ion_skill::lockfile::Lockfile::from_file(&project.join("Ion.lock")).unwrap();
    let mut names: Vec<_> = lock.skills.iter().map(|s| s.name.as_str()).collect();
    names.retain(|name| *name != "ion-cli");
    names.sort();
    assert_eq!(names, ["pdf", "pptx"]);
}

#[test]
fn collection_selection_rejects_unknown_and_empty_names_without_installing() {
    for selection in ["missing", "pdf,missing", "", "pdf,"] {
        let dir = fixture();
        let output = add(&dir, selection);
        assert!(
            !output.status.success(),
            "selection {selection:?} unexpectedly succeeded: {}",
            String::from_utf8_lossy(&output.stdout)
        );
        assert!(!dir.path().join("project/.agents/skills/pdf").exists());
    }
}

#[test]
fn collection_selection_does_not_parse_unselected_metadata() {
    let dir = fixture();
    fs::write(
        dir.path().join("collection/skills/unselected/SKILL.md"),
        "---\nname: unselected\ndescription: Unselected skill.\nmetadata:\n  openclaw:\n    nested: invalid\n---\n# Instructions\n",
    ).unwrap();
    let output = add(&dir, "pdf,pptx");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stdout)
    );
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(
        json["data"]["installed"],
        serde_json::json!(["pdf", "pptx"])
    );
    assert!(
        !dir.path()
            .join("project/.agents/skills/unselected")
            .exists()
    );
}

#[test]
fn collection_selection_still_rejects_malformed_selected_metadata() {
    let dir = fixture();
    fs::write(
        dir.path().join("collection/skills/pdf/SKILL.md"),
        "---\nname: pdf\ndescription: Selected skill.\nmetadata:\n  openclaw:\n    nested: invalid\n---\n# Instructions\n",
    ).unwrap();
    let output = add(&dir, "pdf");
    assert!(!output.status.success());
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert!(json["error"].as_str().unwrap().contains("YAML parse error"));
    assert!(!dir.path().join("project/.agents/skills/pdf").exists());
}

#[test]
fn collection_selection_uses_discovered_directory_names() {
    let dir = fixture();
    let skill = dir.path().join("collection/skills/pdf/SKILL.md");
    let contents = fs::read_to_string(&skill)
        .unwrap()
        .replace("name: pdf", "name: document-reader");
    fs::write(&skill, contents).unwrap();
    let output = add(&dir, "document-reader");
    assert!(!output.status.success());
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert!(
        json["error"]
            .as_str()
            .unwrap()
            .contains("not found in collection")
    );
    let output = add(&dir, "pdf");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stdout)
    );
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json["data"]["installed"], serde_json::json!(["pdf"]));
    assert!(
        dir.path()
            .join("project/.agents/skills/pdf/SKILL.md")
            .exists()
    );
    assert!(
        !dir.path()
            .join("project/.agents/skills/document-reader")
            .exists()
    );
}
