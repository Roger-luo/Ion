---
title: Installation
description: How to install Ion on your system.
---

## Quick Install

The fastest way to install Ion is via the install script:

```bash
curl -fsSL https://raw.githubusercontent.com/Roger-luo/Ion/main/install.sh | sh
```

This downloads a pre-built binary for your platform (macOS or Linux, ARM or x86) and places it in your PATH.

### Install a Specific Version

```bash
curl -fsSL https://raw.githubusercontent.com/Roger-luo/Ion/main/install.sh | sh -s -- 0.2.0
```

## From Source

If you have a Rust toolchain installed:

```bash
cargo install --git https://github.com/Roger-luo/Ion
```

## Self-Update

Once installed, Ion can update itself:

```bash
# Check for updates
ion self check

# Update to the latest version
ion self update

# Show current version and build info
ion self info
```

## Verify Installation

```bash
ion --version
```
