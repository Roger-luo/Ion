# JSON Agent Interface Design

## Problem

Ion's interactive commands (TUI pickers, confirmation prompts) are not agent-friendly. AI agents and CI scripts calling Ion as a subprocess cannot interact with TUIs or answer stdin prompts.

## Solution

A global `--json` flag that provides a two-stage command pattern: commands return structured JSON describing available options, then the agent re-runs with explicit flags to execute its decision.

## Global Flag

```
ion [--json] <command> [args...]
```

`--json` changes behavior in two ways:
1. All output is structured JSON on stdout (no stderr output)
2. Commands that need user decisions stop and return options instead of prompting

## JSON Envelope

```json
// Success
{"success": true, "data": { ... }}

// Needs agent decision (exit code 2)
{"success": false, "action_required": "<action_type>", "data": { ... }}

// Error (exit code 1)
{"success": false, "error": "<message>"}
```

### Exit Codes

- `0` — success
- `1` — error
- `2` — action required (agent must re-run with explicit flags)

### `action_required` Values

- `"validation_warnings"` — skill has warnings, needs `--allow-warnings`
- `"skill_selection"` — collection discovered, needs `--skills`
- `"confirm_removal"` — removal preview, needs `--yes`

## Per-Command Behavior

### `ion search <query>`

**Human mode (default):** TUI picker (currently `--interactive`, becomes default). Falls back to plain-text list when stdout is not a TTY.

**JSON mode:**
```bash
$ ion --json search "testing"
{"success": true, "data": [
  {"name": "test-driven-development", "source": "obra/skills/test-driven-development", "description": "...", "stars": 42}
]}
```

The `--interactive/-i` flag is removed.

### `ion add <source>` (single skill)

**JSON mode — with warnings:**
```bash
# Stage 1: preview
$ ion --json add foo/bar
{"success": false, "action_required": "validation_warnings", "data": {
  "skill": "bar",
  "warnings": [{"checker": "security", "message": "..."}]
}}

# Stage 2: proceed
$ ion --json add foo/bar --allow-warnings
{"success": true, "data": {"name": "bar", "installed_to": ".agents/skills/bar/"}}
```

### `ion add <collection>` (multi-skill repo)

**JSON mode:**
```bash
# Stage 1: discover
$ ion --json add foo/collection
{"success": false, "action_required": "skill_selection", "data": {
  "skills": [
    {"name": "skill-a", "status": "clean"},
    {"name": "skill-b", "status": "warnings", "warning_count": 2},
    {"name": "skill-c", "status": "error"}
  ]
}}

# Stage 2: select and install
$ ion --json add foo/collection --skills skill-a,skill-b --allow-warnings
{"success": true, "data": {"installed": ["skill-a", "skill-b"]}}
```

### `ion remove <name>`

**JSON mode:**
```bash
# Stage 1: preview
$ ion --json remove foo
{"success": false, "action_required": "confirm_removal", "data": {"skills": ["foo"]}}

# Stage 2: execute
$ ion --json remove foo --yes
{"success": true, "data": {"removed": ["foo"]}}
```

### `ion project init`

- `--json` without `--target`: returns detected/available targets, exit 2
- `--json` with `--target`: executes and returns result as JSON

### `ion config`

- `--json` with `get`/`set`/`list` subcommands: JSON output
- `--json` with no subcommand: error (TUI not available in JSON mode)

### Other Commands

`skill list`, `skill info`, `self info`, `self check`, `update`, etc. output their data as JSON instead of formatted text.

## New CLI Flags

**Global:**
- `--json` on the `Cli` struct

**`ion add`:**
- `--allow-warnings` — proceed despite validation warnings
- `--skills <comma-list>` — select specific skills from a collection

**`ion search`:**
- Remove `--interactive/-i` (interactive TUI is now the default)

**`ion remove`:**
- `--yes/-y` unchanged (already exists)

## Scope

- All changes in the `ion` CLI crate — no changes to `ion-skill` library
- Define shared JSON response types (serde Serialize)
- Thread `--json` flag through command dispatch
- TTY detection for search fallback
