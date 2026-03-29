---
title: "scenario::manifest"
description: "Parsing for `template.toml` manifest files."
order: 999
---

Parsing for `template.toml` manifest files.

## TemplateManifest

A parsed `template.toml` manifest.

### Fields

| Name | Type | Description |
|------|------|-------------|
| `variables` | `HashMap<String, VariableDecl>` | Declared template variables. |
| `files` | `FilesConfig` | File configuration: optional files, mappings, symlinks. |

### Methods

#### `from_dir`

```rust
pub fn from_dir(dir: &Path) -> Result<Self, Error>
```

Parse `template.toml` from the given template directory.

### Trait Implementations

- `Debug`
- `Clone`
- `Deserialize<'de>`
- `Default`

---

## VariableDecl

Declaration of a single template variable.

### Fields

| Name | Type | Description |
|------|------|-------------|
| `description` | `Option<String>` | Human-readable description of the variable. |
| `default` | `Option<String>` | Default value. If `None`, the variable is required. |

### Trait Implementations

- `Debug`
- `Clone`
- `Deserialize<'de>`
- `Default`

---

## FilesConfig

File-related configuration from `template.toml`.

### Fields

| Name | Type | Description |
|------|------|-------------|
| `optional` | `Vec<String>` | Glob patterns for files excluded by default (opt-in via `.include()`). |
| `mappings` | `HashMap<String, String>` | Source path (in template dir) → destination path (rendered). |
| `symlinks` | `HashMap<String, String>` | Symlink path (rendered) → symlink target (rendered). |

### Trait Implementations

- `Debug`
- `Clone`
- `Deserialize<'de>`
- `Default`

