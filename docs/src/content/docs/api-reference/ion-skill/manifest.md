---
title: "ion-skill::manifest"
description: "Ion.toml types — project skill configuration with targets, skill entries, and per-project options."
order: 999
---

## ManifestOptions

### Fields

| Name | Type | Description |
|------|------|-------------|
| `targets` | `BTreeMap<String, String>` |  |
| `skills_dir` | `Option<String>` |  |

### Methods

#### `get_value`

```rust
pub fn get_value(&self, key: &str) -> Option<String>
```

Get a project config value by key. Supports dot-notation for targets
and top-level keys like "skills-dir".

#### `skills_dir_or_default`

```rust
pub fn skills_dir_or_default(&self) -> &str
```

Returns the configured skills directory, or the default `.agents/skills`.

#### `list_values`

```rust
pub fn list_values(&self) -> Vec<(String, String)>
```

List all project config values as (key, value) pairs.

### Trait Implementations

- `Debug`
- `Clone`
- `Default`
- `Serialize`
- `Deserialize<'de>`

---

## ProjectMeta

Metadata about the project itself (not its dependencies).

Present in `[project]` section of Ion.toml for projects that are themselves
skills (e.g. binary skill projects). Optional — most Ion.toml files only
have `[skills]` and `[options]`.

### Fields

| Name | Type | Description |
|------|------|-------------|
| `project_type` | `Option<String>` | The project type: "binary" for binary skill projects. |
| `binary` | `Option<String>` | Override the binary executable name (defaults to Cargo.toml package name). |

### Methods

#### `is_binary`

```rust
pub fn is_binary(&self) -> bool
```

Returns true if this project declares itself as a binary skill.

### Trait Implementations

- `Debug`
- `Clone`
- `Default`
- `Serialize`
- `Deserialize<'de>`

---

## Manifest

### Fields

| Name | Type | Description |
|------|------|-------------|
| `project` | `Option<ProjectMeta>` |  |
| `skills` | `BTreeMap<String, SkillEntry>` |  |
| `options` | `ManifestOptions` |  |
| `agents` | `Option<AgentsConfig>` |  |

### Methods

#### `from_file`

```rust
pub fn from_file(path: &Path) -> Result<Self>
```

#### `parse`

```rust
pub fn parse(content: &str) -> Result<Self>
```

#### `resolve_entry`

```rust
pub fn resolve_entry(entry: &SkillEntry) -> Result<SkillSource>
```

Resolve a manifest entry into a SkillSource.

Prefer calling `entry.resolve()` directly. This static method is kept
for backward compatibility.

#### `empty`

```rust
pub fn empty() -> Self
```

### Trait Implementations

- `Debug`
- `Clone`
- `Serialize`
- `Deserialize<'de>`

---

## SkillEntry

### Variants

- **`Shorthand(String)`**
- **`Full { source_type: Option<SourceType>, source: Option<String>, version: Option<String>, rev: Option<String>, path: Option<String>, binary: Option<String>, asset_pattern: Option<String>, forked_from: Option<String>, dev: Option<bool> }`**

### Methods

#### `resolve`

```rust
pub fn resolve(&self) -> Result<SkillSource>
```

Resolve this manifest entry into a fully qualified SkillSource.

### Trait Implementations

- `Debug`
- `Clone`
- `Serialize`
- `Deserialize<'de>`

---

## read_project_meta

```rust
pub fn read_project_meta(path: &Path) -> Option<ProjectMeta>
```

Read just the `[project]` section from an Ion.toml file, if present.

Returns `None` if the file doesn't exist, can't be parsed, or has no
`[project]` section. This is intentionally lenient — it's used for
auto-detection, not validation.

