---
title: "ion-skill::manifest_writer"
description: ""
order: 999
---

## add_skill

```rust
pub fn add_skill(manifest_path: &Path, name: &str, source: &SkillSource) -> Result<String>
```

Add a skill entry to an Ion.toml string. Returns the updated TOML string.

---

## remove_skill

```rust
pub fn remove_skill(manifest_path: &Path, name: &str) -> Result<String>
```

Remove a skill entry from an Ion.toml file. Returns the updated TOML string.

---

## write_targets

```rust
pub fn write_targets(manifest_path: &Path, targets: &BTreeMap<String, String>) -> Result<String>
```

Write target entries to an Ion.toml file's [options.targets] section.
Creates the file with a [skills] section if it doesn't exist.
Preserves all existing content.

---

## write_skills_dir

```rust
pub fn write_skills_dir(manifest_path: &Path, skills_dir: &str) -> Result<String>
```

Write a skills-dir value to an Ion.toml file's [options] section.
Creates the file with a [skills] section if it doesn't exist.
Preserves all existing content.

---

## write_agents_config

```rust
pub fn write_agents_config(manifest_path: &Path, template: &str, rev: Option<&str>, path: Option<&str>) -> Result<String>
```

Write an [agents] section to an Ion.toml file.
Creates the file with a [skills] section if it doesn't exist.
Preserves all existing content.

---

## set_option

```rust
pub fn set_option(manifest_path: &Path, key: &str, val: &str) -> Result<String>
```

Set a value in the [options] section of Ion.toml.

Handles nested sub-sections like `targets.claude` by writing to
`[options.targets]`, and direct option keys like `options.skills-dir`
by writing to `[options]`.

---

## delete_option

```rust
pub fn delete_option(manifest_path: &Path, key: &str) -> Result<String>
```

Delete a value from the [options] section of Ion.toml.

For `targets.<key>`, removes the key from `[options.targets]`.
For `options.<key>`, removes the key from `[options]`.

