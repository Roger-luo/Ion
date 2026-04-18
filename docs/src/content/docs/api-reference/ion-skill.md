---
title: "ion-skill"
description: "Core library for Ion skill management — installation, validation, search, and configuration."
order: 100
---

*Version 0.1.0*

## Modules

| Module | Description |
|--------|-------------|
| [agents](/docs/api-reference/ion-skill/agents) | AGENTS.md template management |
| [binary](/docs/api-reference/ion-skill/binary) | Binary skill installation from GitHub Releases |
| [config](/docs/api-reference/ion-skill/config) | Global user configuration |
| [error](/docs/api-reference/ion-skill/error) | Error types |
| [git](/docs/api-reference/ion-skill/git) | Git operations for skill management |
| [gitignore](/docs/api-reference/ion-skill/gitignore) | Manage .gitignore entries for skills |
| [installer](/docs/api-reference/ion-skill/installer) | Skill installation pipeline |
| [lockfile](/docs/api-reference/ion-skill/lockfile) | Ion.lock types for pinned skill versions |
| [manifest](/docs/api-reference/ion-skill/manifest) | Ion.toml project configuration types |
| [manifest_writer](/docs/api-reference/ion-skill/manifest-writer) | Programmatic Ion.toml editing |
| [migrate](/docs/api-reference/ion-skill/migrate) | Migrate skills from legacy formats |
| [registry](/docs/api-reference/ion-skill/registry) | Global registry of skill repositories |
| [search](/docs/api-reference/ion-skill/search) | Skill search results and multi-backend runners |
| [skill](/docs/api-reference/ion-skill/skill) | SKILL.md parsing and metadata |
| [source](/docs/api-reference/ion-skill/source) | Skill source abstraction |
| [update](/docs/api-reference/ion-skill/update) | Check and apply skill updates |
| [validate](/docs/api-reference/ion-skill/validate) | Skill validation framework |

## Re-exports

- `pub use ionem::self_update` as **self_update**
- `pub use error::Error` as **Error**

