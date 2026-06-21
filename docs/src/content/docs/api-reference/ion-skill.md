---
title: "ion-skill"
description: "Core library for Ion skill management — installation, validation, search, and configuration."
order: 100
---

*Version 0.1.0*

Core library for Ion skill management — installation, validation, search, and configuration.

## Modules

| Module | Description |
|--------|-------------|
| [agents](/docs/api-reference/ion-skill/agents) | AGENTS.md template management — fetch, render, and keep agent context files up to date. |
| [binary](/docs/api-reference/ion-skill/binary) | Binary skill installation — download from GitHub Releases, extract, and verify platform-specific executables. |
| [config](/docs/api-reference/ion-skill/config) | Global user configuration — cache settings, registry sources, and agent target defaults stored in `~/.config/ion/config.toml`. |
| [error](/docs/api-reference/ion-skill/error) | Error types for the ion-skill library, covering IO, parsing, Git, HTTP, validation, and manifest failures. |
| [git](/docs/api-reference/ion-skill/git) | Git operations for skill management — clone, fetch, checkout, and compute directory checksums. |
| [gitignore](/docs/api-reference/ion-skill/gitignore) | Manage `.gitignore` entries for installed skills and agent directories. |
| [installer](/docs/api-reference/ion-skill/installer) | Skill installation pipeline — resolve, fetch, validate, deploy to target directories, and write manifest/lockfile. |
| [lockfile](/docs/api-reference/ion-skill/lockfile) | Ion.lock types — track installed skills with pinned versions and checksums across Git, binary, and local sources. |
| [manifest](/docs/api-reference/ion-skill/manifest) | Ion.toml types — project skill configuration with targets, skill entries, and per-project options. |
| [manifest_writer](/docs/api-reference/ion-skill/manifest-writer) | Programmatic Ion.toml editing — add/remove skills, write targets, and set configuration options in place. |
| [migrate](/docs/api-reference/ion-skill/migrate) | Migrate skills from legacy Ion formats to the current manifest and lockfile layout. |
| [registry](/docs/api-reference/ion-skill/registry) | Global registry of skill repositories — tracks which projects use which remote repos and cleans up stale entries. |
| [search](/docs/api-reference/ion-skill/search) | Skill search results and multi-backend search runners — GitHub, registry, and agent sources with relevance sorting. |
| [skill](/docs/api-reference/ion-skill/skill) | SKILL.md parsing — read and validate skill metadata from frontmatter. |
| [source](/docs/api-reference/ion-skill/source) | Skill source abstraction — represent and resolve GitHub, Git, HTTP, path, binary, and local skill origins. |
| [templates](/docs/api-reference/ion-skill/templates) | Built-in AGENTS.md templates shipped with the ion binary. |
| [tool_permission](/docs/api-reference/ion-skill/tool-permission) |  |
| [update](/docs/api-reference/ion-skill/update) | Skill update infrastructure — check for newer versions and apply updates across Git and binary sources. |
| [validate](/docs/api-reference/ion-skill/validate) | Skill validation framework — run checkers against SKILL.md files and aggregate findings by severity. |
| [workspace](/docs/api-reference/ion-skill/workspace) | Project workspace context — load manifest and lockfile, resolve effective options and skill paths for a project. |

## Re-exports

- `pub use ionem::self_update` as **self_update**
- `pub use error::Error` as **Error**

