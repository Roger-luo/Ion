---
title: "ion-skill::registry"
description: "Global registry of skill repositories — tracks which projects use which remote repos and cleans up stale entries."
order: 999
---

## RepoEntry

### Fields

| Name | Type | Description |
|------|------|-------------|
| `url` | `String` |  |
| `projects` | `Vec<String>` |  |

### Trait Implementations

- `Debug`
- `Clone`
- `Default`
- `Serialize`
- `Deserialize<'de>`

---

## Registry

### Fields

| Name | Type | Description |
|------|------|-------------|
| `repos` | `BTreeMap<String, RepoEntry>` |  |

### Methods

#### `registry_path`

```rust
pub fn registry_path() -> Option<PathBuf>
```

Returns the path to the global registry file.

#### `load`

```rust
pub fn load() -> Result<Self>
```

Load the global registry. Returns empty registry if file doesn't exist.

#### `load_from`

```rust
pub fn load_from(path: &Path) -> Result<Self>
```

Load registry from a specific path.

#### `save`

```rust
pub fn save(&self) -> Result<()>
```

Save registry to the default path.

#### `save_to`

```rust
pub fn save_to(&self, path: &Path) -> Result<()>
```

Save registry to a specific path.

#### `register`

```rust
pub fn register(&mut self, repo_hash: &str, url: &str, project_dir: &str)
```

Register that a project uses a specific repo.

#### `unregister`

```rust
pub fn unregister(&mut self, repo_hash: &str, project_dir: &str)
```

Unregister a project from a specific repo.

#### `cleanup_stale`

```rust
pub fn cleanup_stale(&mut self) -> Vec<(String, String)>
```

Remove repos with no remaining projects. Returns list of removed repo hashes.

### Trait Implementations

- `Debug`
- `Clone`
- `Default`
- `Serialize`
- `Deserialize<'de>`

