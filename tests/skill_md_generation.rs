use std::path::Path;
use std::process::Command;

fn ion() -> Command {
    Command::new(env!("CARGO_BIN_EXE_ion"))
}

/// Run an ion command and return stdout. Accepts exit 0 and 2 (action_required).
fn capture_json(args: &[&str], dir: &Path) -> String {
    let output = ion()
        .args(args)
        .current_dir(dir)
        .output()
        .expect("failed to execute ion");

    let code = output.status.code().unwrap_or(-1);
    assert!(
        code == 0 || code == 2,
        "ion {:?} failed with exit {code}\nstderr: {}",
        args,
        String::from_utf8_lossy(&output.stderr)
    );

    String::from_utf8(output.stdout)
        .expect("non-utf8 stdout")
        .trim()
        .to_string()
}

/// Replace dynamic substrings in JSON output for determinism.
fn stabilize(json: &str, replacements: &[(&str, &str)]) -> String {
    let mut result = json.to_string();
    for (from, to) in replacements {
        result = result.replace(from, to);
    }
    result
}

fn render_skill_md() -> String {
    let template_src = std::fs::read_to_string(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("templates/ion-cli.md.j2"),
    )
    .expect("failed to read template");

    // -- project init (no targets) --
    let init_dir = tempfile::tempdir().unwrap();
    // Create .claude/ so "detected: true" appears for claude (realistic example)
    std::fs::create_dir(init_dir.path().join(".claude")).unwrap();
    let example_init_no_targets =
        capture_json(&["--json", "project", "init"], init_dir.path());

    // -- project init (with targets) --
    let init_dir2 = tempfile::tempdir().unwrap();
    let example_init_with_targets = capture_json(
        &[
            "--json",
            "project",
            "init",
            "--target",
            "claude",
            "--target",
            "cursor",
        ],
        init_dir2.path(),
    );

    // -- remove (confirm, exit 2) --
    let remove_dir = tempfile::tempdir().unwrap();
    std::fs::write(
        remove_dir.path().join("Ion.toml"),
        "[skills]\ntest-skill = \"owner/repo\"\n",
    )
    .unwrap();
    std::fs::write(remove_dir.path().join("Ion.lock"), "").unwrap();
    let example_remove_confirm =
        capture_json(&["--json", "remove", "test-skill"], remove_dir.path());

    // -- remove (with --yes) --
    let remove_dir2 = tempfile::tempdir().unwrap();
    std::fs::write(
        remove_dir2.path().join("Ion.toml"),
        "[skills]\ntest-skill = \"owner/repo\"\n",
    )
    .unwrap();
    std::fs::write(remove_dir2.path().join("Ion.lock"), "").unwrap();
    std::fs::create_dir_all(remove_dir2.path().join(".agents/skills/test-skill")).unwrap();
    let example_remove_yes = capture_json(
        &["--json", "remove", "test-skill", "--yes"],
        remove_dir2.path(),
    );

    // -- skill list (empty project) --
    let list_dir = tempfile::tempdir().unwrap();
    std::fs::write(list_dir.path().join("Ion.toml"), "[skills]\n").unwrap();
    let example_skill_list =
        capture_json(&["--json", "skill", "list"], list_dir.path());

    // -- validate --
    let validate_dir = tempfile::tempdir().unwrap();
    let skill_dir = validate_dir.path().join("test-skill");
    std::fs::create_dir_all(&skill_dir).unwrap();
    std::fs::write(
        skill_dir.join("SKILL.md"),
        "---\nname: test-skill\ndescription: A test skill\n---\n\n# Test Skill\n",
    )
    .unwrap();
    let skill_dir_str = skill_dir.display().to_string();
    let raw_validate = capture_json(
        &["--json", "skill", "validate", &skill_dir_str],
        validate_dir.path(),
    );
    // Replace absolute path with relative for determinism
    let example_validate = stabilize(
        &raw_validate,
        &[(
            &format!("{}/SKILL.md", skill_dir_str),
            "test-skill/SKILL.md",
        )],
    );

    // -- config list (project-scoped to avoid depending on user's global config) --
    let config_dir = tempfile::tempdir().unwrap();
    std::fs::write(
        config_dir.path().join("Ion.toml"),
        "[options.targets]\nclaude = \".claude/skills\"\ncursor = \".cursor/skills\"\n",
    )
    .unwrap();
    let example_config_list = capture_json(
        &["--json", "config", "list", "--project"],
        config_dir.path(),
    );

    // -- config get --
    let example_config_get = capture_json(
        &["--json", "config", "get", "targets.claude", "--project"],
        config_dir.path(),
    );

    // -- config set --
    let config_set_dir = tempfile::tempdir().unwrap();
    std::fs::write(
        config_set_dir.path().join("Ion.toml"),
        "[options.targets]\nclaude = \".claude/skills\"\n",
    )
    .unwrap();
    let example_config_set = capture_json(
        &[
            "--json",
            "config",
            "set",
            "targets.claude",
            ".claude/commands",
            "--project",
        ],
        config_set_dir.path(),
    );

    // -- cache gc: verify structure only (output depends on global registry state, template uses static example) --
    let gc_output = capture_json(&["--json", "cache", "gc", "--dry-run"], init_dir.path());
    let gc: serde_json::Value = serde_json::from_str(&gc_output).unwrap();
    assert_eq!(gc["success"], true);
    assert_eq!(gc["data"]["dry_run"], true);
    assert!(gc["data"]["removed"].is_array());

    // -- self info: verify structure only (template uses static example) --
    let self_info_output = capture_json(&["--json", "self", "info"], init_dir.path());
    let self_info: serde_json::Value =
        serde_json::from_str(&self_info_output).unwrap();
    assert_eq!(self_info["success"], true);
    assert!(self_info["data"]["version"].is_string());
    assert!(self_info["data"]["target"].is_string());
    assert!(self_info["data"]["exe"].is_string());

    // Render template
    let mut env = minijinja::Environment::new();
    env.add_template("skill.md", &template_src).unwrap();
    let tmpl = env.get_template("skill.md").unwrap();
    tmpl.render(minijinja::context! {
        example_init_no_targets,
        example_init_with_targets,
        example_remove_confirm,
        example_remove_yes,
        example_skill_list,
        example_validate,
        example_config_list,
        example_config_get,
        example_config_set,
    })
    .unwrap()
}

#[test]
fn skill_md_matches_template() {
    let rendered = render_skill_md();
    let committed = std::fs::read_to_string(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("SKILL.md"),
    )
    .expect("failed to read SKILL.md");

    if rendered.trim() != committed.trim() {
        let rendered_lines: Vec<&str> = rendered.trim().lines().collect();
        let committed_lines: Vec<&str> = committed.trim().lines().collect();

        for (i, (r, c)) in rendered_lines
            .iter()
            .zip(committed_lines.iter())
            .enumerate()
        {
            if r != c {
                eprintln!("First difference at line {}:", i + 1);
                eprintln!("  rendered:  {:?}", r);
                eprintln!("  committed: {:?}", c);
                break;
            }
        }

        if rendered_lines.len() != committed_lines.len() {
            eprintln!(
                "Line count differs: rendered={}, committed={}",
                rendered_lines.len(),
                committed_lines.len()
            );
        }

        panic!(
            "SKILL.md is out of date with template. Run:\n  \
             cargo test regenerate_skill_md -- --ignored\n\
             to regenerate it."
        );
    }
}

#[test]
#[ignore]
fn regenerate_skill_md() {
    let rendered = render_skill_md();
    let path =
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("SKILL.md");
    std::fs::write(&path, rendered).expect("failed to write SKILL.md");
    println!("Regenerated SKILL.md");
}
