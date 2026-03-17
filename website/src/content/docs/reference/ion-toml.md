---
title: Ion.toml
description: Project manifest file reference.
---

`Ion.toml` is the project manifest file that declares your skill dependencies and configuration. It lives in your project root.

## Example

```toml
[options]
skills-dir = ".agents/skills"

[options.targets]
claude = ".claude/skills"
cursor = ".cursor/skills"

[skills]
brainstorming = "obra/superpowers/brainstorming"

[skills.code-review]
source = "owner/repo/code-review"
rev = "abc123"

[skills.my-custom-skill]
type = "local"

[skills.ejected-skill]
type = "local"
forked-from = "obra/superpowers/brainstorming"

[skills.my-tool]
type = "binary"
source = "owner/mytool"
binary = "mytool"
```

## `[options]`

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `skills-dir` | `string` | `".agents/skills"` | Directory for local skills |

## `[options.targets]`

A table mapping target names to their skill installation directories. Skills are symlinked into each target directory during install.

Known targets (used by `ion project init --target`):

| Target | Default path |
|--------|-------------|
| `claude` | `.claude/skills` |
| `cursor` | `.cursor/skills` |
| `windsurf` | `.windsurf/skills` |

Custom targets use `name = "path"` syntax:

```toml
[options.targets]
claude = ".claude/skills"
my-tool = "custom/skills/dir"
```

If no targets are configured, skills are only installed to the `skills-dir` (`.agents/skills/` by default) and Ion will print a hint suggesting `ion project init`.

Global targets from `~/.config/ion/config.toml` are merged with project targets. Project targets take precedence on key collision.

## `[skills]`

Each skill entry is keyed by its name (lowercase with hyphens, 1-64 characters).

### Shorthand

The simplest form is a string shorthand:

```toml
[skills]
brainstorming = "obra/superpowers/brainstorming"
```

### Remote Skills (full form)

```toml
[skills.code-review]
source = "owner/repo/code-review"
rev = "abc123"
```

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `source` | `string` | Yes | Skill source (e.g., `"owner/repo"` or `"owner/repo/path"`) |
| `rev` | `string` | No | Pin to a specific git revision |

### Local Skills

```toml
[skills.my-custom-skill]
type = "local"

[skills.ejected-skill]
type = "local"
forked-from = "obra/superpowers/brainstorming"
```

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `type` | `string` | Yes | Must be `"local"` |
| `forked-from` | `string` | No | Original source if ejected from a remote skill |

Local skills live in `{skills-dir}/{name}/` and are managed by git directly.

### Binary Skills

```toml
[skills.my-tool]
type = "binary"
source = "owner/mytool"
binary = "mytool"
asset-pattern = "mytool-{version}-{target}.tar.gz"
```

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `type` | `string` | Yes | Must be `"binary"` |
| `source` | `string` | Yes | GitHub `owner/repo` for the binary |
| `binary` | `string` | No | Binary name (defaults to skill name) |
| `asset-pattern` | `string` | No | Custom GitHub Release asset naming pattern |
| `rev` | `string` | No | Pin to a specific release version |

## Ion.lock

The lockfile (`Ion.lock`) is automatically generated and should be committed to version control. It pins exact revisions and checksums for reproducible installs.

```toml
[[skill]]
name = "brainstorming"
source = "https://github.com/obra/superpowers.git"
path = "brainstorming"
commit = "abc123..."
checksum = "sha256:..."
```

Binary skills include additional fields:

```toml
[[skill]]
name = "my-tool"
source = "https://github.com/owner/mytool.git"
binary = "mytool"
binary-version = "0.2.0"
binary-checksum = "sha256:..."
```
