---
title: "ion-skill::lockfile"
description: ""
order: 999
---

## LockedSkill

### Fields

| Name | Type | Description |
|------|------|-------------|
| `name` | `String` |  |
| `source` | `String` |  |
| `path` | `Option<String>` |  |
| `version` | `Option<String>` |  |
| `kind` | `LockedSkillKind` |  |

### Methods

#### `git`

```rust
pub fn git(name: impl Into<String>, source: impl Into<String>, commit: String, checksum: String) -> Self
```

#### `binary`

```rust
pub fn binary(name: impl Into<String>, source: impl Into<String>, binary_name: impl Into<String>, binary_version: Option<String>, binary_checksum: Option<String>) -> Self
```

#### `local`

```rust
pub fn local(name: impl Into<String>) -> Self
```

#### `http`

```rust
pub fn http(name: impl Into<String>, source: impl Into<String>) -> Self
```

#### `path`

```rust
pub fn path(name: impl Into<String>, source: impl Into<String>) -> Self
```

#### `with_path`

```rust
pub fn with_path(self, path: impl Into<String>) -> Self
```

#### `with_version`

```rust
pub fn with_version(self, version: impl Into<String>) -> Self
```

#### `with_source`

```rust
pub fn with_source(self, source: impl Into<String>) -> Self
```

#### `with_checksum`

```rust
pub fn with_checksum(self, checksum: impl Into<String>) -> Self
```

#### `with_dev`

```rust
pub fn with_dev(self) -> Self
```

#### `is_binary`

```rust
pub fn is_binary(&self) -> bool
```

#### `is_dev`

```rust
pub fn is_dev(&self) -> bool
```

#### `binary_name`

```rust
pub fn binary_name(&self) -> Option<&str>
```

#### `binary_version`

```rust
pub fn binary_version(&self) -> Option<&str>
```

#### `commit`

```rust
pub fn commit(&self) -> Option<&str>
```

#### `checksum`

```rust
pub fn checksum(&self) -> Option<&str>
```

### Trait Implementations

- `Debug`
- `Clone`
- `PartialEq`
- `Eq`

---

## Lockfile

### Fields

| Name | Type | Description |
|------|------|-------------|
| `skills` | `Vec<LockedSkill>` |  |
| `agents` | `Option<AgentsLockEntry>` |  |

### Methods

#### `from_file`

```rust
pub fn from_file(path: &Path) -> Result<Self>
```

#### `write_to`

```rust
pub fn write_to(&self, path: &Path) -> Result<()>
```

#### `find`

```rust
pub fn find(&self, name: &str) -> Option<&LockedSkill>
```

#### `upsert`

```rust
pub fn upsert(&mut self, skill: LockedSkill)
```

#### `remove`

```rust
pub fn remove(&mut self, name: &str)
```

### Trait Implementations

- `Debug`
- `Clone`
- `Default`

---

## LockedSkillKind

### Variants

- **`Git { commit: String, checksum: String }`**
- **`Binary { binary_name: String, binary_version: Option<String>, binary_checksum: Option<String>, dev: bool }`**
- **`Local { checksum: Option<String> }`**
- **`Http { checksum: Option<String> }`**
- **`Path { checksum: Option<String> }`**

### Trait Implementations

- `Debug`
- `Clone`
- `PartialEq`
- `Eq`

