use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use anyhow::bail;
use ion_skill::binary;

const REPO: &str = "Roger-luo/Ion";
/// Tag prefix used by release-plz for the ion crate
const TAG_PREFIX: &str = "ion-v";
/// How often to check for updates (24 hours)
const UPDATE_CHECK_INTERVAL: Duration = Duration::from_secs(24 * 60 * 60);

pub fn info(json: bool) -> anyhow::Result<()> {
    let version = env!("CARGO_PKG_VERSION");
    let target = env!("TARGET");
    let exe = std::env::current_exe().unwrap_or_else(|_| PathBuf::from("unknown"));

    if json {
        crate::json::print_success(serde_json::json!({
            "version": version,
            "target": target,
            "exe": exe.display().to_string(),
        }));
        return Ok(());
    }

    println!("ion {version}");
    println!("target: {target}");
    println!("exe: {}", exe.display());
    Ok(())
}

pub fn check(json: bool) -> anyhow::Result<()> {
    let current = env!("CARGO_PKG_VERSION");

    let release = binary::fetch_latest_release_by_tag_prefix(REPO, TAG_PREFIX)?;
    let latest = binary::parse_version_from_tag(&release.tag_name);

    if json {
        crate::json::print_success(serde_json::json!({
            "installed": current,
            "latest": latest,
            "update_available": current != latest,
        }));
        return Ok(());
    }

    println!("installed: {current}");
    println!("latest:    {latest}");

    if current == latest {
        println!("\nAlready up to date.");
    } else {
        println!("\nUpdate available: {current} -> {latest}");
        println!("Run `ion self update` to install it.");
    }
    Ok(())
}

pub fn update(version: Option<&str>, json: bool) -> anyhow::Result<()> {
    let current = env!("CARGO_PKG_VERSION");
    let release = match version {
        Some(v) => {
            let ver = v.strip_prefix('v').unwrap_or(v);
            let tag = format!("{TAG_PREFIX}{ver}");
            binary::fetch_github_release(REPO, Some(&tag))?
        }
        None => binary::fetch_latest_release_by_tag_prefix(REPO, TAG_PREFIX)?,
    };
    let latest = binary::parse_version_from_tag(&release.tag_name);

    if version.is_none() && current == latest {
        if json {
            crate::json::print_success(serde_json::json!({
                "updated": false,
                "version": current,
                "message": "Already up to date",
            }));
            return Ok(());
        }
        println!("Already up to date ({current}).");
        return Ok(());
    }

    if !json {
        println!("Updating ion {current} -> {latest}...");
    }

    let platform = binary::Platform::detect();
    let asset_names: Vec<String> = release.assets.iter().map(|a| a.name.clone()).collect();

    let asset_name = match platform.match_asset("ion", &asset_names) {
        Some(name) => name,
        None => {
            if !json {
                println!("No prebuilt binary found for {}.", platform.target_triple());
                println!("Available assets: {}", asset_names.join(", "));
            }
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
    if !json {
        println!("Downloading {asset_name}...");
    }
    binary::download_file(&asset.browser_download_url, &archive_path)?;

    let extract_dir = tmp_dir.path().join("extracted");
    binary::extract_tar_gz(&archive_path, &extract_dir)?;

    let new_binary = binary::find_binary_in_dir(&extract_dir, "ion")?;
    let installed_path = replace_exe(&new_binary)?;

    if json {
        crate::json::print_success(serde_json::json!({
            "updated": true,
            "old_version": current,
            "new_version": latest,
            "exe": installed_path.display().to_string(),
        }));
        return Ok(());
    }

    println!("Updated to ion {latest}");
    println!("exe: {}", installed_path.display());
    Ok(())
}

/// Silently check for updates and print a hint to stderr if one is available.
/// Uses a cache file to avoid hitting the GitHub API on every invocation.
pub fn check_for_update_hint() {
    let _ = check_for_update_hint_inner();
}

fn update_check_cache_path() -> Option<PathBuf> {
    dirs::data_dir().map(|d| d.join("ion").join("update_check.json"))
}

fn check_for_update_hint_inner() -> Option<()> {
    let cache_path = update_check_cache_path()?;
    let current = env!("CARGO_PKG_VERSION");

    // Check if we have a recent cached result
    if let Ok(contents) = std::fs::read_to_string(&cache_path)
        && let Ok(cached) = serde_json::from_str::<serde_json::Value>(&contents)
    {
        let ts = cached.get("timestamp")?.as_u64()?;
        let now = SystemTime::now().duration_since(UNIX_EPOCH).ok()?.as_secs();
        if now.saturating_sub(ts) < UPDATE_CHECK_INTERVAL.as_secs() {
            // Cache is fresh — show hint if update was available
            let latest = cached.get("latest")?.as_str()?;
            if latest != current {
                print_update_hint(current, latest);
            }
            return Some(());
        }
    }

    // Cache is stale or missing — fetch from GitHub (best-effort)
    let release = binary::fetch_latest_release_by_tag_prefix(REPO, TAG_PREFIX).ok()?;
    let latest = binary::parse_version_from_tag(&release.tag_name);

    // Write cache
    let now = SystemTime::now().duration_since(UNIX_EPOCH).ok()?.as_secs();
    let cache_value = serde_json::json!({
        "timestamp": now,
        "latest": latest,
    });
    if let Some(parent) = cache_path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let _ = std::fs::write(&cache_path, cache_value.to_string());

    if latest != current {
        print_update_hint(current, latest);
    }

    Some(())
}

fn print_update_hint(current: &str, latest: &str) {
    eprintln!(
        "\nUpdate available: ion {} -> {} — run `ion self update` to install",
        current, latest
    );
}

pub fn uninstall(yes: bool, json: bool) -> anyhow::Result<()> {
    let exe = std::env::current_exe()?.canonicalize()?;

    // Collect all directories/files to remove
    let data_dir = dirs::data_dir().map(|d| d.join("ion"));
    let config_dir = dirs::config_dir().map(|d| d.join("ion"));

    if json && !yes {
        let mut paths = Vec::new();
        if let Some(ref d) = data_dir {
            paths.push(d.display().to_string());
        }
        if let Some(ref d) = config_dir {
            paths.push(d.display().to_string());
        }
        paths.push(exe.display().to_string());
        crate::json::print_action_required(
            "confirm_uninstall",
            serde_json::json!({
                "paths": paths,
            }),
        );
    }

    if !json {
        println!("This will remove:");
        if let Some(ref d) = data_dir
            && d.exists()
        {
            println!("  {} (repos, cache, registry, binaries)", d.display());
        }
        if let Some(ref d) = config_dir
            && d.exists()
        {
            println!("  {} (config, starred repos)", d.display());
        }
        println!("  {} (binary)", exe.display());
    }

    if !yes {
        use std::io::Write;
        println!();
        print!("Are you sure? [y/N] ");
        std::io::stdout().flush()?;
        let mut answer = String::new();
        std::io::stdin().read_line(&mut answer)?;
        if !answer.trim().eq_ignore_ascii_case("y")
            && !answer.trim().eq_ignore_ascii_case("yes")
        {
            bail!("Aborted.");
        }
    }

    let mut removed = Vec::new();

    // Remove data directory (repos, search_cache, bin, registry, update_check, builtin)
    if let Some(ref d) = data_dir
        && d.exists()
    {
        std::fs::remove_dir_all(d)?;
        removed.push(d.display().to_string());
        if !json {
            println!("  Removed {}", d.display());
        }
    }

    // Remove config directory (config.toml, starred.json)
    if let Some(ref d) = config_dir
        && d.exists()
    {
        std::fs::remove_dir_all(d)?;
        removed.push(d.display().to_string());
        if !json {
            println!("  Removed {}", d.display());
        }
    }

    // Delete the binary itself — must be last
    let exe_path = exe.display().to_string();
    if let Err(e) = std::fs::remove_file(&exe) {
        if e.kind() == std::io::ErrorKind::PermissionDenied {
            if json {
                crate::json::print_error(&format!(
                    "Permission denied removing {}. Try: sudo ion self uninstall --yes",
                    exe_path
                ));
            }
            bail!(
                "Permission denied removing {}. Try: sudo ion self uninstall --yes",
                exe_path
            );
        }
        bail!("Failed to remove binary: {e}");
    }
    removed.push(exe_path);

    if json {
        crate::json::print_success(serde_json::json!({
            "removed": removed,
        }));
        return Ok(());
    }

    println!("  Removed binary");
    println!("\nion has been uninstalled.");
    Ok(())
}

fn replace_exe(new_binary: &Path) -> anyhow::Result<PathBuf> {
    let current_exe = std::env::current_exe()?.canonicalize()?;
    let backup = current_exe.with_extension("old");

    // Move current executable to backup
    if let Err(e) = std::fs::rename(&current_exe, &backup) {
        if e.kind() == std::io::ErrorKind::PermissionDenied {
            bail!("Permission denied. Try: sudo ion self update");
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
