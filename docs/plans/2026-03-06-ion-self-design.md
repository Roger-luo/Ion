# `ion self` Design

## Problem

Ion has no way to update itself. Users must manually re-run `cargo install --git` to get new versions. There's also no CI pipeline to build and publish pre-built binaries.

## Solution

Add an `ion self` subcommand group with three commands (`update`, `check`, `info`) and a GitHub Actions release workflow that publishes pre-built binaries for macOS and Linux.

## Subcommands

### `ion self update [--version X.Y.Z]`

1. Get current version from `env!("CARGO_PKG_VERSION")`
2. Fetch latest (or specified) release from `Roger-luo/Ion` GitHub Releases
3. If already up to date, say so and exit
4. Detect platform via existing `binary::Platform::detect()`
5. Match a release asset using naming convention `ion-{version}-{target}.tar.gz`
6. If no matching asset: suggest `cargo install --git https://github.com/Roger-luo/Ion --force` as fallback
7. Download to temp file in same directory as current exe (ensures same filesystem for rename)
8. Extract binary from archive
9. Replace current executable via atomic rename:
   - Rename current binary to `ion.old`
   - Rename new binary to original path
   - Delete `ion.old`
10. On permission error: tell user to retry with `sudo` or show temp file path for manual copy
11. Print old → new version

### `ion self check`

Same version-fetch logic as update, but only prints the comparison:
```
Current: 0.1.21
Latest:  0.1.22
Run `ion self update` to upgrade.
```
Or: `Already up to date (0.1.22)`.

### `ion self info`

```
ion 0.1.22
target: aarch64-apple-darwin
exe: /Users/roger/.cargo/bin/ion
```

Uses `env!("CARGO_PKG_VERSION")`, build-time target triple, and `std::env::current_exe()`.

## Implementation

### Reuse `binary.rs`

The existing `crates/ion-skill/src/binary.rs` provides:
- `fetch_github_release(repo, tag)` — fetch release metadata
- `Platform::detect()` — detect OS + arch
- `Platform::match_asset()` — find correct asset for platform
- `download_file(url, dest)` — HTTP download
- `extract_tar_gz(archive, dest)` — archive extraction
- `file_checksum()` — SHA256 verification

New addition: a `replace_current_exe(new_binary_path)` helper for the atomic rename.

### New files

- `src/commands/self_cmd.rs` — command implementations
- `.github/workflows/release.yml` — CI release pipeline

### `main.rs` changes

Add `SelfCommands` enum:
```rust
#[derive(Subcommand)]
enum SelfCommands {
    Update { #[arg(long)] version: Option<String> },
    Check,
    Info,
}
```

Note: the subcommand group is named `Self_` or similar in the enum since `Self` is a Rust keyword, but clap renders it as `ion self` via `#[command(name = "self")]`.

### Build-time target triple

Add a `build.rs` that sets `TARGET` env var:
```rust
fn main() {
    println!("cargo:rustc-env=TARGET={}", std::env::var("TARGET").unwrap());
}
```

Then use `env!("TARGET")` in `self info`.

## CI Release Workflow

`.github/workflows/release.yml` triggered on tag push (`v*`).

### Build matrix

| Target | OS |
|--------|-----|
| `aarch64-apple-darwin` | macOS ARM |
| `x86_64-apple-darwin` | macOS Intel |
| `x86_64-unknown-linux-gnu` | Linux x86_64 |
| `aarch64-unknown-linux-gnu` | Linux ARM |

### Asset naming

`ion-{version}-{target}.tar.gz` — e.g. `ion-0.1.22-aarch64-apple-darwin.tar.gz`

Each archive contains the `ion` binary.

### Flow

1. Tag push triggers workflow
2. Build binary for each target (cross-compilation for Linux ARM)
3. Create tar.gz archive per target
4. Create GitHub Release from the tag
5. Upload all 4 archives as release assets

## Constants

- Repository: `Roger-luo/Ion` (hardcoded)
- Current version: `env!("CARGO_PKG_VERSION")`
- Build target: `env!("TARGET")`

## Error handling

| Scenario | Behavior |
|----------|----------|
| No matching asset | Suggest `cargo install --git` fallback |
| Permission denied on replace | Tell user to use `sudo` or show temp path |
| Network error | Clear error message |
| Already up to date | Print message and exit 0 |
| Specified version not found | Error with available versions hint |
