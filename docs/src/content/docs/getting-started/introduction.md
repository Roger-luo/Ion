---
title: Introduction
description: What Ion is and why it exists.
order: 1
---

# Introduction

Ion is a CLI skill manager for AI agent tools. It handles the installation, validation, and organization of **skills** — reusable prompts and instructions that enhance the capabilities of AI agents like Claude, Cursor, Windsurf, and others.

## Why Ion?

As AI agents become more capable, the ecosystem of reusable skills grows. But managing these skills across projects and agents is cumbersome:

- **No standard tooling** — skills are typically copied manually between projects
- **No quality assurance** — no way to validate skills before installing them
- **No reproducibility** — without lockfiles, skill versions can drift across environments
- **No discoverability** — finding useful skills means searching GitHub or asking around

Ion solves all of these with a familiar package-manager workflow: `ion add`, `ion update`, `ion search`.

## How it works

Ion uses two manifest files to track your skills:

- **`Ion.toml`** — declares which skills your project uses and where they come from
- **`Ion.lock`** — pins exact versions and checksums for reproducibility

Skills can come from multiple sources:

| Source | Example |
|--------|---------|
| GitHub | `github:owner/repo` |
| Git | `git:https://example.com/repo.git` |
| HTTP | `https://example.com/skill.tar.gz` |
| Local | Created with `ion skill new` |

## Next steps

- [Install Ion](/docs/getting-started/installation) on your system
- Follow the [Quick Start](/docs/getting-started/quick-start) guide to add your first skill
