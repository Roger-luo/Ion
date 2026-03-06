# Binary CLI Skills Developer Guide

Binary skills are CLI tools that provide capabilities to AI agents. Ion manages
downloading, versioning, and invoking these tools so agents can use them
seamlessly.

There are two patterns:

- **Binary IS the skill** -- the agent invokes the tool directly via `ion run`.
- **Binary generates SKILL.md** -- the tool has a `skill` subcommand that emits
  a skill definition to stdout.

## Quick Start

```bash
ion new --bin --path my-tool
cd my-tool
cargo build
cargo run -- skill   # test the skill subcommand
```

## The `skill` Subcommand Convention

Every Ion binary skill should expose a `skill` subcommand that prints a valid
SKILL.md to stdout (YAML frontmatter + markdown body). Ion runs this during
install to generate or update the SKILL.md.

```bash
$ mytool skill
---
name: mytool
description: What this tool does. Invoke with `ion run mytool`.
metadata:
  binary: mytool
  version: 1.0.0
---

# mytool

Instructions for the agent on how to use this tool...
```

Alternatively, you can bundle a static `SKILL.md` in your release tarball
instead of implementing the subcommand.

## SKILL.md Metadata for Binaries

The YAML frontmatter must include a `binary` field so Ion knows this is a
binary skill:

```yaml
---
name: mytool
description: What this tool does. Invoke with `ion run mytool`.
metadata:
  binary: mytool
  version: 1.0.0
---
```

- `binary` -- required. Tells Ion which executable to invoke.
- `version` -- should match your release version.

## Publishing on GitHub

Create GitHub Releases with semantic version tags (e.g., `v1.0.0`) and attach
platform-specific tarballs as release assets.

### Naming convention

Use `{binary}-{target}.tar.gz`:

```
mytool-x86_64-apple-darwin.tar.gz
mytool-aarch64-apple-darwin.tar.gz
mytool-x86_64-unknown-linux-gnu.tar.gz
mytool-aarch64-unknown-linux-gnu.tar.gz
```

Alternative naming with `{binary}-{os}-{arch}.tar.gz` is also supported:

```
mytool-linux-amd64.tar.gz
mytool-darwin-arm64.tar.gz
```

For non-standard naming, set `asset-pattern` in Ion.toml (see Configuration
below).

## Installing a Binary Skill

Users install your published tool with:

```bash
ion add owner/mytool --bin
```

## Local Development

```bash
# Link your local build for testing
ion link ./path/to/my-tool

# Or add from a local path
ion add ./path/to/my-tool --bin
```

## Configuration in Ion.toml

```toml
[skills]
# GitHub source (most common)
mytool = { type = "binary", source = "owner/mytool", binary = "mytool" }

# Pinned version
mytool = { type = "binary", source = "owner/mytool", binary = "mytool", rev = "v1.2.0" }

# Custom asset naming
mytool = { type = "binary", source = "owner/mytool", binary = "mytool", asset-pattern = "mytool-{version}-{os}-{arch}.tar.gz" }

# Generic URL source
mytool = { type = "binary", source = "https://example.com/releases/{version}/mytool-{target}.tar.gz", binary = "mytool", rev = "1.2.0" }
```

## Target Triples and Platform Aliases

Ion resolves platform-specific assets using target triples and common aliases:

| Platform     | Target Triple                  | OS Aliases            | Arch Aliases          |
|--------------|--------------------------------|-----------------------|-----------------------|
| macOS x86_64 | `x86_64-apple-darwin`          | darwin, macos, apple  | x86_64, amd64, x64   |
| macOS ARM    | `aarch64-apple-darwin`         | darwin, macos, apple  | aarch64, arm64        |
| Linux x86_64 | `x86_64-unknown-linux-gnu`     | linux                 | x86_64, amd64, x64   |
| Linux ARM    | `aarch64-unknown-linux-gnu`    | linux                 | aarch64, arm64        |

When searching for release assets, Ion tries the full target triple first, then
falls back to OS/arch alias combinations.
