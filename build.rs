/// Compact JSON string (single-line).
fn compact(v: serde_json::Value) -> String {
    serde_json::to_string(&v).unwrap()
}

/// Pretty-print a serde_json::Value.
fn pretty(v: serde_json::Value) -> String {
    serde_json::to_string_pretty(&v).unwrap()
}

fn main() {
    ionem::build::emit_target();

    // Expose ionem's version so the scaffold template uses the correct version
    let ionem_toml = std::fs::read_to_string("crates/ionem/Cargo.toml")
        .expect("failed to read crates/ionem/Cargo.toml");
    for line in ionem_toml.lines() {
        if let Some(ver) = line.strip_prefix("version = \"") {
            let ver = ver.trim_end_matches('"');
            println!("cargo:rustc-env=IONEM_VERSION={ver}");
            break;
        }
    }
    println!("cargo:rerun-if-changed=crates/ionem/Cargo.toml");

    ionem::build::render_skill_md_from("templates/ion-cli.md.j2", |template_src| {
        let mut env = minijinja::Environment::new();
        env.add_template("skill.md", template_src).unwrap();
        let tmpl = env.get_template("skill.md").unwrap();

        tmpl.render(minijinja::context! {
            example_init => compact(serde_json::json!({
                "success": true,
                "data": {
                    "targets": {"claude": ".claude/skills", "cursor": ".cursor/skills"},
                    "manifest": "Ion.toml"
                }
            })),

            example_search => pretty(serde_json::json!({
                "success": true,
                "data": [
                    {"name": "code-review", "description": "Automated code review skill", "source": "obra/skills/code-review", "registry": "github", "stars": 42},
                    {"name": "pr-reviewer", "description": "Pull request review assistant", "source": "acme/pr-reviewer", "registry": "skills.sh", "stars": 18}
                ]
            })),

            example_add => compact(serde_json::json!({
                "success": true,
                "data": {"name": "code-review", "installed_to": ".agents/skills/code-review/", "targets": ["claude", "cursor"]}
            })),

            example_install_all => compact(serde_json::json!({
                "success": true,
                "data": {"installed": ["code-review", "test-driven-dev"], "skipped": ["pinned-skill"]}
            })),

            example_remove => compact(serde_json::json!({
                "success": true,
                "data": {"removed": ["test-skill"]}
            })),

            example_skill_list => compact(serde_json::json!({
                "success": true, "data": []
            })),

            example_skill_info => compact(serde_json::json!({
                "success": true,
                "data": {"name": "code-review", "description": "Automated code review skill", "source_type": "Github", "source": "obra/skills", "path": "code-review"}
            })),

            example_update => pretty(serde_json::json!({
                "success": true,
                "data": {
                    "updated": [{"name": "code-review", "old_version": "v1.1.0", "new_version": "v1.2.0", "binary": false}],
                    "skipped": [{"name": "pinned-skill", "reason": "pinned to refs/tags/v1.0"}],
                    "failed": [],
                    "up_to_date": [{"name": "test-driven-dev"}]
                }
            })),

            example_validate => compact(serde_json::json!({
                "success": true,
                "data": {"skills": [{"path": "test-skill/SKILL.md", "name": "test-skill", "findings": [], "errors": 0, "warnings": 0, "infos": 0}], "total_errors": 0, "total_warnings": 0, "total_infos": 0}
            })),

            example_config => compact(serde_json::json!({
                "success": true, "data": {"targets.claude": ".claude/skills", "targets.cursor": ".cursor/skills"}
            })),

            example_gc => compact(serde_json::json!({
                "success": true, "data": {"dry_run": true, "removed": []}
            })),

            example_self_info => compact(serde_json::json!({
                "success": true, "data": {"version": "0.2.1", "target": "aarch64-apple-darwin", "exe": "/usr/local/bin/ion"}
            })),
        })
        .expect("failed to render SKILL.md template")
    });
}
