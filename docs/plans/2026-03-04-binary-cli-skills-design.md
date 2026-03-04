# Binary CLI Skills Design

## Problem

Ion currently manages declarative SKILL.md files from git repos, HTTP sources, and local paths. There's no support for CLI tools that provide agent capabilities — Rust binaries that the agent invokes during conversations, or binaries that generate their own SKILL.md descriptions.

## Solution

Extend Ion with a `Binary` source type that downloads platform-specific CLI binaries, generates SKILL.md files from them, and provides `ion run` as the universal invocation interface.

## Architecture: Unified Source Model

Add a `Binary` variant to the existing `Source` enum. Binary skills flow through the same installation pipeline as other skills (validate SKILL.md, deploy symlinks, update lockfile), with a binary download/extract step prepended.

## Two Patterns Supported

1. **Binary IS the skill** — The CLI tool is invoked by the agent via `ion run <tool> <args>`. The SKILL.md describes the tool's capabilities and tells the agent how to use it.

2. **Binary generates SKILL.md** — The CLI has a `skill` subcommand that outputs a SKILL.md to stdout. Ion runs this during installation to capture the skill description.

Both patterns use the same infrastructure. A binary skill always has a `skill` subcommand (convention). During install, Ion runs `<binary> skill` to generate the SKILL.md, or uses a bundled SKILL.md if present in the tarball.

## Data Model

### Ion.toml

```toml
[skills]
# Binary skill from GitHub Releases
mytool = { type = "binary", source = "owner/mytool", binary = "mytool" }

# Binary skill with explicit URL pattern
othertool = { type = "binary", source = "https://example.com/releases/{version}/othertool-{target}.tar.gz", binary = "othertool" }

# Existing patterns unchanged
brainstorming = "anthropics/skills/brainstorming"
```

### Ion.lock Extension

```toml
[[skill]]
name = "mytool"
source = "https://github.com/owner/mytool.git"
version = "1.2.0"
commit = "abc123"
checksum = "sha256:..."
binary = "mytool"
binary_version = "1.2.0"
binary_checksum = "sha256:..."
```

### SKILL.md Metadata

Generated SKILL.md includes `binary` in metadata frontmatter:

```yaml
---
name: mytool
description: A tool that does X. Invoke with `ion run mytool <args>`.
metadata:
  binary: mytool
  version: 1.2.0
---
```

## Platform Detection

Detect the current platform using `std::env::consts`:
- OS: `linux`, `macos`, `windows`
- Arch: `x86_64`, `aarch64`
- Target triple: e.g. `x86_64-apple-darwin`, `aarch64-unknown-linux-gnu`

## Binary Download Flow

### GitHub Releases

1. Query `GET /repos/{owner}/{repo}/releases/latest` (or specific tag via `rev`)
2. Match release asset by naming convention:
   - `{binary}-{target}.tar.gz`
   - `{binary}-{os}-{arch}.tar.gz`
   - `{binary}-{arch}-{os}.tar.gz`
   - Configurable pattern for non-standard naming
3. Download and verify checksum if provided
4. Extract `.tar.gz`, `.tar.xz`, or `.zip`
5. Locate executable in extracted contents

### Generic URLs

URL templates with placeholders: `{version}`, `{target}`, `{os}`, `{arch}`, `{binary}`

## Binary Storage

```
~/.local/share/ion/bin/
├── mytool/
│   ├── 1.2.0/
│   │   └── mytool
│   └── current -> 1.2.0/
└── othertool/
    ├── 0.5.0/
    │   └── othertool
    └── current -> 0.5.0/
```

Binaries are NOT added to PATH. They live in Ion's managed storage only. The agent invokes them exclusively through `ion run`.

## `ion run` Command

```
ion run <name> [-- <args>...]
```

1. Read Ion.toml/Ion.lock for the named skill
2. Resolve binary path from `~/.local/share/ion/bin/{name}/{version}/{binary}`
3. Verify binary exists (error if missing, suggest `ion install`)
4. `exec` the binary with args, passing through stdin/stdout/stderr

This is the universal interface for binary skills. The SKILL.md tells the agent to use `ion run <tool> <args>` — no absolute paths, no PATH pollution, fully portable across machines.

## Installation Flow

```
ion add owner/mytool --bin
  │
  ├─ Resolve source as Binary type
  ├─ Query GitHub API for releases
  ├─ Detect platform, match asset
  │
  ├─ Download & extract binary
  ├─ Store at ~/.local/share/ion/bin/mytool/{version}/
  ├─ Update current symlink
  │
  ├─ Check for bundled SKILL.md in tarball
  ├─ If none: run `<binary> skill`, capture stdout
  ├─ Validate generated SKILL.md
  │
  ├─ Deploy SKILL.md via symlinks (existing pipeline)
  ├─ Update Ion.toml, Ion.lock, .gitignore, registry
  └─ Done
```

## Update Flow

1. Query GitHub API for latest release (or check URL)
2. Compare with lockfile `binary_version`
3. If newer: download, install alongside old version, update `current` symlink
4. Re-run `<binary> skill` to regenerate SKILL.md
5. Update lockfile with new version/checksum

## Remove Flow

1. Remove skill symlinks (existing behavior)
2. Check global registry — if no other projects reference this binary, remove from `ion/bin/`
3. Update manifest, lockfile, registry

## Phased Implementation

### Phase 1: Core Binary Skills
1. Binary source type — `type = "binary"` in Ion.toml, `--bin` flag for `ion add`
2. Platform detection — OS/arch detection, target triple construction
3. GitHub Releases download — API query, asset matching, download, extraction
4. Binary storage — Versioned storage with `current` symlink
5. SKILL.md generation — Run `<binary> skill`, capture stdout, validate
6. Bundled SKILL.md fallback — Use SKILL.md from tarball if present
7. `ion run` command — Resolve and execute binary skills
8. Lockfile extension — Track binary name, version, checksum

### Phase 2: Lifecycle Management
9. `ion update` for binaries — Check new releases, download, regenerate SKILL.md
10. Binary cleanup on remove — Remove binary when no projects reference it
11. `ion install` binary support — Install binaries from lockfile
12. Version pinning — `rev = "v1.2.0"` for specific release tags

### Phase 3: Extended Sources
13. Generic URL downloads — Non-GitHub URLs with placeholders
14. Asset naming patterns — Configurable for non-standard releases
15. Binary validation — Verify executable, test `--version` / `skill` subcommand
16. `ion list` binary indicator — Show binary skills in listings
17. `ion info` binary details — Version, path, size

### Phase 4: Developer Experience
18. `ion new --bin` — Scaffold Rust CLI with `skill` subcommand
19. Skill subcommand library/template — Crate for implementing `<binary> skill`
20. Documentation — Guide for CLI authors on Ion compatibility
