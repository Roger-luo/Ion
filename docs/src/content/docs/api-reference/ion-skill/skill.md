---
title: "ion-skill::skill"
description: "SKILL.md parsing — read and validate skill metadata from frontmatter."
order: 999
---

SKILL.md parsing — read and validate skill metadata from frontmatter.

## SkillMetadata

Parsed SKILL.md frontmatter.

### Fields

| Name | Type | Description |
|------|------|-------------|
| `name` | `String` |  |
| `description` | `String` |  |
| `license` | `Option<String>` |  |
| `compatibility` | `Option<String>` |  |
| `metadata` | `Option<HashMap<String, String>>` |  |
| `allowed_tools` | `Option<String>` |  |

### Methods

#### `parse`

```rust
pub fn parse(content: &str) -> Result<(Self, String)>
```

Parse SKILL.md content (frontmatter + body).

#### `from_file`

```rust
pub fn from_file(path: &Path) -> Result<(Self, String)>
```

Parse SKILL.md from a file path.

#### `version`

```rust
pub fn version(&self) -> Option<&str>
```

Get the version from metadata, if present.

#### `validate_name`

```rust
pub fn validate_name(name: &str) -> Result<()>
```

Validate the skill name against the spec rules.

### Trait Implementations

- `Debug`
- `Clone`
- `Deserialize<'de>`

