---
title: "ion-skill::binary"
description: ""
order: 999
---

## BinaryValidation

Result of validating an installed binary.

### Fields

| Name | Type | Description |
|------|------|-------------|
| `is_executable` | `bool` |  |
| `version_output` | `Option<String>` |  |
| `has_skill_command` | `bool` |  |

### Trait Implementations

- `Debug`

---

## BinaryInstallResult

### Fields

| Name | Type | Description |
|------|------|-------------|
| `version` | `String` |  |
| `binary_checksum` | `String` |  |
| `warnings` | `Vec<String>` |  |

### Trait Implementations

- `Debug`

---

## CargoProject

Information extracted from a local Cargo project.

### Fields

| Name | Type | Description |
|------|------|-------------|
| `binary_name` | `String` |  |
| `version` | `String` |  |

### Trait Implementations

- `Debug`

---

## fetch_github_release

```rust
pub fn fetch_github_release(repo: &str, tag: Option<&str>) -> Result<GitHubRelease>
```

---

## fetch_latest_release_by_tag_prefix

```rust
pub fn fetch_latest_release_by_tag_prefix(repo: &str, prefix: &str) -> Result<GitHubRelease>
```

---

## download_file

```rust
pub fn download_file(url: &str, dest: &Path) -> Result<()>
```

---

## extract_tar_gz

```rust
pub fn extract_tar_gz(archive_path: &Path, dest_dir: &Path) -> Result<Vec<PathBuf>>
```

---

## find_binary_in_dir

```rust
pub fn find_binary_in_dir(dir: &Path, binary_name: &str) -> Result<PathBuf>
```

---

## bin_dir

```rust
pub fn bin_dir() -> PathBuf
```

---

## binary_path

```rust
pub fn binary_path(name: &str, version: &str) -> PathBuf
```

---

## install_binary_file

```rust
pub fn install_binary_file(binary_file: &Path, name: &str, version: &str, bin_root: &Path) -> Result<()>
```

Install a binary file to versioned storage.
Creates: {bin_root}/{name}/{version}/{name} and {bin_root}/{name}/current -> {version}

---

## file_checksum

```rust
pub fn file_checksum(path: &Path) -> Result<String>
```

---

## generate_skill_md

```rust
pub fn generate_skill_md(binary_path: &Path) -> Result<String>
```

Run `<binary> self skill` and capture the SKILL.md output from stdout.

---

## find_bundled_skill_md

```rust
pub fn find_bundled_skill_md(extract_dir: &Path) -> Option<PathBuf>
```

Look for a SKILL.md in an extracted archive directory.

---

## validate_binary

```rust
pub fn validate_binary(binary_path: &Path) -> Result<BinaryValidation>
```

Validate that an installed binary is functional.

Checks executable permissions, tries `--version`, and checks for a `self skill` subcommand.
Returns an error only if the binary does not exist; other checks are best-effort.

---

## is_binary_installed

```rust
pub fn is_binary_installed(name: &str, version: &str) -> bool
```

Check if a binary is already installed at the given version.

---

## install_binary_from_github

```rust
pub fn install_binary_from_github(repo: &str, binary_name: &str, rev: Option<&str>, skill_dir: &Path, asset_pattern: Option<&str>) -> Result<BinaryInstallResult>
```

Full binary skill installation from GitHub Releases.

---

## expand_url_template

```rust
pub fn expand_url_template(template: &str, binary_name: &str, version: &str) -> String
```

Expand placeholders in a URL template using detected platform info.

Supported placeholders: `{version}`, `{target}`, `{os}`, `{arch}`, `{binary}`.

---

## install_binary_from_url

```rust
pub fn install_binary_from_url(url_template: &str, binary_name: &str, version: &str, skill_dir: &Path) -> Result<BinaryInstallResult>
```

Install a binary from a generic URL template.

The `url_template` may contain placeholders expanded by [`expand_url_template`].
The archive is expected to be a `.tar.gz` file.

---

## remove_binary_version

```rust
pub fn remove_binary_version(name: &str, version: &str) -> Result<()>
```

Remove a specific version of a binary from storage.

---

## remove_binary

```rust
pub fn remove_binary(name: &str) -> Result<()>
```

Remove all versions of a binary and its parent directory.

---

## list_installed_binaries

```rust
pub fn list_installed_binaries() -> Result<Vec<String>>
```

List all installed binary skill names (directory names under bin_dir).

---

## cargo_project_info

```rust
pub fn cargo_project_info(project_path: &Path) -> Result<CargoProject>
```

Run `cargo metadata` to extract binary name and version from a Cargo project.

Finds the first `[[bin]]` target in the package. Errors if no binary target is found
or if the path doesn't contain a valid Cargo project.

---

## install_binary_from_local

```rust
pub fn install_binary_from_local(project_path: &Path, binary_name: &str, skill_dir: &Path) -> Result<BinaryInstallResult>
```

Build a local Cargo project in release mode and install the binary.

---

## setup_dev_binary

```rust
pub fn setup_dev_binary(project_path: &Path, binary_name: &str, skill_dir: &Path) -> Result<CargoProject>
```

Set up a dev-mode local binary skill (no release build, just metadata + SKILL.md).

Validates the project is a valid Cargo binary crate and generates the SKILL.md.
The actual binary is not built — `ion run` will forward to `cargo run` at runtime.

