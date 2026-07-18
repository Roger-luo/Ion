//! Integration tests for `ion init`'s AGENTS.md handling.
//!
//! `ion init` should, by default and in every channel (interactive, piped,
//! `--json`), ensure the project has an ion-managed AGENTS.md: create one from
//! a detected language template (or a generic fallback) when none exists, and
//! migrate an existing CLAUDE.md into AGENTS.md. `--no-agents` opts out.

use std::process::Command;

fn ion_cmd() -> Command {
    Command::new(env!("CARGO_BIN_EXE_ion"))
}

/// A fresh project with no recognizable language files gets a generic AGENTS.md.
#[test]
fn init_creates_generic_agents_md_json() {
    let dir = tempfile::tempdir().unwrap();
    let output = ion_cmd()
        .args(["--json", "init", "--target", "claude"])
        .current_dir(dir.path())
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(0));

    // stdout must be a single, pure JSON object (no leaked progress lines).
    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value =
        serde_json::from_str(&stdout).expect("init stdout should be pure JSON");
    assert_eq!(parsed["success"], true);
    assert_eq!(parsed["data"]["agents_md"]["action"], "created");
    assert_eq!(parsed["data"]["agents_md"]["template"], "builtin:generic");

    assert!(
        dir.path().join("AGENTS.md").exists(),
        "AGENTS.md should be created for a generic project"
    );
}

/// Even piped (no TTY), a fresh init writes AGENTS.md — the behavior is not
/// gated on an interactive terminal.
#[test]
fn init_creates_agents_md_piped() {
    let dir = tempfile::tempdir().unwrap();
    let output = ion_cmd()
        .args(["init", "--target", "claude"])
        .current_dir(dir.path())
        .output()
        .unwrap();
    assert!(output.status.success());
    assert!(dir.path().join("AGENTS.md").exists());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("AGENTS.md"),
        "human output should mention AGENTS.md:\n{stdout}"
    );
}

/// A detected language (Cargo.toml → rust) picks the matching built-in template.
#[test]
fn init_detects_language_template() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("Cargo.toml"), "[package]\nname = \"x\"\n").unwrap();
    let output = ion_cmd()
        .args(["--json", "init", "--target", "claude"])
        .current_dir(dir.path())
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(0));
    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value = serde_json::from_str(&stdout).expect("pure JSON");
    assert_eq!(parsed["data"]["agents_md"]["template"], "builtin:rust");
}

/// An existing CLAUDE.md (real content, no AGENTS.md) is migrated to AGENTS.md
/// with CLAUDE.md left as a symlink — even non-interactively.
#[test]
fn init_migrates_existing_claude_md_json() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("CLAUDE.md"),
        "# House rules\n\nAlways write tests first.\n",
    )
    .unwrap();

    let output = ion_cmd()
        .args(["--json", "init", "--target", "claude"])
        .current_dir(dir.path())
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(0), "init should succeed");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value = serde_json::from_str(&stdout).expect("pure JSON");
    assert_eq!(parsed["data"]["agents_md"]["action"], "migrated");

    // AGENTS.md now holds the old CLAUDE.md content.
    let agents = std::fs::read_to_string(dir.path().join("AGENTS.md")).unwrap();
    assert!(agents.contains("Always write tests first."));

    // CLAUDE.md is now a symlink to AGENTS.md.
    let meta = std::fs::symlink_metadata(dir.path().join("CLAUDE.md")).unwrap();
    assert!(
        meta.is_symlink(),
        "CLAUDE.md should be a symlink after migration"
    );
}

/// An existing AGENTS.md (no CLAUDE.md) is left intact and CLAUDE.md is linked
/// to it — no template is attached to the user's own content.
#[test]
fn init_existing_agents_md_links_claude() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("AGENTS.md"), "# My own instructions\n").unwrap();

    let output = ion_cmd()
        .args(["init", "--target", "claude"])
        .current_dir(dir.path())
        .output()
        .unwrap();
    assert!(output.status.success());

    // Content untouched.
    let agents = std::fs::read_to_string(dir.path().join("AGENTS.md")).unwrap();
    assert!(agents.contains("My own instructions"));

    // CLAUDE.md linked to AGENTS.md.
    let meta = std::fs::symlink_metadata(dir.path().join("CLAUDE.md")).unwrap();
    assert!(meta.is_symlink(), "CLAUDE.md should link to AGENTS.md");

    // No template tracking attached to hand-written content.
    let manifest = std::fs::read_to_string(dir.path().join("Ion.toml")).unwrap();
    assert!(
        !manifest.contains("[agents]"),
        "should not attach a template to an existing hand-written AGENTS.md:\n{manifest}"
    );
}

/// Both AGENTS.md and CLAUDE.md have real content → conflict is skipped safely
/// (neither file is destroyed) in non-interactive mode.
#[test]
fn init_agents_conflict_skipped_json() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("AGENTS.md"), "# Agents\n\nA.\n").unwrap();
    std::fs::write(dir.path().join("CLAUDE.md"), "# Claude\n\nB.\n").unwrap();

    let output = ion_cmd()
        .args(["--json", "init", "--target", "claude"])
        .current_dir(dir.path())
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(0));
    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value = serde_json::from_str(&stdout).expect("pure JSON");
    assert_eq!(parsed["data"]["agents_md"]["action"], "skipped");

    // CLAUDE.md must remain a real file with its content.
    let meta = std::fs::symlink_metadata(dir.path().join("CLAUDE.md")).unwrap();
    assert!(
        !meta.is_symlink(),
        "conflicting CLAUDE.md must be preserved"
    );
    let claude = std::fs::read_to_string(dir.path().join("CLAUDE.md")).unwrap();
    assert!(claude.contains("B."));
}

/// After creating AGENTS.md from a template, init prompts the human/agent to
/// fill it in — in both the human text and the JSON `next` array.
#[test]
fn init_prompts_to_fill_in_agents_md() {
    // Human channel.
    let dir = tempfile::tempdir().unwrap();
    let output = ion_cmd()
        .args(["init", "--target", "claude"])
        .current_dir(dir.path())
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("AGENTS.md") && stdout.contains("Fill in"),
        "human output should prompt to fill in AGENTS.md:\n{stdout}"
    );

    // Agent channel.
    let dir2 = tempfile::tempdir().unwrap();
    let output = ion_cmd()
        .args(["--json", "init", "--target", "claude"])
        .current_dir(dir2.path())
        .output()
        .unwrap();
    let parsed: serde_json::Value =
        serde_json::from_str(&String::from_utf8_lossy(&output.stdout)).unwrap();
    let next = parsed["data"]["next"].as_array().unwrap();
    assert!(
        next.iter()
            .any(|s| s.as_str().is_some_and(|s| s.contains("AGENTS.md"))),
        "JSON next steps should include filling in AGENTS.md: {next:?}"
    );
}

/// `--no-agents` opts out of all AGENTS.md handling.
#[test]
fn init_no_agents_flag_skips_creation() {
    let dir = tempfile::tempdir().unwrap();
    let output = ion_cmd()
        .args(["init", "--target", "claude", "--no-agents"])
        .current_dir(dir.path())
        .output()
        .unwrap();
    assert!(output.status.success());
    assert!(
        !dir.path().join("AGENTS.md").exists(),
        "--no-agents should not create AGENTS.md"
    );
}
