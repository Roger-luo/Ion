use std::path::{Path, PathBuf};

use anyhow::bail;
use ion_skill::binary;

const REPO: &str = "Roger-luo/Ion";

pub fn info() -> anyhow::Result<()> {
    let version = env!("CARGO_PKG_VERSION");
    let target = env!("TARGET");
    let exe = std::env::current_exe().unwrap_or_else(|_| PathBuf::from("unknown"));
    println!("ion {version}");
    println!("target: {target}");
    println!("exe: {}", exe.display());
    Ok(())
}

pub fn check() -> anyhow::Result<()> {
    let current = env!("CARGO_PKG_VERSION");
    println!("installed: {current}");

    let release = binary::fetch_github_release(REPO, None)?;
    let latest = binary::parse_version_from_tag(&release.tag_name);
    println!("latest:    {latest}");

    if current == latest {
        println!("\nAlready up to date.");
    } else {
        println!("\nUpdate available: {current} -> {latest}");
        println!("Run `ion self update` to install it.");
    }
    Ok(())
}

pub fn update(version: Option<&str>) -> anyhow::Result<()> {
    let current = env!("CARGO_PKG_VERSION");
    let tag = version.map(|v| {
        if v.starts_with('v') {
            v.to_string()
        } else {
            format!("v{v}")
        }
    });

    let release = binary::fetch_github_release(REPO, tag.as_deref())?;
    let latest = binary::parse_version_from_tag(&release.tag_name);

    if version.is_none() && current == latest {
        println!("Already up to date ({current}).");
        return Ok(());
    }

    println!("Updating ion {current} -> {latest}...");

    let platform = binary::Platform::detect();
    let asset_names: Vec<String> = release.assets.iter().map(|a| a.name.clone()).collect();

    let asset_name = match platform.match_asset("ion", &asset_names) {
        Some(name) => name,
        None => {
            println!("No prebuilt binary found for {}.", platform.target_triple());
            println!(
                "Available assets: {}",
                asset_names.join(", ")
            );
            bail!(
                "Install from source instead:\n  cargo install --git https://github.com/{REPO} --force"
            );
        }
    };

    let asset = release
        .assets
        .iter()
        .find(|a| a.name == asset_name)
        .expect("matched asset must exist in release");

    let tmp_dir = tempfile::tempdir()?;
    let archive_path = tmp_dir.path().join(&asset_name);
    println!("Downloading {asset_name}...");
    binary::download_file(&asset.browser_download_url, &archive_path)?;

    let extract_dir = tmp_dir.path().join("extracted");
    binary::extract_tar_gz(&archive_path, &extract_dir)?;

    let new_binary = binary::find_binary_in_dir(&extract_dir, "ion")?;
    let installed_path = replace_exe(&new_binary)?;

    println!("Updated to ion {latest}");
    println!("exe: {}", installed_path.display());
    Ok(())
}

fn replace_exe(new_binary: &Path) -> anyhow::Result<PathBuf> {
    let current_exe = std::env::current_exe()?.canonicalize()?;
    let backup = current_exe.with_extension("old");

    // Move current executable to backup
    if let Err(e) = std::fs::rename(&current_exe, &backup) {
        if e.kind() == std::io::ErrorKind::PermissionDenied {
            bail!(
                "Permission denied. Try: sudo ion self update"
            );
        }
        bail!("Failed to back up current executable: {e}");
    }

    // Copy new binary into place
    if let Err(e) = std::fs::copy(new_binary, &current_exe) {
        // Restore backup on failure
        let _ = std::fs::rename(&backup, &current_exe);
        bail!("Failed to install new binary: {e}");
    }

    // Set executable permissions on unix
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&current_exe, std::fs::Permissions::from_mode(0o755))?;
    }

    // Clean up backup
    let _ = std::fs::remove_file(&backup);

    Ok(current_exe)
}
