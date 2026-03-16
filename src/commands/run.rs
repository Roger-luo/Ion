use anyhow::bail;
use ion_skill::binary;
use ion_skill::lockfile::Lockfile;

pub fn run(name: &str, args: &[String], json: bool) -> anyhow::Result<()> {
    let lockfile_path = std::path::Path::new("Ion.lock");
    if !lockfile_path.exists() {
        bail!("No Ion.lock found. Run `ion install` first.");
    }

    let lockfile = Lockfile::from_file(lockfile_path)?;
    let locked = lockfile.find(name).ok_or_else(|| {
        anyhow::anyhow!(
            "Skill '{}' not found in lockfile. Run `ion add {} --bin` first.",
            name,
            name
        )
    })?;

    let binary_name = locked.binary.as_deref().ok_or_else(|| {
        anyhow::anyhow!(
            "Skill '{}' is not a binary skill (no binary field in lockfile).",
            name
        )
    })?;

    let version = locked.binary_version.as_deref().ok_or_else(|| {
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
