---
title: "ion-skill::agents"
description: ""
order: 999
---

## AgentsConfig

Configuration for AGENTS.md template management.
Parsed from [agents] in Ion.toml.

### Fields

| Name | Type | Description |
|------|------|-------------|
| `template` | `Option<String>` | Template source (GitHub shorthand, Git URL, HTTP, or local path) |
| `rev` | `Option<String>` | Pin to a specific git revision |
| `path` | `Option<String>` | Path to AGENTS.md within the source repo (default: "AGENTS.md" at root) |

### Trait Implementations

- `Debug`
- `Clone`
- `Default`
- `Serialize`
- `Deserialize<'de>`

---

## AgentsLockEntry

Lock entry for the AGENTS.md template.
Tracks the last-synced state in Ion.lock.

### Fields

| Name | Type | Description |
|------|------|-------------|
| `template` | `String` |  |
| `rev` | `Option<String>` |  |
| `checksum` | `String` |  |
| `updated_at` | `String` |  |

### Trait Implementations

- `Debug`
- `Clone`
- `Serialize`
- `Deserialize<'de>`

---

## FetchedTemplate

Result of fetching an AGENTS.md template

### Fields

| Name | Type | Description |
|------|------|-------------|
| `content` | `String` |  |
| `rev` | `Option<String>` |  |
| `checksum` | `String` |  |

---

## ensure_agent_symlinks

```rust
pub fn ensure_agent_symlinks(project_dir: &Path, targets: &BTreeMap<String, String>) -> Result<()>
```

For each configured target that has an entry in AGENT_FILE_SYMLINKS,
create a symlink (e.g. CLAUDE.md -> AGENTS.md) if AGENTS.md exists
and the symlink doesn't.

Symlinks are only created for targets configured in [options.targets].
If a target filename already exists as a regular file or a symlink
pointing elsewhere, a warning is printed and it is skipped.

---

## is_agents_pointer

```rust
pub fn is_agents_pointer(content: &str) -> bool
```

Check whether a file's content is just a pointer to AGENTS.md.

Returns `true` if every non-blank line's only purpose is to reference
`@AGENTS.md` — e.g. the bare string `@AGENTS.md` or prose like
"treat @AGENTS.md the same as this file".

Returns `false` if the file contains additional instructions beyond
the reference, has no `@AGENTS.md` at all, or is empty.

---

## now_iso8601

```rust
pub fn now_iso8601() -> String
```

Current UTC time as ISO 8601 string (e.g. "2026-03-27T12:00:00Z").

---

## fetch_template

```rust
pub fn fetch_template(source_str: &str, rev: Option<&str>, file_path: Option<&str>, _project_dir: &Path) -> Result<FetchedTemplate>
```

Fetch an AGENTS.md template from a source.

Resolves the source using SkillSource::infer, fetches the repo/path,
and extracts the AGENTS.md file at the specified path (default: root).

