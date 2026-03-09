---
name: ion-cli
description: "Operate the Ion skill manager from CLI using --json flag for structured, non-interactive control of skill installation, search, and project management."
compatibility:
  - claude
  - cursor
  - windsurf
---

# Ion CLI for Agents

Operate the Ion skill manager programmatically using the `--json` flag.

## JSON Interface

All commands support `ion --json <command>`. The `--json` flag:

- Outputs structured JSON to stdout
- Disables all interactive prompts (TUI, confirmations)
- Uses a two-stage pattern: commands that need decisions return options (exit 2), you re-run with explicit flags

### Response Envelope

```json
{"success": true, "data": { ... }}
{"success": false, "action_required": "<type>", "data": { ... }}
{"success": false, "error": "message"}
```

### Exit Codes

- `0` — success
- `1` — error
- `2` — action required (re-run with explicit flags)

## Commands

### Initialize a project

```bash
# Discover available targets
ion --json project init
# Exit 2, returns: {"action_required": "target_selection", "data": {"available_targets": [...]}}

# Initialize with specific targets
ion --json project init --target claude
ion --json project init --target claude --target cursor
```

### Search for skills

```bash
ion --json search "code review"
# Returns: {"success": true, "data": [{"name": "...", "source": "...", "description": "...", "stars": N}, ...]}
```

Use the `source` field from results to install a skill.

### Add a skill

```bash
# Single skill — succeeds directly
ion --json add owner/repo/skill-name

# Single skill with validation warnings
ion --json add owner/repo/skill-name
# Exit 2 if warnings: {"action_required": "validation_warnings", "data": {"skill": "...", "warnings": [...]}}
# Re-run to accept:
ion --json add owner/repo/skill-name --allow-warnings

# Skill collection — returns discovered skills
ion --json add owner/repo
# Exit 2: {"action_required": "skill_selection", "data": {"skills": [{"name": "...", "status": "clean|warnings|error"}, ...]}}
# Install specific skills:
ion --json add owner/repo --skills skill-a,skill-b
ion --json add owner/repo --skills skill-a,skill-b --allow-warnings
```

### Install all from Ion.toml

```bash
ion --json add
# If warnings exist and --allow-warnings not set: exit 2
ion --json add --allow-warnings
```

### Remove a skill

```bash
# Preview what will be removed
ion --json remove skill-name
# Exit 2: {"action_required": "confirm_removal", "data": {"skills": ["skill-name"]}}

# Confirm removal
ion --json remove skill-name --yes
```

### List installed skills

```bash
ion --json skill list
```

### Show skill info

```bash
ion --json skill info skill-name
```

### Update skills

```bash
ion --json update          # update all
ion --json update skill-name  # update one
```

### Validate skills

```bash
ion --json skill validate
ion --json skill validate path/to/SKILL.md
```

### Configuration

```bash
ion --json config list
ion --json config get targets.claude
ion --json config set targets.claude .claude/skills
ion --json config list --project
```

### Cache management

```bash
ion --json cache gc
ion --json cache gc --dry-run
```

### Self management

```bash
ion --json self info      # version, target, exe path
ion --json self check     # check for updates
ion --json self update    # update ion itself
```

## Typical Agent Workflow

```bash
# 1. Initialize project if needed
ion --json project init --target claude

# 2. Search for relevant skills
ion --json search "testing"

# 3. Install skills from search results (use the "source" field)
ion --json add obra/skills/test-driven-development

# 4. Handle warnings if they come back (exit code 2)
ion --json add some/skill --allow-warnings

# 5. List what's installed
ion --json skill list

# 6. Remove a skill
ion --json remove old-skill --yes
```

## Key Flags

| Flag | Scope | Purpose |
|------|-------|---------|
| `--json` | Global | Structured JSON output, no prompts |
| `--allow-warnings` | `add` | Proceed despite validation warnings |
| `--skills a,b,c` | `add` | Select specific skills from a collection |
| `--yes` / `-y` | `remove` | Skip removal confirmation |
| `--target name` | `project init` | Specify targets non-interactively |
| `--force` | `project init`, `skill new` | Overwrite existing files |
