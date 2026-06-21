---
title: "ion-skill::source"
description: "Skill source abstraction — represent and resolve GitHub, Git, HTTP, path, binary, and local skill origins."
order: 999
---

Skill source abstraction — represent and resolve GitHub, Git, HTTP, path, binary, and local skill origins.

## SkillSource

A fully resolved skill source.

### Fields

| Name | Type | Description |
|------|------|-------------|
| `source` | `String` | The raw source string (URL, path, or owner/repo shorthand). |
| `path` | `Option<String>` | Subdirectory path within the source (for multi-skill repos). |
| `rev` | `Option<String>` | Pinned revision (git commit, tag, or branch). |
| `version` | `Option<String>` | Required SKILL.md version. |
| `kind` | `SkillSourceKind` | Source-type-specific data. |

### Methods

#### `github`

```rust
pub fn github(source: impl Into<String>) -> Self
```

#### `git`

```rust
pub fn git(source: impl Into<String>) -> Self
```

#### `http`

```rust
pub fn http(source: impl Into<String>) -> Self
```

#### `path`

```rust
pub fn path(source: impl Into<String>) -> Self
```

#### `local`

```rust
pub fn local() -> Self
```

#### `binary`

```rust
pub fn binary(source: impl Into<String>, binary_name: impl Into<String>) -> Self
```

#### `new`

```rust
pub fn new(source_type: SourceType, source: impl Into<String>) -> Self
```

Compatibility: convert SourceType to SkillSourceKind.

Used by `manifest.rs::SkillEntry::resolve()`. Will be removed once
callers migrate to the named constructors.

#### `from_path`

```rust
pub fn from_path(source: &str) -> Self
```

Create a path-based skill source.

Alias for `Self::path(source)`. Kept for backward compatibility.

#### `with_rev`

```rust
pub fn with_rev(self, rev: impl Into<String>) -> Self
```

#### `with_path`

```rust
pub fn with_path(self, path: impl Into<String>) -> Self
```

#### `with_version`

```rust
pub fn with_version(self, version: impl Into<String>) -> Self
```

#### `with_binary`

```rust
pub fn with_binary(self, binary: impl Into<String>) -> Self
```

#### `with_asset_pattern`

```rust
pub fn with_asset_pattern(self, pattern: impl Into<String>) -> Self
```

#### `with_forked_from`

```rust
pub fn with_forked_from(self, forked_from: impl Into<String>) -> Self
```

#### `with_dev`

```rust
pub fn with_dev(self, dev_mode: bool) -> Self
```

#### `is_github`

```rust
pub fn is_github(&self) -> bool
```

#### `is_git_based`

```rust
pub fn is_git_based(&self) -> bool
```

#### `is_binary`

```rust
pub fn is_binary(&self) -> bool
```

#### `is_local`

```rust
pub fn is_local(&self) -> bool
```

#### `is_path`

```rust
pub fn is_path(&self) -> bool
```

#### `is_http`

```rust
pub fn is_http(&self) -> bool
```

#### `is_local_path`

```rust
pub fn is_local_path(&self) -> bool
```

Returns true if this source points to a local filesystem path
(either a Path source or a Binary source with a local project).

#### `is_remote_installable`

```rust
pub fn is_remote_installable(&self) -> bool
```

True for sources that need gitignore entries (not Path or Local).

#### `display_name`

```rust
pub fn display_name(&self) -> String
```

Derive a human-readable skill name from this source.
Uses the path's last segment if available, otherwise the source's last segment.

#### `infer`

```rust
pub fn infer(source: &str) -> Result<Self>
```

Infer a SkillSource from a raw source string (no explicit type).

#### `http_skill_url`

```rust
pub fn http_skill_url(&self) -> Result<String>
```

Return the URL to fetch the SKILL.md file from an HTTP source.
If the URL doesn't already end with `skill.md` (case-insensitive),
appends `/skill.md`.

#### `git_url`

```rust
pub fn git_url(&self) -> Result<String>
```

Build a git clone URL for this source.

### Trait Implementations

- `Debug`
- `Clone`
- `PartialEq`
- `Eq`

---

## SourceType

The type of source a skill is fetched from.

Kept for Ion.toml serde deserialization (used by `SkillEntry::Full` in manifest.rs).

### Variants

- **`Github`**
- **`Git`**
- **`Http`**
- **`Path`**
- **`Binary`**
- **`Local`**

### Trait Implementations

- `Debug`
- `Clone`
- `PartialEq`
- `Eq`
- `Serialize`
- `Deserialize<'de>`

---

## SkillSourceKind

Per-source-type data that only makes sense for that variant.

### Variants

- **`Github`**
- **`Git`**
- **`Http`**
- **`Path`**
- **`Binary { binary_name: String, asset_pattern: Option<String>, local_project: Option<PathBuf>, dev: bool }`**
- **`Local { forked_from: Option<String> }`**

### Trait Implementations

- `Debug`
- `Clone`
- `PartialEq`
- `Eq`

