---
title: Installation
description: How to install Ion on your system.
order: 2
---

# Installation

Ion provides pre-built binaries for macOS and Linux on both x86_64 and ARM architectures.

## Quick install

The fastest way to install Ion is with the install script:

```bash
curl -fsSL https://raw.githubusercontent.com/Roger-luo/Ion/main/install.sh | sh
```

This detects your platform and downloads the appropriate binary.

## From GitHub Releases

Download the latest release directly from [GitHub Releases](https://github.com/Roger-luo/Ion/releases). Binaries follow the naming convention:

```
ion-{version}-{target}.tar.gz
```

Available targets:

| Target | Platform |
|--------|----------|
| `aarch64-apple-darwin` | macOS (Apple Silicon) |
| `x86_64-apple-darwin` | macOS (Intel) |
| `aarch64-unknown-linux-gnu` | Linux (ARM) |
| `x86_64-unknown-linux-gnu` | Linux (x86) |

## From source

If you have Rust installed, you can build from source:

```bash
cargo install --git https://github.com/Roger-luo/Ion.git
```

## Verify installation

After installing, verify that Ion is available:

```bash
ion --version
```

## Self-update

Ion can update itself:

```bash
ion self update    # Update to the latest version
ion self check     # Check if an update is available
ion self info      # Show version and build info
```
