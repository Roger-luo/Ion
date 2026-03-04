use anyhow::bail;
use ion_skill::binary;
use ion_skill::lockfile::Lockfile;

pub fn run(name: &str, args: &[String]) -> anyhow::Result<()> {
    let lockfile_path = std::path::Path::new("Ion.lock");
    if !lockfile_path.exists() {
        bail!("No Ion.lock found. Run `ion install` first.");
    }

    let lockfile = Lockfile::from_file(lockfile_path)?;
    let locked = lockfile.find(name).ok_or_else(|| {
        anyhow::anyhow!("Skill '{}' not found in lockfile. Run `ion add {} --bin` first.", name, name)
    })?;

    let binary_name = locked.binary.as_deref().ok_or_else(|| {
        anyhow::anyhow!("Skill '{}' is not a binary skill (no binary field in lockfile).", name)
    })?;

    let version = locked.binary_version.as_deref().ok_or_else(|| {
        anyhow::anyhow!("Skill '{}' has no binary_version in lockfile. Try `ion install`.", name)
    })?;

    let bin_path = binary::binary_path(binary_name, version);
    if !bin_path.exists() {
        bail!(
            "Binary '{}' v{} not found at {}. Run `ion install` to download it.",
            binary_name, version, bin_path.display()
        );
    }

    let status = std::process::Command::new(&bin_path)
        .args(args)
        .status()
        .map_err(|e| anyhow::anyhow!("Failed to execute {}: {}", bin_path.display(), e))?;

    std::process::exit(status.code().unwrap_or(1));
}
