use std::io::{BufRead, BufReader, Write};
use std::path::Path;
use std::process::Stdio;

use anyhow::bail;
use ion_skill::binary;

use crate::context::WorkspaceContext;

pub fn run(
    name: &str,
    args: &[String],
    json: bool,
    project_flags: &[String],
) -> anyhow::Result<()> {
    let ws = WorkspaceContext::load(project_flags)?;
    let project = ws.single_project()?;

    if !project.lockfile_path.exists() {
        bail!("No Ion.lock found. Run `ion install` first.");
    }

    let lockfile = project.lockfile()?;
    let locked = lockfile.find(name).ok_or_else(|| {
        anyhow::anyhow!(
            "Skill '{}' not found in lockfile. Run `ion add {} --bin` first.",
            name,
            name
        )
    })?;

    let binary_name = locked.binary_name().ok_or_else(|| {
        anyhow::anyhow!(
            "Skill '{}' is not a binary skill (no binary field in lockfile).",
            name
        )
    })?;

    // Dev mode: refresh skill from last-built debug binary, then forward to `cargo run`.
    // The refresh runs before `cargo run` (which calls std::process::exit) using the binary
    // from the previous build. This means skill updates lag one invocation, which is fine
    // for a dev workflow: build → first run uses stale skill, second run uses fresh skill.
    if locked.is_dev() {
        let project = ws.single_project()?;
        let options = ws.merged_options_for(project)?;
        let installer = ws.installer_for(project, &options);
        let skill_md_path = installer.skill_dir(name).join("SKILL.md");
        try_refresh_dev_skill(&locked.source, binary_name, &skill_md_path);
        return run_dev(&locked.source, binary_name, args, json);
    }

    let version = locked.binary_version().ok_or_else(|| {
        anyhow::anyhow!(
            "Skill '{}' has no binary_version in lockfile. Try `ion install`.",
            name
        )
    })?;

    let bin_path = binary::binary_path(binary_name, version);
    if !bin_path.exists() {
        bail!(
            "Binary '{}' v{} not found at {}. Run `ion install` to download it.",
            binary_name,
            version,
            bin_path.display()
        );
    }

    if json {
        // In JSON mode, capture output and wrap exit code
        let output = std::process::Command::new(&bin_path)
            .args(args)
            .output()
            .map_err(|e| anyhow::anyhow!("Failed to execute {}: {}", bin_path.display(), e))?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        let code = output.status.code().unwrap_or(1);

        if output.status.success() {
            crate::json::print_success(serde_json::json!({
                "binary": binary_name,
                "version": version,
                "exit_code": code,
                "stdout": stdout,
                "stderr": stderr,
            }));
        } else {
            println!(
                "{}",
                serde_json::to_string_pretty(&serde_json::json!({
                    "success": false,
                    "error": format!("Binary '{}' exited with code {}", binary_name, code),
                    "binary": binary_name,
                    "version": version,
                    "exit_code": code,
                    "stdout": stdout,
                    "stderr": stderr,
                }))
                .unwrap()
            );
            std::process::exit(code);
        }
    }

    run_with_session(&bin_path, args)
}

/// Refresh the skill file from the last-built dev binary if it is newer than the
/// current SKILL.md. Best-effort: all errors are silently ignored.
fn try_refresh_dev_skill(source_path: &str, binary_name: &str, skill_md_path: &Path) {
    let bin_name = if cfg!(windows) {
        format!("{binary_name}.exe")
    } else {
        binary_name.to_string()
    };
    let debug_bin = Path::new(source_path)
        .join("target")
        .join("debug")
        .join(&bin_name);

    if !debug_bin.exists() {
        return;
    }

    // Skip if the skill file is already up to date (binary not newer).
    let should_refresh = match (
        debug_bin.metadata().and_then(|m| m.modified()),
        skill_md_path.metadata().and_then(|m| m.modified()),
    ) {
        (Ok(bin_mtime), Ok(skill_mtime)) => bin_mtime > skill_mtime,
        _ => true,
    };
    if !should_refresh {
        return;
    }

    let Ok(content) = binary::generate_skill_md(&debug_bin) else {
        return;
    };
    let current = std::fs::read_to_string(skill_md_path).unwrap_or_default();
    if current != content {
        let _ = std::fs::write(skill_md_path, content);
    }
}

/// Run a dev-mode binary skill by forwarding to `cargo run` in the project directory.
fn run_dev(
    source_path: &str,
    binary_name: &str,
    args: &[String],
    json: bool,
) -> anyhow::Result<()> {
    let project_path = std::path::PathBuf::from(source_path);
    let manifest_path = project_path.join("Cargo.toml");

    if !manifest_path.exists() {
        bail!(
            "Cargo.toml not found at {}. Is the dev binary skill path correct?",
            manifest_path.display()
        );
    }

    let str_args: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
    let proj = ionem::shell::cargo::project(&manifest_path);

    if json {
        match proj.run(binary_name, &str_args) {
            Ok(stdout) => {
                crate::json::print_success(serde_json::json!({
                    "binary": binary_name,
                    "dev": true,
                    "exit_code": 0,
                    "stdout": stdout,
                    "stderr": "",
                }));
            }
            Err(ionem::shell::CliError::Failed { code, stderr, .. }) => {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&serde_json::json!({
                        "success": false,
                        "error": format!("cargo run exited with code {}", code),
                        "binary": binary_name,
                        "dev": true,
                        "exit_code": code,
                        "stdout": "",
                        "stderr": stderr,
                    }))
                    .unwrap()
                );
                std::process::exit(code);
            }
            Err(e) => return Err(e.into()),
        }
        return Ok(());
    }

    if let Err(e) = proj.run_interactive(binary_name, &str_args) {
        if let ionem::shell::CliError::Failed { code, .. } = e {
            std::process::exit(code);
        }
        return Err(e.into());
    }
    std::process::exit(0);
}

// ---------------------------------------------------------------------------
// Session-aware runner — intercepts <request-tool> tags and prompts the user
// ---------------------------------------------------------------------------

/// Parse a `<request-tool>ToolName</request-tool>` tag from a line of output.
fn parse_request_tool(line: &str) -> Option<String> {
    let trimmed = line.trim();
    let inner = trimmed
        .strip_prefix("<request-tool>")
        .and_then(|rest| rest.strip_suffix("</request-tool>"))?;
    let tool = inner.trim();
    if tool.is_empty() {
        return None;
    }
    Some(tool.to_string())
}

/// Run a binary skill with session management.
///
/// Spawns the binary with piped stdout/stdin, reads its output line-by-line,
/// and intercepts `<request-tool>` tags. When a tag is detected the user is
/// prompted to approve or deny the tool. Approved tools are communicated
/// back to the child on stdin as `TOOL_GRANTED:<name>` and recorded in the
/// session's allowed-tools set.
///
/// If the `ION_SESSION_STATE` environment variable is set, the final session
/// state (including all granted tools) is written to that path as JSON on exit.
fn run_with_session(bin_path: &Path, args: &[String]) -> anyhow::Result<()> {
    let mut child = std::process::Command::new(bin_path)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
        .map_err(|e| anyhow::anyhow!("Failed to execute {}: {}", bin_path.display(), e))?;

    let child_stdout = child
        .stdout
        .take()
        .ok_or_else(|| anyhow::anyhow!("Failed to capture child stdout"))?;
    let mut child_stdin = child
        .stdin
        .take()
        .ok_or_else(|| anyhow::anyhow!("Failed to capture child stdin"))?;

    let reader = BufReader::new(child_stdout);
    let mut allowed_tools: Vec<String> = Vec::new();

    for line in reader.lines() {
        let line = line?;
        if let Some(tool) = parse_request_tool(&line) {
            eprint!("Tool '{}' requested. Allow? [y/N] ", tool);
            std::io::stderr().flush()?;

            let mut response = String::new();
            std::io::stdin().read_line(&mut response)?;

            if response.trim().eq_ignore_ascii_case("y") {
                allowed_tools.push(tool.clone());
                writeln!(child_stdin, "TOOL_GRANTED:{tool}")?;
                child_stdin.flush()?;
            } else {
                writeln!(child_stdin, "TOOL_DENIED:{tool}")?;
                child_stdin.flush()?;
            }
        } else {
            println!("{line}");
        }
    }

    // Persist session state when ION_SESSION_STATE is set
    if let Ok(state_path) = std::env::var("ION_SESSION_STATE") {
        let state = serde_json::json!({ "allowed_tools": allowed_tools });
        std::fs::write(&state_path, serde_json::to_string_pretty(&state)?)?;
    }

    let status = child.wait()?;
    std::process::exit(status.code().unwrap_or(1));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_request_tool_basic() {
        assert_eq!(
            parse_request_tool("<request-tool>Bash</request-tool>"),
            Some("Bash".to_string())
        );
    }

    #[test]
    fn parse_request_tool_with_whitespace() {
        assert_eq!(
            parse_request_tool("  <request-tool> Read </request-tool>  "),
            Some("Read".to_string())
        );
    }

    #[test]
    fn parse_request_tool_empty() {
        assert_eq!(parse_request_tool("<request-tool></request-tool>"), None);
    }

    #[test]
    fn parse_request_tool_not_a_tag() {
        assert_eq!(parse_request_tool("hello world"), None);
        assert_eq!(parse_request_tool("<plan>do stuff</plan>"), None);
    }
}
