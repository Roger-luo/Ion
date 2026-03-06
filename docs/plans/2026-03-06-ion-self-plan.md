# `ion self` Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add `ion self update`, `ion self check`, and `ion self info` commands that let ion update itself from GitHub Releases, plus a CI workflow to build and publish binaries.

**Architecture:** Reuse the existing `binary.rs` infrastructure (GitHub Releases API, platform detection, asset matching, download, extraction) in a new `src/commands/self_cmd.rs`. Add a `build.rs` to embed the build target triple at compile time. Create a GitHub Actions release workflow for 4 platform targets.

**Tech Stack:** Rust, clap, reqwest (already dep), flate2/tar (already dep), GitHub Actions

---

### Task 1: Add `build.rs` to embed target triple

**Files:**
- Create: `build.rs`

**Step 1: Create `build.rs`**

```rust
fn main() {
    println!(
        "cargo:rustc-env=TARGET={}",
        std::env::var("TARGET").unwrap()
    );
}
```

This makes `env!("TARGET")` available at compile time (e.g. `aarch64-apple-darwin`).

**Step 2: Verify it compiles**

Run: `cargo build`
Expected: Builds successfully. No test needed — we'll use it in Task 2.

**Step 3: Commit**

```bash
git add build.rs
git commit -m "build: add build.rs to embed TARGET triple at compile time"
```

---

### Task 2: Add `SelfCommands` enum and `ion self info`

**Files:**
- Modify: `src/main.rs`
- Create: `src/commands/self_cmd.rs`
- Modify: `src/commands/mod.rs`

**Step 1: Add `pub mod self_cmd;` to `src/commands/mod.rs`**

Add the line in alphabetical order among the existing `pub mod` declarations.

**Step 2: Add `SelfCommands` enum to `src/main.rs`**

Add to the `Commands` enum (note: `Self` is a Rust keyword so we use `#[command(name = "self")]`):

```rust
/// Manage the ion installation
#[command(name = "self")]
Self_ {
    #[command(subcommand)]
    action: SelfCommands,
},
```

Add the subcommand enum:

```rust
#[derive(Subcommand)]
enum SelfCommands {
    /// Check if a newer version is available
    Check,
    /// Show version, build target, and executable path
    Info,
    /// Update ion to the latest version
    Update {
        /// Install a specific version instead of latest
        #[arg(long)]
        version: Option<String>,
    },
}
```

Add dispatch in `main()`:

```rust
Commands::Self_ { action } => match action {
    SelfCommands::Check => commands::self_cmd::check(),
    SelfCommands::Info => commands::self_cmd::info(),
    SelfCommands::Update { version } => commands::self_cmd::update(version.as_deref()),
},
```

**Step 3: Implement `info()` in `src/commands/self_cmd.rs`**

```rust
use crate::style::Paint;
use crate::context::ProjectContext;

const REPO: &str = "Roger-luo/Ion";

pub fn info() -> anyhow::Result<()> {
    let version = env!("CARGO_PKG_VERSION");
    let target = env!("TARGET");
    let exe = std::env::current_exe()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|_| "(unknown)".to_string());

    println!("ion {version}");
    println!("target: {target}");
    println!("exe: {exe}");
    Ok(())
}
```

Add stub implementations for `check()` and `update()` that just print "not implemented yet":

```rust
pub fn check() -> anyhow::Result<()> {
    anyhow::bail!("not implemented yet")
}

pub fn update(_version: Option<&str>) -> anyhow::Result<()> {
    anyhow::bail!("not implemented yet")
}
```

**Step 4: Run tests**

Run: `cargo test`
Expected: All existing tests pass. The new command compiles.

**Step 5: Commit**

```bash
git add build.rs src/commands/self_cmd.rs src/commands/mod.rs src/main.rs
git commit -m "feat: add ion self info command with build target triple"
```

---

### Task 3: Implement `ion self check`

**Files:**
- Modify: `src/commands/self_cmd.rs`

**Step 1: Implement `check()`**

Replace the stub with:

```rust
pub fn check() -> anyhow::Result<()> {
    let current = env!("CARGO_PKG_VERSION");
    println!("Current: {current}");

    let release = ion_skill::binary::fetch_github_release(REPO, None)
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    let latest = ion_skill::binary::parse_version_from_tag(&release.tag_name);

    println!("Latest:  {latest}");

    if current == latest {
        println!("Already up to date.");
    } else {
        println!("Run `ion self update` to upgrade.");
    }
    Ok(())
}
```

**Step 2: Run tests**

Run: `cargo test`
Expected: All pass (no new tests needed — this hits the network).

**Step 3: Commit**

```bash
git add src/commands/self_cmd.rs
git commit -m "feat: implement ion self check"
```

---

### Task 4: Implement `ion self update`

**Files:**
- Modify: `src/commands/self_cmd.rs`

**Step 1: Add a `replace_exe` helper**

```rust
/// Replace the current running executable with a new binary.
/// Uses rename-based approach: current → current.old, new → current, delete old.
fn replace_exe(new_binary: &std::path::Path) -> anyhow::Result<std::path::PathBuf> {
    let current_exe = std::env::current_exe()?;
    // Resolve symlinks to get the real path
    let current_exe = std::fs::canonicalize(&current_exe)?;
    let backup = current_exe.with_extension("old");

    // Rename current → .old
    std::fs::rename(&current_exe, &backup)
        .map_err(|e| {
            if e.kind() == std::io::ErrorKind::PermissionDenied {
                anyhow::anyhow!(
                    "Permission denied replacing {}. Try: sudo ion self update",
                    current_exe.display()
                )
            } else {
                anyhow::anyhow!("Failed to back up current binary: {e}")
            }
        })?;

    // Copy new binary to current location
    if let Err(e) = std::fs::copy(new_binary, &current_exe) {
        // Restore backup on failure
        let _ = std::fs::rename(&backup, &current_exe);
        anyhow::bail!("Failed to install new binary: {e}");
    }

    // Set executable permissions
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&current_exe, std::fs::Permissions::from_mode(0o755))?;
    }

    // Clean up backup
    let _ = std::fs::remove_file(&backup);

    Ok(current_exe)
}
```

**Step 2: Implement `update()`**

Replace the stub:

```rust
pub fn update(version: Option<&str>) -> anyhow::Result<()> {
    let current = env!("CARGO_PKG_VERSION");

    // Fetch the target release
    let tag = version.map(|v| {
        if v.starts_with('v') { v.to_string() } else { format!("v{v}") }
    });
    let release = ion_skill::binary::fetch_github_release(REPO, tag.as_deref())
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    let new_version = ion_skill::binary::parse_version_from_tag(&release.tag_name);

    if version.is_none() && current == new_version {
        println!("Already up to date ({current}).");
        return Ok(());
    }

    println!("Updating ion {current} → {new_version}...");

    // Find matching platform asset
    let platform = ion_skill::binary::Platform::detect();
    let asset_names: Vec<String> = release.assets.iter().map(|a| a.name.clone()).collect();
    let asset_name = platform.match_asset("ion", &asset_names);

    let asset_name = match asset_name {
        Some(name) => name,
        None => {
            eprintln!("No pre-built binary found for {}.", platform.target_triple());
            eprintln!("You can update manually with:");
            eprintln!("  cargo install --git https://github.com/{REPO} --force");
            anyhow::bail!("No matching release asset for this platform");
        }
    };

    let asset = release.assets.iter()
        .find(|a| a.name == asset_name)
        .unwrap();

    // Download and extract
    let tmp_dir = tempfile::tempdir()?;
    let archive_path = tmp_dir.path().join(&asset_name);

    println!("  Downloading {asset_name}...");
    ion_skill::binary::download_file(&asset.browser_download_url, &archive_path)
        .map_err(|e| anyhow::anyhow!("{e}"))?;

    let extract_dir = tmp_dir.path().join("extracted");
    ion_skill::binary::extract_tar_gz(&archive_path, &extract_dir)
        .map_err(|e| anyhow::anyhow!("{e}"))?;

    let new_binary = ion_skill::binary::find_binary_in_dir(&extract_dir, "ion")
        .map_err(|e| anyhow::anyhow!("{e}"))?;

    // Replace current executable
    let installed_path = replace_exe(&new_binary)?;
    println!("  Installed to {}", installed_path.display());
    println!("Updated ion {current} → {new_version}");

    Ok(())
}
```

**Step 3: Run tests**

Run: `cargo test`
Expected: All pass. (Update itself is not unit-testable without mocking the network.)

**Step 4: Commit**

```bash
git add src/commands/self_cmd.rs
git commit -m "feat: implement ion self update with binary replacement"
```

---

### Task 5: Integration tests

**Files:**
- Modify: `tests/integration.rs`

**Step 1: Add help tests**

```rust
#[test]
fn self_info_shows_version_and_target() {
    let output = ion_cmd().args(["self", "info"]).output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(output.status.success());
    assert!(stdout.contains("ion "));
    assert!(stdout.contains("target:"));
    assert!(stdout.contains("exe:"));
}

#[test]
fn self_help_shows_subcommands() {
    let output = ion_cmd().args(["self", "--help"]).output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(output.status.success());
    assert!(stdout.contains("check"));
    assert!(stdout.contains("info"));
    assert!(stdout.contains("update"));
}
```

Note: We don't test `self check` or `self update` in integration tests because they hit the network (GitHub API). Those are verified manually.

**Step 2: Update `help_shows_all_commands` test**

Add `"self"` to the assertions in the existing `help_shows_all_commands` test.

**Step 3: Run tests**

Run: `cargo test`
Expected: All pass including new tests.

**Step 4: Commit**

```bash
git add tests/integration.rs
git commit -m "test: add integration tests for ion self commands"
```

---

### Task 6: GitHub Actions release workflow

**Files:**
- Create: `.github/workflows/release.yml`

**Step 1: Create the workflow file**

```yaml
name: Release

on:
  push:
    tags:
      - 'v*'

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
          cd target/${{ matrix.target }}/release
          tar czf ../../../ion-${GITHUB_REF_NAME#v}-${{ matrix.target }}.tar.gz ion
          cd ../../..

      - name: Upload artifact
        uses: actions/upload-artifact@v4
        with:
          name: ion-${{ matrix.target }}
          path: ion-*.tar.gz

  release:
    needs: build
    runs-on: ubuntu-latest
    steps:
      - name: Download all artifacts
        uses: actions/download-artifact@v4
        with:
          merge-multiple: true

      - name: Create GitHub Release
        uses: softprops/action-gh-release@v2
        with:
          files: ion-*.tar.gz
          generate_release_notes: true
```

**Step 2: Verify the file is valid YAML**

Run: `cat .github/workflows/release.yml`
Expected: Well-formed YAML, no syntax errors.

**Step 3: Commit**

```bash
git add .github/workflows/release.yml
git commit -m "ci: add GitHub Actions release workflow for 4 platform targets"
```

---

### Task 7: Final verification

**Step 1: Run full test suite**

Run: `cargo test`
Expected: All tests pass.

**Step 2: Run clippy**

Run: `cargo clippy`
Expected: No new warnings.

**Step 3: Manual smoke test**

Run: `cargo run -- self info`
Expected: Prints version, target triple, and executable path.

Run: `cargo run -- self check`
Expected: Prints current and latest version from GitHub (requires network).

**Step 4: Final commit if any fixups needed**

```bash
git add -A
git commit -m "fix: address any final issues from verification"
```
