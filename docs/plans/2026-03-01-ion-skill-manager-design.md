# ion — Agent Skill Manager

## Overview

ion is a package manager for AI agent skills, similar to Cargo but for the [Agent Skills](https://agentskills.io/specification) ecosystem. It manages skill dependencies at the project level, fetching skills from git repositories, HTTP sources, or local paths, and installing them into the standard `.agents/` directory.

## Project Structure

```
ion/
├── Cargo.toml          # Workspace root + CLI binary
├── src/
│   ├── main.rs
│   └── commands/
│       ├── mod.rs
│       ├── add.rs
│       ├── remove.rs
│       ├── install.rs
│       ├── list.rs
│       └── info.rs
├── crates/
│   └── ion-skill/      # Skill management library
│       ├── Cargo.toml
│       └── src/
│           ├── lib.rs
│           ├── manifest.rs    # ion.toml parsing/writing
│           ├── lockfile.rs    # ion.lock parsing/writing
│           ├── skill.rs       # SKILL.md parsing & validation
│           ├── source.rs      # Source resolution (github, git, http, path)
│           ├── installer.rs   # Downloads & installs skills
│           └── resolver.rs    # Shorthand → git URL resolution
```

The root crate is the CLI binary (clap-based). `ion-skill` under `crates/` contains all business logic, enabling future use as a library by other tools.

## Manifest Format (ion.toml)

```toml
[skills]
# GitHub shorthand: owner/repo/skill-path
brainstorming = "anthropics/skills/brainstorming"

# Explicit with type and version
frontend-design = { type = "github", source = "anthropics/skills/frontend-design", version = "1.0" }

# Pin to specific git ref
my-tool = { type = "github", source = "org/skills/my-tool", rev = "v2.0" }

# Any git host
gitlab-skill = { type = "git", source = "https://gitlab.com/org/skills.git", path = "my-skill" }

# HTTP download
remote-skill = { type = "http", source = "https://example.com/skills/my-skill.tar.gz" }

# Local path
local-skill = { type = "path", source = "../my-local-skill" }

[options]
install-to-claude = true   # Also install into .claude/skills/
```

### Source Types

| Type     | Source Format                        | Description                                    |
|----------|--------------------------------------|------------------------------------------------|
| `github` | `owner/repo` or `owner/repo/path`   | GitHub repository, default when format matches |
| `git`    | Any git URL                          | Generic git repository + optional `path`       |
| `http`   | URL to archive                       | Direct download (tar.gz, zip)                  |
| `path`   | Filesystem path                      | Local directory                                |

When `type` is omitted, ion infers it:
- `owner/repo` or `owner/repo/path` pattern → `github`
- URL starting with `https://github.com` → `github`
- Other git URLs → `git`
- HTTP URLs → `http`
- Paths starting with `.` or `/` → `path`

### Skill Entry Fields

| Field     | Required | Description                                           |
|-----------|----------|-------------------------------------------------------|
| `type`    | No       | Source type (github, git, http, path). Auto-inferred.  |
| `source`  | Yes      | Location of the skill (shorthand, URL, or path)       |
| `version` | No       | Required metadata.version from SKILL.md               |
| `rev`     | No       | Git ref (branch, tag, or commit SHA) to pin to        |
| `path`    | No       | Subdirectory within a git repo containing the skill   |

## Lockfile Format (ion.lock)

```toml
[[skill]]
name = "brainstorming"
source = "https://github.com/anthropics/skills.git"
path = "brainstorming"
version = "1.0"
commit = "abc123def456789..."
checksum = "sha256:..."

[[skill]]
name = "my-tool"
source = "https://github.com/org/skills.git"
path = "my-tool"
commit = "789xyz..."
checksum = "sha256:..."
```

The lockfile pins exact commit SHAs and checksums for reproducible installs. It is generated/updated by `ion add` and `ion install`.

## Commands

### `ion add <skill> [--rev <ref>]`

Add a skill to the project.

1. Parse the source (infer type if not explicit)
2. Fetch the skill (clone repo, download archive, or read local path)
3. Validate: check for valid SKILL.md with required frontmatter
4. If `--rev` specified, checkout that ref
5. If `version` specified, validate metadata.version matches
6. Add entry to `ion.toml`
7. Install to `.agents/skills/<name>/` (and `.claude/skills/` if enabled)
8. Update `ion.lock` with pinned commit/checksum

### `ion remove <skill>`

Remove a skill from the project.

1. Remove entry from `ion.toml`
2. Remove entry from `ion.lock`
3. Delete from `.agents/skills/<name>/`
4. Delete from `.claude/skills/<name>/` if present

### `ion install`

Install all skills from the manifest.

1. Read `ion.toml`
2. If `ion.lock` exists, use pinned versions; otherwise resolve fresh
3. Fetch and validate each skill
4. Install to `.agents/skills/` (and `.claude/skills/` if enabled)
5. Create or update `ion.lock`

### `ion list`

List all skills declared in `ion.toml` with their source, version, and install status.

### `ion info <skill>`

Display detailed information about a skill by fetching and parsing its SKILL.md metadata.

## Installation Layout

```
project/
├── ion.toml
├── ion.lock
├── .agents/
│   └── skills/
│       ├── brainstorming/
│       │   └── SKILL.md
│       └── my-tool/
│           ├── SKILL.md
│           └── scripts/
├── .claude/                    # When install-to-claude = true
│   └── skills/
│       ├── brainstorming/
│       └── my-tool/
```

Skills are copied (not symlinked) to avoid compatibility issues across different agent tools.

## Key Dependencies

- **clap** (derive) — CLI argument parsing
- **toml** / **toml_edit** — Manifest and lockfile parsing/writing
- **serde** / **serde_yaml** — SKILL.md frontmatter parsing
- **git2** or shell git — Git operations (clone, fetch, checkout)
- **reqwest** — HTTP downloads
- **sha2** — Checksum generation

## Future Extensions

- Subagent management (beyond skills)
- Other extension types
- Central registry/search
- `ion update` command for updating pinned versions
- `ion publish` for publishing skills
