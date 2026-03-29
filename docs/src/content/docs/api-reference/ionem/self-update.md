---
title: "ionem::self_update"
description: "Reusable self-management infrastructure for binary skills."
order: 999
---

Reusable self-management infrastructure for binary skills.

Binary skills built for the Ion ecosystem are expected to implement a standard
`self` subcommand group:

- `<binary> self skill`   — print the embedded SKILL.md to stdout
- `<binary> self info`    — show version, build target, and executable path
- `<binary> self check`   — check if a newer version is available
- `<binary> self update`  — download and install a newer version

This module provides [`SelfManager`] which implements the core logic for
`info`, `check`, and `update`. Downstream binary skills configure it with
their GitHub repo, binary name, and tag prefix, then delegate their `self`
subcommands to it.

# Example

```rust,ignore
use ionem::self_update::SelfManager;

let manager = SelfManager::new(
    "owner/my-tool",          // GitHub repo
    "my-tool",                // binary name in release assets
    "v",                      // tag prefix (e.g. "v1.0.0")
    env!("CARGO_PKG_VERSION"),
    env!("TARGET"),
);

// In your clap match:
// SelfCommands::Skill => print!(include_str!("../SKILL.md")),
// SelfCommands::Info  => manager.print_info(),
// SelfCommands::Check => manager.print_check()?,
// SelfCommands::Update { version } => manager.run_update(version.as_deref())?,
```

## SelfManager

Configuration and executor for the standard binary skill `self` subcommands.

### Fields

| Name | Type | Description |
|------|------|-------------|
| `repo` | `String` | GitHub repository in `owner/repo` format. |
| `binary_name` | `String` | The binary executable name (used for asset matching). |
| `tag_prefix` | `String` | Tag prefix for release tags (e.g. `"v"` for `v1.0.0`, `"my-tool-v"` for `my-tool-v1.0.0`). |
| `current_version` | `String` | Current version of this binary (typically `env!("CARGO_PKG_VERSION")`). |
| `target` | `String` | Build target triple (typically `env!("TARGET")`, set via `build.rs`). |

### Methods

#### `new`

```rust
pub fn new(repo: &str, binary_name: &str, tag_prefix: &str, current_version: &str, target: &str) -> Self
```

Create a new `SelfManager` with the given configuration.

# Arguments

* `repo` — GitHub repository in `owner/repo` format
* `binary_name` — executable name in release assets
* `tag_prefix` — prefix before the version in git tags (e.g. `"v"`, `"my-tool-v"`)
* `current_version` — the running binary's version (e.g. `env!("CARGO_PKG_VERSION")`)
* `target` — build target triple (e.g. `env!("TARGET")`)

#### `info`

```rust
pub fn info(&self) -> SelfInfo
```

Return information about the current binary.

#### `print_info`

```rust
pub fn print_info(&self)
```

Print self info to stdout.

#### `check`

```rust
pub fn check(&self) -> Result<CheckResult>
```

Check whether a newer version is available on GitHub Releases.

#### `print_check`

```rust
pub fn print_check(&self) -> Result<()>
```

Print check result to stdout.

#### `update`

```rust
pub fn update(&self, version: Option<&str>) -> Result<UpdateResult>
```

Download and install a newer version, replacing the current executable.

If `version` is `None`, fetches the latest release. If `version` is `Some`,
fetches the release tagged `{tag_prefix}{version}`.

#### `run_update`

```rust
pub fn run_update(&self, version: Option<&str>) -> Result<()>
```

Run update and print progress to stdout.

---

## CheckResult

Result of checking for updates.

### Fields

| Name | Type | Description |
|------|------|-------------|
| `installed` | `String` |  |
| `latest` | `String` |  |
| `update_available` | `bool` |  |

### Trait Implementations

- `Debug`

---

## UpdateResult

Result of performing an update.

### Fields

| Name | Type | Description |
|------|------|-------------|
| `updated` | `bool` |  |
| `old_version` | `String` |  |
| `new_version` | `String` |  |
| `exe` | `PathBuf` |  |

### Trait Implementations

- `Debug`

---

## SelfInfo

Information about the current binary.

### Fields

| Name | Type | Description |
|------|------|-------------|
| `version` | `String` |  |
| `target` | `String` |  |
| `exe` | `PathBuf` |  |

### Trait Implementations

- `Debug`

---

## replace_exe

```rust
pub fn replace_exe(new_binary: &Path) -> Result<PathBuf>
```

Replace the current running executable with a new binary.

Uses a backup-copy-cleanup strategy for atomic replacement.

---

## is_newer_version

```rust
pub fn is_newer_version(current: &str, latest: &str) -> bool
```

Returns true if `latest` is strictly newer than `current`.

