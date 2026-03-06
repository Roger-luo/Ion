# Command Reorganization Design

## Problem

Ion has 16 flat top-level commands. `ion --help` is overwhelming and hard to scan. Commands span different concerns (daily workflow, skill authoring, project setup, housekeeping) but are all presented equally.

## Solution

Reduce to 5 top-level commands + 3 subcommand groups. Group by usage frequency and domain.

## Command Structure

### Top-level (daily workflow)

```
ion add [source] [--rev] [--bin]       # no args = install all from Ion.toml
ion remove <name> [-y]
ion search <query> [--agent] [-i] [--source] [--limit] [-v]
ion update [name]
ion run <name> [args...]
```

`ion add` absorbs `ion install`: when called with no arguments, it reads Ion.toml and installs all skills (same behavior as the old `ion install`). When called with a source argument, it adds that specific skill.

### `ion skill` (authoring & inspection)

```
ion skill new [--path] [--bin] [--collection] [--force]
ion skill validate [path]
ion skill info <name>
ion skill list
ion skill link <path>
```

### `ion project` (setup & migration)

```
ion project init [-t target] [--force]
ion project migrate [--from] [--dry-run]
```

### `ion cache` (housekeeping)

```
ion cache gc [--dry-run]
```

### `ion config` (unchanged)

```
ion config [get|set|list|edit]
```

## Help Output

```
Agent skill manager

Usage: ion <COMMAND>

Commands:
  add      Add skills to the project, or install all from Ion.toml
  remove   Remove a skill from the project
  search   Search for skills across registries and GitHub
  update   Update skills to their latest versions
  run      Run a binary skill
  skill    Create, inspect, and validate skills
  project  Project setup and migration
  cache    Manage the skill cache
  config   Manage ion configuration
```

## Migration

Clean break — no hidden aliases. This is pre-1.0 software. Old command names (`install`, `list`, `info`, `init`, `link`, `validate`, `new`, `migrate`, `gc`) are removed entirely.

## Implementation Notes

- `Commands` enum gains `Skill`, `Project`, `Cache` variants with nested `#[derive(Subcommand)]` enums, matching the existing `Config` pattern
- Command module files stay in `src/commands/` — no directory restructuring, just re-routing in `main.rs`
- `ion add` changes `source` from `String` to `Option<String>`; `None` triggers the install-all path
- All flags on moved commands remain identical
- Integration tests update to use new command paths
