---
title: "ion-skill::migrate"
description: "Migrate skills from legacy Ion formats to the current manifest and lockfile layout."
order: 999
---

## SkillsLockFile

### Fields

| Name | Type | Description |
|------|------|-------------|
| `version` | `u32` |  |
| `skills` | `BTreeMap<String, SkillsLockEntry>` |  |

### Trait Implementations

- `Debug`
- `Deserialize<'de>`

---

## SkillsLockEntry

### Fields

| Name | Type | Description |
|------|------|-------------|
| `source` | `String` |  |
| `source_type` | `String` |  |
| `computed_hash` | `String` |  |

### Trait Implementations

- `Debug`
- `Deserialize<'de>`

---

## DiscoveredSkill

### Fields

| Name | Type | Description |
|------|------|-------------|
| `name` | `String` |  |
| `source` | `Option<SkillSource>` |  |
| `version` | `Option<String>` |  |
| `installed_path` | `PathBuf` |  |
| `origin` | `DiscoveryOrigin` |  |

### Trait Implementations

- `Debug`
- `Clone`

---

## ResolvedSkill

Per-skill resolution provided by the CLI layer after prompting the user.

### Fields

| Name | Type | Description |
|------|------|-------------|
| `name` | `String` |  |
| `source` | `SkillSource` |  |
| `rev` | `Option<String>` |  |

---

## MigrateOptions

### Fields

| Name | Type | Description |
|------|------|-------------|
| `dry_run` | `bool` |  |
| `manifest_options` | `ManifestOptions` |  |

---

## DiscoveryOrigin

### Variants

- **`LockFile`**
- **`AgentsDir`**
- **`ClaudeDir`**

### Trait Implementations

- `Debug`
- `Clone`
- `PartialEq`
- `Eq`

---

## discover_from_lockfile

```rust
pub fn discover_from_lockfile(lockfile_path: &Path) -> Result<Vec<DiscoveredSkill>>
```

Parse a skills-lock.json file and return discovered skills.

---

## discover_from_directories

```rust
pub fn discover_from_directories(project_dir: &Path) -> Result<Vec<DiscoveredSkill>>
```

Scan .agents/skills/ and .claude/skills/ for installed skills.

---

## migrate

```rust
pub fn migrate(project_dir: &Path, resolved: &[ResolvedSkill], options: &MigrateOptions) -> Result<Vec<LockedSkill>>
```

Execute migration for a list of resolved skills.
Returns (migrated count, list of locked skills).

---

## discover_leftover_skills

```rust
pub fn discover_leftover_skills(project_dir: &Path, migrated_names: &HashSet<String>, target_paths: &[String]) -> Result<Vec<DiscoveredSkill>>
```

Scan agent skill directories for non-symlink skill directories that weren't
migrated. These are "leftover" skills that need to be either matched to a
known remote skill or treated as project-specific custom skills.

---

## move_skill_to_local

```rust
pub fn move_skill_to_local(project_dir: &Path, skill: &DiscoveredSkill, options: &ManifestOptions) -> Result<()>
```

Move a leftover skill directory to `.agents/skills/<name>/` and create
symlinks from target directories back to it. This is used for custom
project-specific skills that don't match any known remote skill.

