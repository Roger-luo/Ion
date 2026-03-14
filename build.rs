use std::path::PathBuf;

fn main() {
    println!(
        "cargo:rustc-env=TARGET={}",
        std::env::var("TARGET").unwrap()
    );

    generate_skill_md();
}

/// Pretty-print a serde_json::Value.
fn pretty(v: serde_json::Value) -> String {
    serde_json::to_string_pretty(&v).unwrap()
}

/// Render templates/ion-cli.md.j2 with all example JSON and write to OUT_DIR/SKILL.md.
fn generate_skill_md() {
    let manifest_dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let template_path = manifest_dir.join("templates/ion-cli.md.j2");
    let out_dir = PathBuf::from(std::env::var("OUT_DIR").unwrap());

    // Rebuild when the template changes
    println!("cargo:rerun-if-changed=templates/ion-cli.md.j2");

    let template_src = std::fs::read_to_string(&template_path)
        .expect("failed to read templates/ion-cli.md.j2");

    let mut env = minijinja::Environment::new();
    env.add_template("skill.md", &template_src).unwrap();
    let tmpl = env.get_template("skill.md").unwrap();

    let rendered = tmpl
        .render(minijinja::context! {
            // -- Project init --
            example_init_no_targets => pretty(serde_json::json!({
                "success": false,
                "action_required": "target_selection",
                "data": {
                    "available_targets": [
                        {"name": "claude", "path": ".claude/skills", "detected": true},
                        {"name": "cursor", "path": ".cursor/skills", "detected": false},
                        {"name": "windsurf", "path": ".windsurf/skills", "detected": false}
                    ],
                    "hint": "Re-run with --target flags to select targets"
                }
            })),
            example_init_with_targets => pretty(serde_json::json!({
                "success": true,
                "data": {
                    "targets": {"claude": ".claude/skills", "cursor": ".cursor/skills"},
                    "manifest": "Ion.toml"
                }
            })),

            // -- Search --
            example_search => pretty(serde_json::json!({
                "success": true,
                "data": [
                    {
                        "name": "code-review",
                        "description": "Automated code review skill",
                        "source": "obra/skills/code-review",
                        "registry": "github",
                        "stars": 42
                    },
                    {
                        "name": "pr-reviewer",
                        "description": "Pull request review assistant",
                        "source": "acme/pr-reviewer",
                        "registry": "skills.sh",
                        "stars": 18
                    }
                ]
            })),

            // -- Add --
            example_add_success => pretty(serde_json::json!({
                "success": true,
                "data": {
                    "name": "code-review",
                    "installed_to": ".agents/skills/code-review/",
                    "targets": ["claude", "cursor"]
                }
            })),
            example_add_warnings => pretty(serde_json::json!({
                "success": false,
                "action_required": "validation_warnings",
                "data": {
                    "skill": "experimental-skill",
                    "warnings": [
                        {"severity": "warning", "checker": "security", "message": "Skill requests shell access"}
                    ]
                }
            })),
            example_add_warnings_accept => pretty(serde_json::json!({
                "success": true,
                "data": {
                    "name": "experimental-skill",
                    "installed_to": ".agents/skills/experimental-skill/",
                    "targets": ["claude"]
                }
            })),
            example_add_collection => pretty(serde_json::json!({
                "success": false,
                "action_required": "skill_selection",
                "data": {
                    "skills": [
                        {"name": "code-review", "status": "clean"},
                        {"name": "test-driven-dev", "status": "clean"},
                        {"name": "experimental", "status": "warnings", "warning_count": 2}
                    ]
                }
            })),
            example_add_collection_select => pretty(serde_json::json!({
                "success": true,
                "data": {
                    "name": "code-review",
                    "installed_to": ".agents/skills/code-review/",
                    "targets": ["claude"]
                }
            })),
            example_install_all => pretty(serde_json::json!({
                "success": true,
                "data": {
                    "installed": ["code-review", "test-driven-dev"],
                    "skipped": ["pinned-skill"]
                }
            })),

            // -- Remove --
            example_remove_confirm => pretty(serde_json::json!({
                "success": false,
                "action_required": "confirm_removal",
                "data": {
                    "skills": ["test-skill"]
                }
            })),
            example_remove_yes => pretty(serde_json::json!({
                "success": true,
                "data": {
                    "removed": ["test-skill"]
                }
            })),

            // -- Skill list --
            example_skill_list => pretty(serde_json::json!({
                "success": true,
                "data": []
            })),

            // -- Skill info --
            example_skill_info => pretty(serde_json::json!({
                "success": true,
                "data": {
                    "name": "code-review",
                    "description": "Automated code review skill",
                    "source_type": "Github",
                    "source": "obra/skills",
                    "path": "code-review",
                    "git_url": "https://github.com/obra/skills.git"
                }
            })),

            // -- Update --
            example_update => pretty(serde_json::json!({
                "success": true,
                "data": {
                    "updated": [
                        {"name": "code-review", "old_version": "v1.1.0", "new_version": "v1.2.0", "binary": false}
                    ],
                    "skipped": [
                        {"name": "pinned-skill", "reason": "pinned to refs/tags/v1.0"}
                    ],
                    "failed": [],
                    "up_to_date": [
                        {"name": "test-driven-dev"}
                    ]
                }
            })),
            example_update_single => pretty(serde_json::json!({
                "success": true,
                "data": {
                    "updated": [
                        {"name": "code-review", "old_version": "v1.1.0", "new_version": "v1.2.0", "binary": false}
                    ],
                    "skipped": [],
                    "failed": [],
                    "up_to_date": []
                }
            })),

            // -- Validate --
            example_validate => pretty(serde_json::json!({
                "success": true,
                "data": {
                    "skills": [{
                        "path": "test-skill/SKILL.md",
                        "name": "test-skill",
                        "findings": [],
                        "errors": 0,
                        "warnings": 0,
                        "infos": 0
                    }],
                    "total_errors": 0,
                    "total_warnings": 0,
                    "total_infos": 0
                }
            })),

            // -- Config --
            example_config_list => pretty(serde_json::json!({
                "success": true,
                "data": {
                    "targets.claude": ".claude/skills",
                    "targets.cursor": ".cursor/skills"
                }
            })),
            example_config_get => pretty(serde_json::json!({
                "success": true,
                "data": {
                    "key": "targets.claude",
                    "value": ".claude/skills"
                }
            })),
            example_config_set => pretty(serde_json::json!({
                "success": true,
                "data": {
                    "key": "targets.claude",
                    "value": ".claude/commands"
                }
            })),

            // -- Cache gc --
            example_gc_dry_run => pretty(serde_json::json!({
                "success": true,
                "data": {
                    "dry_run": true,
                    "removed": []
                }
            })),

            // -- Self --
            example_self_info => pretty(serde_json::json!({
                "success": true,
                "data": {
                    "version": "0.2.1",
                    "target": "aarch64-apple-darwin",
                    "exe": "/usr/local/bin/ion"
                }
            })),
            example_self_check => pretty(serde_json::json!({
                "success": true,
                "data": {
                    "installed": "0.2.0",
                    "latest": "0.2.1",
                    "update_available": true
                }
            })),
            example_self_update => pretty(serde_json::json!({
                "success": true,
                "data": {
                    "updated": true,
                    "old_version": "0.2.0",
                    "new_version": "0.2.1",
                    "exe": "/usr/local/bin/ion"
                }
            })),
        })
        .expect("failed to render SKILL.md template");

    std::fs::write(out_dir.join("SKILL.md"), rendered)
        .expect("failed to write SKILL.md to OUT_DIR");
}
