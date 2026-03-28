use anyhow::bail;
use ion_skill::binary;

use crate::context::ProjectContext;

pub fn run(name: &str, args: &[String], json: bool) -> anyhow::Result<()> {
    let ctx = ProjectContext::load()?;

    if !ctx.lockfile_path.exists() {
        bail!("No Ion.lock found. Run `ion install` first.");
    }

    let lockfile = ctx.lockfile()?;
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

    // Dev mode: forward to `cargo run` in the local project
    if locked.is_dev() {
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

    let status = std::process::Command::new(&bin_path)
        .args(args)
        .status()
        .map_err(|e| anyhow::anyhow!("Failed to execute {}: {}", bin_path.display(), e))?;

    std::process::exit(status.code().unwrap_or(1));
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
