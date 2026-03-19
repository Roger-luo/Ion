use std::path::PathBuf;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use anyhow::bail;
use ion_skill::binary;
use ion_skill::self_update::{SelfManager, is_newer_version};

const REPO: &str = "Roger-luo/Ion";
/// Tag prefix used by release-plz for the ion crate
const TAG_PREFIX: &str = "ion-v";
/// How often to check for updates (24 hours)
const UPDATE_CHECK_INTERVAL: Duration = Duration::from_secs(24 * 60 * 60);

/// The embedded SKILL.md for the ion-cli builtin skill.
const SKILL_CONTENT: &str = include_str!(concat!(env!("OUT_DIR"), "/SKILL.md"));

fn manager() -> SelfManager {
    SelfManager::new(REPO, "ion", TAG_PREFIX, env!("CARGO_PKG_VERSION"), env!("TARGET"))
}

/// Detect whether the binary was installed by an external package manager.
/// Returns `Some("manager name")` if managed, `None` if self-update is safe.
fn detect_package_manager() -> Option<&'static str> {
    let exe = std::env::current_exe().ok()?.canonicalize().ok()?;
    let exe_str = exe.to_str()?;

    // cargo install → ~/.cargo/bin/ion
    if let Some(cargo_home) = std::env::var_os("CARGO_HOME") {
        if exe.starts_with(cargo_home) {
            return Some("cargo");
        }
    } else if let Some(home) = dirs::home_dir() {
        if exe.starts_with(home.join(".cargo/bin")) {
            return Some("cargo");
        }
    }

    // Homebrew on macOS — /opt/homebrew/bin/ or /usr/local/Cellar/
    if exe_str.contains("/Cellar/") || exe_str.starts_with("/opt/homebrew/bin/") {
        return Some("brew");
    }

    None
}

pub fn skill() {
    print!("{}", SKILL_CONTENT);
}

pub fn info(json: bool) -> anyhow::Result<()> {
    let mgr = manager();
    let info = mgr.info();

    if json {
        crate::json::print_success(serde_json::json!({
            "version": info.version,
            "target": info.target,
            "exe": info.exe.display().to_string(),
        }));
        return Ok(());
    }

    mgr.print_info();
    Ok(())
}

pub fn check(json: bool) -> anyhow::Result<()> {
    let mgr = manager();
    let result = mgr.check()?;

    if json {
        crate::json::print_success(serde_json::json!({
            "installed": result.installed,
            "latest": result.latest,
            "update_available": result.update_available,
        }));
        return Ok(());
    }

    println!("installed: {}", result.installed);
    println!("latest:    {}", result.latest);

    if !result.update_available {
        println!("\nAlready up to date.");
    } else {
        let command = match detect_package_manager() {
            Some("cargo") => "cargo install --git https://github.com/Roger-luo/Ion --force",
            Some("brew") => "brew upgrade ion",
            _ => "ion self update",
        };
        println!("\nUpdate available: {} -> {}", result.installed, result.latest);
        println!("Run `{command}` to install it.");
    }
    Ok(())
}

pub fn update(version: Option<&str>, json: bool) -> anyhow::Result<()> {
    if let Some(pkg_manager) = detect_package_manager() {
        let hint = match pkg_manager {
            "cargo" => "cargo install --git https://github.com/Roger-luo/Ion --force",
            "brew" => "brew upgrade ion",
            _ => "your package manager",
        };
        if json {
            crate::json::print_error(&format!(
                "ion was installed via {pkg_manager}. Update with: {hint}"
            ));
        }
        bail!(
            "ion was installed via {pkg_manager}, so `ion self update` cannot safely replace it.\n\
             Update with: {hint}"
        );
    }

    let mgr = manager();
    let current = env!("CARGO_PKG_VERSION");

    if !json {
        println!("Checking for updates...");
    }

    let result = mgr.update(version)?;

    if !result.updated {
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

    if json {
        crate::json::print_success(serde_json::json!({
            "updated": true,
            "old_version": result.old_version,
            "new_version": result.new_version,
            "exe": result.exe.display().to_string(),
        }));
        return Ok(());
    }

    println!("Updated to ion {}", result.new_version);
    println!("exe: {}", result.exe.display());
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
            if is_newer_version(current, latest) {
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

    if is_newer_version(current, latest) {
        print_update_hint(current, latest);
    }

    Some(())
}

fn print_update_hint(current: &str, latest: &str) {
    let command = match detect_package_manager() {
        Some("cargo") => "cargo install --git https://github.com/Roger-luo/Ion --force",
        Some("brew") => "brew upgrade ion",
        _ => "ion self update",
    };
    eprintln!(
        "\nUpdate available: ion {} -> {} — run `{}` to install",
        current, latest, command
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
        if !answer.trim().eq_ignore_ascii_case("y") && !answer.trim().eq_ignore_ascii_case("yes") {
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
