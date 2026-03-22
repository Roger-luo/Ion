use std::path::Path;

const CI_TEMPLATE: &str = r#"name: CI

on:
  push:
    branches: [main]
  pull_request:
    branches: [main]

env:
  CARGO_TERM_COLOR: always

jobs:
  fmt:
    name: Format
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          components: rustfmt
      - run: cargo fmt --all --check

  clippy:
    name: Clippy
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          components: clippy
      - uses: Swatinem/rust-cache@v2
      - run: cargo clippy --all-targets --all-features -- -D warnings

  test:
    name: Test
    strategy:
      matrix:
        os: [ubuntu-latest, macos-latest]
    runs-on: ${{ matrix.os }}
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - uses: Swatinem/rust-cache@v2
      - run: cargo test --all-features
"#;

const RELEASE_TEMPLATE: &str = r#"name: Release

on:
  push:
    tags:
      - "v*"

permissions:
  contents: write

jobs:
  build:
    strategy:
      matrix:
        include:
          - target: aarch64-apple-darwin
            os: macos-latest
          - target: x86_64-apple-darwin
            os: macos-latest
          - target: x86_64-unknown-linux-gnu
            os: ubuntu-latest
          - target: aarch64-unknown-linux-gnu
            os: ubuntu-latest

    runs-on: ${{ matrix.os }}

    steps:
      - uses: actions/checkout@v4

      - name: Install Rust toolchain
        uses: dtolnay/rust-toolchain@stable
        with:
          targets: ${{ matrix.target }}

      - name: Install cross-compilation tools
        if: matrix.target == 'aarch64-unknown-linux-gnu'
        run: |
          sudo apt-get update
          sudo apt-get install -y gcc-aarch64-linux-gnu
          echo "CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER=aarch64-linux-gnu-gcc" >> $GITHUB_ENV

      - name: Build
        run: cargo build --release --target ${{ matrix.target }}

      - name: Package
        shell: bash
        run: |
          VERSION="${GITHUB_REF_NAME#v}"
          cd target/${{ matrix.target }}/release
          tar czf ../../../{name}-${VERSION}-${{ matrix.target }}.tar.gz {name}
          cd ../../..

      - name: Upload artifact
        uses: actions/upload-artifact@v4
        with:
          name: {name}-${{ matrix.target }}
          path: "{name}-*.tar.gz"

  upload:
    needs: build
    runs-on: ubuntu-latest
    steps:
      - name: Download all artifacts
        uses: actions/download-artifact@v4
        with:
          merge-multiple: true

      - name: Upload assets to release
        uses: softprops/action-gh-release@v2
        with:
          tag_name: ${{ github.ref_name }}
          files: "{name}-*.tar.gz"
"#;

const RELEASE_PLZ_WORKFLOW_TEMPLATE: &str = r#"name: Release-plz

on:
  push:
    branches:
      - main

permissions:
  contents: write
  pull-requests: write

jobs:
  release-plz-release:
    name: Release-plz release
    runs-on: ubuntu-latest
    steps:
      - name: Checkout repository
        uses: actions/checkout@v4
        with:
          fetch-depth: 0
      - name: Install Rust toolchain
        uses: dtolnay/rust-toolchain@stable
      - name: Run release-plz
        uses: release-plz/action@v0.5
        with:
          command: release
        env:
          GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}

  release-plz-pr:
    name: Release-plz PR
    runs-on: ubuntu-latest
    concurrency:
      group: release-plz-${{ github.ref }}
      cancel-in-progress: false
    steps:
      - name: Checkout repository
        uses: actions/checkout@v4
        with:
          fetch-depth: 0
      - name: Install Rust toolchain
        uses: dtolnay/rust-toolchain@stable
      - name: Run release-plz
        uses: release-plz/action@v0.5
        with:
          command: release-pr
        env:
          GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
"#;

const RELEASE_PLZ_TOML_TEMPLATE: &str = r#"[package]
# Binary skills are distributed via GitHub Releases, not crates.io
publish = false

[changelog]
body = """
{% for group, commits in commits | group_by(attribute="group") %}
### {{ group | upper_first }}
{% for commit in commits %}
- {{ commit.message | upper_first }} ({{ commit.id | truncate(length=7, end="") }})\
{% endfor %}
{% endfor %}
"""
trim = true
commit_parsers = [
    { message = "^feat", group = "Added" },
    { message = "^fix", group = "Fixed" },
    { message = "^doc", group = "Documentation" },
    { message = "^perf", group = "Performance" },
    { message = "^refactor", group = "Refactored" },
    { message = "^test", group = "Testing" },
    { message = "^ci", group = "CI" },
    { message = "^build", group = "Build" },
    { message = "^chore", skip = true },
]
"#;

/// Write GitHub Actions CI/CD workflow files into a project directory.
///
/// Creates `.github/workflows/ci.yml`, `release.yml`, `release-plz.yml`,
/// and a top-level `release-plz.toml`.
pub fn setup_ci(project_dir: &Path, name: &str, force: bool) -> anyhow::Result<Vec<String>> {
    let workflows_dir = project_dir.join(".github/workflows");
    let files: &[(&str, &str, bool)] = &[
        (".github/workflows/ci.yml", CI_TEMPLATE, false),
        (".github/workflows/release.yml", RELEASE_TEMPLATE, true),
        (
            ".github/workflows/release-plz.yml",
            RELEASE_PLZ_WORKFLOW_TEMPLATE,
            false,
        ),
        ("release-plz.toml", RELEASE_PLZ_TOML_TEMPLATE, false),
    ];

    // Pre-check: error if any file exists and --force not set
    if !force {
        for &(rel_path, _, _) in files {
            let full_path = project_dir.join(rel_path);
            if full_path.exists() {
                anyhow::bail!("{rel_path} already exists. Use --force to overwrite.");
            }
        }
    }

    std::fs::create_dir_all(&workflows_dir)?;

    let mut created = Vec::new();
    for &(rel_path, template, needs_name) in files {
        let content = if needs_name {
            template.replace("{name}", name)
        } else {
            template.to_string()
        };
        std::fs::write(project_dir.join(rel_path), content)?;
        created.push(rel_path.to_string());
    }

    Ok(created)
}

/// Read the package name from a Cargo.toml file.
fn read_cargo_package_name(project_dir: &Path) -> anyhow::Result<String> {
    let cargo_toml_path = project_dir.join("Cargo.toml");
    if !cargo_toml_path.exists() {
        anyhow::bail!(
            "No Cargo.toml found in {}. Run this command from a Rust project directory, \
             or use `ion init --bin --ci` to scaffold a new binary skill project with CI.",
            project_dir.display()
        );
    }
    let content = std::fs::read_to_string(&cargo_toml_path)?;
    let doc: toml_edit::DocumentMut = content.parse()?;
    let name = doc
        .get("package")
        .and_then(|p| p.get("name"))
        .and_then(|n| n.as_str())
        .ok_or_else(|| anyhow::anyhow!("Cargo.toml is missing [package].name"))?;
    Ok(name.to_string())
}

/// Entry point for `ion ci` — set up CI/CD in the current directory.
pub fn run(force: bool, json: bool) -> anyhow::Result<()> {
    let project_dir = std::env::current_dir()?;
    let name = read_cargo_package_name(&project_dir)?;
    let created = setup_ci(&project_dir, &name, force)?;

    if json {
        crate::json::print_success(serde_json::json!({
            "name": name,
            "files": created,
        }));
        return Ok(());
    }

    println!("Created CI/CD workflows for '{name}':");
    for f in &created {
        println!("  {f}");
    }
    println!();
    println!("Setup:");
    println!("  1. Push to GitHub");
    println!("  2. CI runs automatically on push and PRs");
    println!("  3. release-plz creates version bump PRs on push to main");
    println!("  4. Merging a release PR tags and builds binaries for 4 targets");

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn setup_ci_creates_all_files() {
        let dir = tempfile::tempdir().unwrap();
        let created = setup_ci(dir.path(), "my-tool", false).unwrap();
        assert_eq!(created.len(), 4);
        assert!(dir.path().join(".github/workflows/ci.yml").exists());
        assert!(dir.path().join(".github/workflows/release.yml").exists());
        assert!(
            dir.path()
                .join(".github/workflows/release-plz.yml")
                .exists()
        );
        assert!(dir.path().join("release-plz.toml").exists());
    }

    #[test]
    fn setup_ci_substitutes_name_in_release() {
        let dir = tempfile::tempdir().unwrap();
        setup_ci(dir.path(), "my-tool", false).unwrap();
        let content =
            std::fs::read_to_string(dir.path().join(".github/workflows/release.yml")).unwrap();
        assert!(content.contains("my-tool-${VERSION}"));
        assert!(content.contains("my-tool-${{ matrix.target }}"));
        assert!(!content.contains("{name}"));
    }

    #[test]
    fn setup_ci_errors_without_force_if_exists() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join(".github/workflows")).unwrap();
        std::fs::write(dir.path().join(".github/workflows/ci.yml"), "existing").unwrap();
        let err = setup_ci(dir.path(), "my-tool", false).unwrap_err();
        assert!(err.to_string().contains("already exists"));
    }

    #[test]
    fn setup_ci_force_overwrites() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join(".github/workflows")).unwrap();
        std::fs::write(dir.path().join(".github/workflows/ci.yml"), "old").unwrap();
        let created = setup_ci(dir.path(), "my-tool", true).unwrap();
        assert_eq!(created.len(), 4);
        let content = std::fs::read_to_string(dir.path().join(".github/workflows/ci.yml")).unwrap();
        assert!(content.contains("cargo fmt"));
    }

    #[test]
    fn ci_template_has_no_name_placeholder() {
        assert!(!CI_TEMPLATE.contains("{name}"));
    }

    #[test]
    fn release_plz_workflow_has_no_name_placeholder() {
        assert!(!RELEASE_PLZ_WORKFLOW_TEMPLATE.contains("{name}"));
    }

    #[test]
    fn release_plz_toml_has_no_name_placeholder() {
        assert!(!RELEASE_PLZ_TOML_TEMPLATE.contains("{name}"));
    }
}
