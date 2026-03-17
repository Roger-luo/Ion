---
title: Binary Skills
description: Install and run compiled tools as skills.
---

Binary skills are compiled executables distributed via GitHub Releases. Ion downloads, verifies, and manages them alongside your prompt-based skills.

## Adding a Binary Skill

```bash
ion add owner/binary-tool
```

When Ion detects a binary skill (via GitHub Releases with pre-built assets), it:

1. Downloads the correct binary for your platform
2. Verifies the checksum
3. Installs it to your local tools directory

## Running Binary Skills

```bash
ion run tool-name [args...]
```

## How It Works

Binary skills use GitHub Releases for distribution. The expected asset naming convention is:

```
tool-name-{version}-{target}.tar.gz
```

Where `{target}` is a Rust target triple like `aarch64-apple-darwin` or `x86_64-unknown-linux-gnu`.

Ion automatically selects the correct asset for your platform and architecture.

## Updates

Binary skills are updated through the same `ion update` workflow:

```bash
ion update tool-name
```

Ion checks GitHub Releases for newer versions and downloads the updated binary.
