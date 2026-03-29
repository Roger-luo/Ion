---
title: "ionem::release"
description: "GitHub release fetching and platform detection for binary skills."
order: 999
---

GitHub release fetching and platform detection for binary skills.

## Platform

Detected platform info for binary downloads.

### Fields

| Name | Type | Description |
|------|------|-------------|
| `os` | `String` |  |
| `arch` | `String` |  |

### Methods

#### `detect`

```rust
pub fn detect() -> Self
```

#### `target_triple`

```rust
pub fn target_triple(&self) -> String
```

#### `match_asset`

```rust
pub fn match_asset(&self, binary_name: &str, asset_names: &[String]) -> Option<String>
```

Match a binary name against release asset names, returning the best match.

### Trait Implementations

- `Debug`
- `Clone`

---

## GitHubRelease

### Fields

| Name | Type | Description |
|------|------|-------------|
| `tag_name` | `String` |  |
| `assets` | `Vec<GitHubAsset>` |  |

### Trait Implementations

- `Debug`
- `Deserialize<'de>`

---

## GitHubAsset

### Fields

| Name | Type | Description |
|------|------|-------------|
| `name` | `String` |  |
| `browser_download_url` | `String` |  |

### Trait Implementations

- `Debug`
- `Deserialize<'de>`

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

Fetch the latest release whose tag starts with the given prefix.

Useful when a repo has multiple crates releasing independently
(e.g. `ion-v*` vs `ion-skill-v*`).

---

## parse_version_from_tag

```rust
pub fn parse_version_from_tag(tag: &str) -> &str
```

---

## download_file

```rust
pub fn download_file(url: &str, dest: &Path) -> Result<()>
```

Download a file from URL to a local path.

---

## extract_tar_gz

```rust
pub fn extract_tar_gz(archive_path: &Path, dest_dir: &Path) -> Result<Vec<PathBuf>>
```

Extract a .tar.gz archive to a destination directory.

---

## find_binary_in_dir

```rust
pub fn find_binary_in_dir(dir: &Path, binary_name: &str) -> Result<PathBuf>
```

Find the binary executable in an extracted directory.

