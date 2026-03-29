---
title: "ion-skill::gitignore"
description: ""
order: 999
---

## find_missing_gitignore_entries

```rust
pub fn find_missing_gitignore_entries(project_dir: &Path, dirs: &[&str]) -> Result<Vec<String>>
```

Check which directories from the given list are missing from .gitignore.
Returns the list of directories that are NOT in .gitignore.

---

## append_to_gitignore

```rust
pub fn append_to_gitignore(project_dir: &Path, entries: &[&str]) -> Result<()>
```

Append entries to .gitignore, creating it if it doesn't exist.

---

## add_skill_entries

```rust
pub fn add_skill_entries(project_dir: &Path, skill_name: &str, target_paths: &[&str], skills_dir: &str) -> Result<()>
```

Add per-skill gitignore entries for a remotely installed skill.
Creates entries for `<skills_dir>/<name>` and `<target>/<name>` for each target.
Idempotent — won't duplicate existing entries.

---

## ensure_agent_file_ignored

```rust
pub fn ensure_agent_file_ignored(project_dir: &Path, filename: &str) -> Result<()>
```

Ensure a single file (e.g. `CLAUDE.md`) is listed in `.gitignore`.
Idempotent — won't duplicate an existing entry.

---

## remove_skill_entries

```rust
pub fn remove_skill_entries(project_dir: &Path, skill_name: &str) -> Result<()>
```

Remove all gitignore entries for a specific skill.
Removes any line ending with `/<name>` under the managed section.
Cleans up the "# Managed by ion" header if no managed entries remain.

