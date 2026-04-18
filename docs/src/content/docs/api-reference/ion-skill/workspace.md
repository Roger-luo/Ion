---
title: "ion-skill::workspace"
description: "Project workspace context — load manifest and lockfile, resolve effective options and skill paths for a project."
order: 999
---

## Project

A single project within a workspace (or the root project itself).

### Fields

| Name | Type | Description |
|------|------|-------------|
| `dir` | `PathBuf` |  |
| `manifest_path` | `PathBuf` |  |
| `lockfile_path` | `PathBuf` |  |

### Methods

#### `new`

```rust
pub fn new(dir: PathBuf) -> Self
```

#### `has_manifest`

```rust
pub fn has_manifest(&self) -> bool
```

#### `manifest`

```rust
pub fn manifest(&self) -> Result<Manifest>
```

#### `manifest_or_empty`

```rust
pub fn manifest_or_empty(&self) -> Result<Manifest>
```

#### `lockfile`

```rust
pub fn lockfile(&self) -> Result<Lockfile>
```

#### `effective_options`

```rust
pub fn effective_options(&self, inherited: &ManifestOptions) -> Result<ManifestOptions>
```

Compute effective options by merging inherited options with this project's local options.
`inherited` comes from the workspace root; local options override inherited ones.

### Trait Implementations

- `Debug`

