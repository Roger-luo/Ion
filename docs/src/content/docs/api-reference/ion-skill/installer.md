---
title: "ion-skill::installer"
description: "Skill installation pipeline — resolve, fetch, validate, deploy to target directories, and write manifest/lockfile."
order: 999
---

## SkillInstaller

Manages skill installation and uninstallation for a project.

*…and private fields*

### Methods

#### `new`

```rust
pub fn new(project_dir: &'a Path, options: &'a ManifestOptions) -> Self
```

#### `project_dir`

```rust
pub fn project_dir(&self) -> &Path
```

#### `options`

```rust
pub fn options(&self) -> &ManifestOptions
```

#### `skill_dir`

```rust
pub fn skill_dir(&self, name: &str) -> PathBuf
```

Compute the canonical skill directory path: `{project_dir}/{skills_dir}/{name}`.

#### `install`

```rust
pub fn install(&self, name: &str, source: &SkillSource) -> Result<LockedSkill>
```

#### `validate`

```rust
pub fn validate(&self, _name: &str, source: &SkillSource) -> Result<ValidationReport>
```

Fetch and validate a skill without deploying it.
Returns the validation report on success (even if it has warnings).
Returns `Error::ValidationFailed` if there are errors,
or `Error::InvalidSkill` if there's no SKILL.md.

#### `install_with_options`

```rust
pub fn install_with_options(&self, name: &str, source: &SkillSource, validation: InstallValidationOptions) -> Result<LockedSkill>
```

#### `discover_skills`

```rust
pub fn discover_skills(source: &SkillSource) -> Result<Vec<(String, String)>>
```

Fetch a source and discover all skills within it.
Returns a list of (skill_name, skill_path_within_repo) pairs.
Used for multi-skill collection repos that have no root SKILL.md.

#### `uninstall`

```rust
pub fn uninstall(&self, name: &str) -> Result<()>
```

#### `deploy`

```rust
pub fn deploy(&self, name: &str, skill_dir: &Path) -> Result<()>
```

---

## InstallValidationOptions

### Fields

| Name | Type | Description |
|------|------|-------------|
| `skip_validation` | `bool` |  |
| `allow_warnings` | `bool` |  |

### Trait Implementations

- `Debug`
- `Clone`
- `Default`

---

## builtin_skills_dir

```rust
pub fn builtin_skills_dir() -> PathBuf
```

Where ion stores built-in skills that ship with the binary.

---

## data_dir

```rust
pub fn data_dir() -> PathBuf
```

Where ion stores cloned repositories persistently.

---

## repo_dir_for_source

```rust
pub fn repo_dir_for_source(source: &SkillSource) -> Result<PathBuf>
```

Compute the cache directory for a git-based source (does not clone or check existence).

---

## cached_repo_path

```rust
pub fn cached_repo_path(source: &SkillSource) -> Option<PathBuf>
```

Return the cached clone directory for a git-based source, if it exists.

This does NOT clone or fetch — it only checks whether a previous clone
left a cached directory on disk. Returns `None` for non-git sources or
if the cache directory doesn't exist.

---

## resolve_skill_dir

```rust
pub fn resolve_skill_dir(repo_dir: &Path, path: Option<&str>) -> Result<PathBuf>
```

Resolve the skill directory within a repo, handling subdirectory skills.
Tries `repo_dir/path` first, then falls back to `repo_dir/skills/path`.

---

## hash_simple

```rust
pub fn hash_simple(s: &str) -> u64
```

