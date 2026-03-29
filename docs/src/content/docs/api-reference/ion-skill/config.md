---
title: "ion-skill::config"
description: ""
order: 999
---

## GlobalConfig

### Fields

| Name | Type | Description |
|------|------|-------------|
| `targets` | `BTreeMap<String, String>` |  |
| `sources` | `BTreeMap<String, String>` |  |
| `cache` | `CacheConfig` |  |
| `ui` | `UiConfig` |  |
| `registries` | `BTreeMap<String, RegistryConfig>` |  |
| `search` | `SearchConfig` |  |

### Methods

#### `config_path`

```rust
pub fn config_path() -> Option<PathBuf>
```

Returns the platform-appropriate path for the global config file.

#### `load`

```rust
pub fn load() -> Result<Self>
```

Load global config from the platform default path.
Returns Default if the file doesn't exist.

#### `load_from`

```rust
pub fn load_from(path: &Path) -> Result<Self>
```

Load global config from a specific path.
Returns Default if the file doesn't exist.

#### `resolve_targets`

```rust
pub fn resolve_targets(&self, project: &ManifestOptions) -> BTreeMap<String, String>
```

Merge global targets with project targets. Project wins on key collision.

#### `resolve_source`

```rust
pub fn resolve_source(&self, input: &str) -> String
```

Expand source aliases. If the first segment of a shorthand matches a source
alias, replace it with the alias value. URLs and paths pass through unchanged.

#### `save_to`

```rust
pub fn save_to(&self, path: &Path) -> Result<()>
```

Save global config to a specific path. Creates parent directories.

#### `get_value`

```rust
pub fn get_value(&self, key: &str) -> Option<String>
```

Get a config value by dot-notation key (e.g., "targets.claude", "ui.color").

#### `set_value_in_file`

```rust
pub fn set_value_in_file(path: &Path, key: &str, value: &str) -> Result<()>
```

Set a config value in a TOML file by dot-notation key, preserving formatting.

#### `delete_value_in_file`

```rust
pub fn delete_value_in_file(path: &Path, key: &str) -> Result<()>
```

Delete a config value from a TOML file by dot-notation key, preserving formatting.

#### `list_values`

```rust
pub fn list_values(&self) -> Vec<(String, String)>
```

List all config values as a Vec of (dot-key, value) pairs.

### Trait Implementations

- `Debug`
- `Clone`
- `Default`
- `Serialize`
- `Deserialize<'de>`

---

## CacheConfig

### Fields

| Name | Type | Description |
|------|------|-------------|
| `max_age_days` | `Option<u32>` |  |

### Trait Implementations

- `Debug`
- `Clone`
- `Default`
- `Serialize`
- `Deserialize<'de>`

---

## UiConfig

### Fields

| Name | Type | Description |
|------|------|-------------|
| `color` | `Option<bool>` |  |

### Trait Implementations

- `Debug`
- `Clone`
- `Default`
- `Serialize`
- `Deserialize<'de>`

---

## RegistryConfig

### Fields

| Name | Type | Description |
|------|------|-------------|
| `url` | `String` |  |
| `default` | `Option<bool>` |  |

### Trait Implementations

- `Debug`
- `Clone`
- `Default`
- `Serialize`
- `Deserialize<'de>`

---

## SearchConfig

### Fields

| Name | Type | Description |
|------|------|-------------|
| `agent_command` | `Option<String>` |  |

### Trait Implementations

- `Debug`
- `Clone`
- `Default`
- `Serialize`
- `Deserialize<'de>`

