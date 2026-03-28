//! Cargo CLI wrappers.
//!
//! Use [`require()`] to verify `cargo` is installed, then call methods on the
//! returned [`Cargo`] handle:
//!
//! ```ignore
//! let cargo = cargo::require()?;
//! let proj = cargo.project(&manifest_path);
//! let meta = proj.metadata()?;
//! proj.build_release()?;
//! let output = proj.run("my-bin", &["arg1"])?;
//! ```

use std::path::Path;

use super::{Cli, CliError, Result};

/// The `cargo` CLI descriptor.
pub const CLI: Cli = Cli {
    name: "cargo",
    hint: "Install from https://rustup.rs",
};

/// Verify `cargo` is installed and return a handle to run commands.
pub fn require() -> Result<Cargo> {
    CLI.require()?;
    Ok(Cargo)
}

/// A validated handle proving the `cargo` CLI is available.
///
/// Obtained via [`require()`]. Context constructors and standalone
/// operations live here.
pub struct Cargo;

impl Cargo {
    /// Create a [`Project`] context bound to the given `Cargo.toml` path.
    pub fn project<'a>(&self, manifest_path: &'a Path) -> Project<'a> {
        project(manifest_path)
    }

    /// Run `cargo init` to create a new project.
    pub fn init(&self, path: &Path, name: &str) -> Result<()> {
        init(path, name)
    }
}

/// Parsed output of `cargo metadata`.
#[derive(Debug)]
pub struct CargoMetadata {
    pub package_name: String,
    pub version: String,
    /// Names of all binary targets.
    pub binary_targets: Vec<String>,
    /// Path to the manifest file.
    pub manifest_path: String,
}

/// Run `cargo metadata --no-deps` and parse the result.
/// `manifest_path` should point to the Cargo.toml file.
pub fn metadata(manifest_path: &Path) -> Result<CargoMetadata> {
    let json_str = CLI.run_command(
        CLI.command()
            .args([
                "metadata",
                "--no-deps",
                "--format-version",
                "1",
                "--manifest-path",
            ])
            .arg(manifest_path),
    )?;

    let json: serde_json::Value =
        serde_json::from_str(&json_str).map_err(|e| CliError::Failed {
            cli: CLI.name.to_string(),
            code: -1,
            stderr: format!("Failed to parse cargo metadata JSON: {e}"),
        })?;

    let packages = json["packages"]
        .as_array()
        .ok_or_else(|| CliError::Failed {
            cli: CLI.name.to_string(),
            code: -1,
            stderr: "No packages in cargo metadata output".to_string(),
        })?;

    // Find the package whose manifest_path matches, fall back to first package.
    let manifest_str = manifest_path.to_string_lossy();
    let package = packages
        .iter()
        .find(|p| {
            p["manifest_path"]
                .as_str()
                .is_some_and(|mp| mp == manifest_str.as_ref())
        })
        .or_else(|| packages.first())
        .ok_or_else(|| CliError::Failed {
            cli: CLI.name.to_string(),
            code: -1,
            stderr: "No packages found in cargo metadata".to_string(),
        })?;

    let package_name = package["name"].as_str().unwrap_or("unknown").to_string();
    let version = package["version"].as_str().unwrap_or("0.0.0").to_string();
    let manifest_path_str = package["manifest_path"].as_str().unwrap_or("").to_string();

    let mut binary_targets = Vec::new();
    if let Some(targets) = package["targets"].as_array() {
        for target in targets {
            let is_bin = target["kind"]
                .as_array()
                .is_some_and(|kinds| kinds.iter().any(|k| k.as_str() == Some("bin")));
            if is_bin && let Some(name) = target["name"].as_str() {
                binary_targets.push(name.to_string());
            }
        }
    }

    Ok(CargoMetadata {
        package_name,
        version,
        binary_targets,
        manifest_path: manifest_path_str,
    })
}

/// Run `cargo build --release` for a project.
pub fn build_release(manifest_path: &Path) -> Result<()> {
    CLI.run_status(
        CLI.command()
            .args(["build", "--release", "--manifest-path"])
            .arg(manifest_path),
    )
}

/// Run `cargo build --release` for a specific binary target.
pub fn build_release_bin(manifest_path: &Path, bin: &str) -> Result<()> {
    CLI.run_status(
        CLI.command()
            .args(["build", "--release", "--manifest-path"])
            .arg(manifest_path)
            .args(["--bin", bin]),
    )
}

/// Run `cargo run -q` with the given arguments. Returns stdout.
/// `manifest_path` should point to the Cargo.toml file.
/// `bin` is the binary target name.
/// `args` are passed after `--` to the binary.
pub fn run(manifest_path: &Path, bin: &str, args: &[&str]) -> Result<String> {
    CLI.run_command(
        CLI.command()
            .args(["run", "-q", "--manifest-path"])
            .arg(manifest_path)
            .args(["--bin", bin, "--"])
            .args(args),
    )
}

/// Run `cargo run` inheriting stdio (for interactive use).
pub fn run_interactive(manifest_path: &Path, bin: &str, args: &[&str]) -> Result<()> {
    CLI.run_status(
        CLI.command()
            .args(["run", "-q", "--manifest-path"])
            .arg(manifest_path)
            .args(["--bin", bin, "--"])
            .args(args),
    )
}

/// Run `cargo init` to create a new project.
pub fn init(path: &Path, name: &str) -> Result<()> {
    CLI.run_status(CLI.command().arg("init").arg(path).args(["--name", name]))
}

// ---------------------------------------------------------------------------
// Project context
// ---------------------------------------------------------------------------

/// Create a [`Project`] context bound to the given `Cargo.toml` path.
pub fn project(manifest_path: &Path) -> Project<'_> {
    Project { manifest_path }
}

/// A Cargo project context that binds a manifest path, so you can call
/// multiple operations without repeating the path.
pub struct Project<'a> {
    manifest_path: &'a Path,
}

impl<'a> Project<'a> {
    /// Run `cargo metadata --no-deps` and parse the result.
    pub fn metadata(&self) -> Result<CargoMetadata> {
        metadata(self.manifest_path)
    }

    /// Run `cargo build --release`.
    pub fn build_release(&self) -> Result<()> {
        build_release(self.manifest_path)
    }

    /// Run `cargo build --release` for a specific binary target.
    pub fn build_release_bin(&self, bin: &str) -> Result<()> {
        build_release_bin(self.manifest_path, bin)
    }

    /// Run `cargo run -q` with the given arguments. Returns stdout.
    pub fn run(&self, bin: &str, args: &[&str]) -> Result<String> {
        self::run(self.manifest_path, bin, args)
    }

    /// Run `cargo run` inheriting stdio (for interactive use).
    pub fn run_interactive(&self, bin: &str, args: &[&str]) -> Result<()> {
        run_interactive(self.manifest_path, bin, args)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_metadata_parses_cargo_project() {
        let dir = tempfile::tempdir().unwrap();
        let cargo_toml = dir.path().join("Cargo.toml");
        fs::write(
            &cargo_toml,
            r#"[package]
name = "test-proj"
version = "1.2.3"
edition = "2021"

[[bin]]
name = "test-proj"
path = "src/main.rs"
"#,
        )
        .unwrap();

        let src_dir = dir.path().join("src");
        fs::create_dir_all(&src_dir).unwrap();
        fs::write(src_dir.join("main.rs"), "fn main() {}\n").unwrap();

        let meta = metadata(&cargo_toml).unwrap();
        assert_eq!(meta.package_name, "test-proj");
        assert_eq!(meta.version, "1.2.3");
        assert!(meta.binary_targets.contains(&"test-proj".to_string()));
        assert!(meta.manifest_path.ends_with("Cargo.toml"));
    }

    #[test]
    fn test_metadata_no_explicit_bin_target() {
        // When no [[bin]] is specified, cargo infers a binary from src/main.rs
        let dir = tempfile::tempdir().unwrap();
        let cargo_toml = dir.path().join("Cargo.toml");
        fs::write(
            &cargo_toml,
            r#"[package]
name = "implicit-bin"
version = "0.5.0"
edition = "2021"
"#,
        )
        .unwrap();

        let src_dir = dir.path().join("src");
        fs::create_dir_all(&src_dir).unwrap();
        fs::write(src_dir.join("main.rs"), "fn main() {}\n").unwrap();

        let meta = metadata(&cargo_toml).unwrap();
        assert_eq!(meta.package_name, "implicit-bin");
        assert_eq!(meta.version, "0.5.0");
        assert!(meta.binary_targets.contains(&"implicit-bin".to_string()));
    }

    #[test]
    fn test_init_creates_cargo_project() {
        let dir = tempfile::tempdir().unwrap();
        let project_dir = dir.path().join("my-new-project");

        init(&project_dir, "my-new-project").unwrap();

        assert!(project_dir.join("Cargo.toml").exists());
        assert!(project_dir.join("src").join("main.rs").exists());

        // Verify the created project has the right name via metadata
        let meta = metadata(&project_dir.join("Cargo.toml")).unwrap();
        assert_eq!(meta.package_name, "my-new-project");
    }
}
