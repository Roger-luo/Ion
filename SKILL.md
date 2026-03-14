---
name: ion-cli
description: "Operate the Ion skill manager from CLI using --json flag for structured, non-interactive control of skill installation, search, and project management."
compatibility: "claude, cursor, windsurf"
---

# Ion CLI for Agents

Operate the Ion skill manager programmatically using the `--json` flag.

## JSON Interface

All commands support `ion --json <command>`. The `--json` flag:

- Outputs structured JSON to stdout
- Disables all interactive prompts (TUI, confirmations)
- Uses a two-stage pattern: commands that need decisions return options (exit 2), you re-run with explicit flags

### Response Envelope

Every response is one of three shapes:


**Success** (exit 0) — operation completed:
```json
{"success": true, "data": { ... }}
```

**Action required** (exit 2) — you must re-run with explicit flags:
```json
{"success": false, "action_required": "<type>", "data": { ... }}
```

**Error** (exit 1) — operation failed:
```json
{"success": false, "error": "message"}
```


## Commands with Examples

Each example below shows the exact command and its JSON output, so you can learn the input-output format.

### Initialize a project

Without `--target`, ion discovers available targets and asks you to choose:

```bash
$ ion --json project init
```
```json
{
  "success": false,
  "action_required": "target_selection",
  "data": {
    "available_targets": [
      {
        "detected": true,
        "name": "claude",
        "path": ".claude/skills"
      },
      {
        "detected": false,
        "name": "cursor",
        "path": ".cursor/skills"
      },
      {
        "detected": false,
        "name": "windsurf",
        "path": ".windsurf/skills"
      }
    ],
    "hint": "Re-run with --target flags to select targets"
  }
}
```

Re-run with explicit targets:

```bash
$ ion --json project init --target claude --target cursor
```
```json
{
  "success": true,
  "data": {
    "manifest": "Ion.toml",
    "targets": {
      "claude": ".claude/skills",
      "cursor": ".cursor/skills"
    }
  }
}
```

### Search for skills

```bash
$ ion --json search "code review"
```
```json
{
  "data": [
    {
      "description": "Automated code review skill",
      "name": "code-review",
      "registry": "github",
      "source": "obra/skills/code-review",
      "stars": 42
    },
    {
      "description": "Pull request review assistant",
      "name": "pr-reviewer",
      "registry": "skills.sh",
      "source": "acme/pr-reviewer",
      "stars": 18
    }
  ],
  "success": true
}
```

Use the `source` field from results to install a skill.

### Add a single skill

```bash
$ ion --json add obra/skills/code-review
```
```json
{
  "data": {
    "installed_to": ".agents/skills/code-review/",
    "name": "code-review",
    "targets": [
      "claude",
      "cursor"
    ]
  },
  "success": true
}
```

If the skill has validation warnings, you get exit 2 instead:

```bash
$ ion --json add acme/experimental-skill
```
```json
{
  "action_required": "validation_warnings",
  "data": {
    "skill": "experimental-skill",
    "warnings": [
      {
        "checker": "security",
        "message": "Skill requests shell access",
        "severity": "warning"
      }
    ]
  },
  "success": false
}
```

Re-run with `--allow-warnings` to accept:

```bash
$ ion --json add acme/experimental-skill --allow-warnings
```
```json
{
  "data": {
    "installed_to": ".agents/skills/experimental-skill/",
    "name": "experimental-skill",
    "targets": [
      "claude"
    ]
  },
  "success": true
}
```

### Add from a skill collection

When a repo contains multiple skills, ion lists them for you to choose:

```bash
$ ion --json add obra/skills
```
```json
{
  "action_required": "skill_selection",
  "data": {
    "skills": [
      {
        "name": "code-review",
        "status": "clean"
      },
      {
        "name": "test-driven-dev",
        "status": "clean"
      },
      {
        "name": "experimental",
        "status": "warnings",
        "warning_count": 2
      }
    ]
  },
  "success": false
}
```

Pick specific skills:

```bash
$ ion --json add obra/skills --skills code-review,test-driven-dev
```
```json
{
  "data": {
    "installed_to": ".agents/skills/code-review/",
    "name": "code-review",
    "targets": [
      "claude"
    ]
  },
  "success": true
}
```

### Install all from Ion.toml

```bash
$ ion --json add
```
```json
{
  "data": {
    "installed": [
      "code-review",
      "test-driven-dev"
    ],
    "skipped": [
      "pinned-skill"
    ]
  },
  "success": true
}
```

### Remove a skill

First call returns a confirmation prompt:

```bash
$ ion --json remove test-skill
```
```json
{
  "success": false,
  "action_required": "confirm_removal",
  "data": {
    "skills": [
      "test-skill"
    ]
  }
}
```

Confirm with `--yes`:

```bash
$ ion --json remove test-skill --yes
```
```json
{
  "success": true,
  "data": {
    "removed": [
      "test-skill"
    ]
  }
}
```

### List installed skills

```bash
$ ion --json skill list
```
```json
{
  "success": true,
  "data": []
}
```

### Show skill info

```bash
$ ion --json skill info code-review
```
```json
{
  "data": {
    "description": "Automated code review skill",
    "git_url": "https://github.com/obra/skills.git",
    "name": "code-review",
    "path": "code-review",
    "source": "obra/skills",
    "source_type": "Github"
  },
  "success": true
}
```

### Update skills

```bash
$ ion --json update
```
```json
{
  "data": {
    "failed": [],
    "skipped": [
      {
        "name": "pinned-skill",
        "reason": "pinned to refs/tags/v1.0"
      }
    ],
    "up_to_date": [
      {
        "name": "test-driven-dev"
      }
    ],
    "updated": [
      {
        "binary": false,
        "name": "code-review",
        "new_version": "v1.2.0",
        "old_version": "v1.1.0"
      }
    ]
  },
  "success": true
}
```

Update a single skill:

```bash
$ ion --json update code-review
```
```json
{
  "data": {
    "failed": [],
    "skipped": [],
    "up_to_date": [],
    "updated": [
      {
        "binary": false,
        "name": "code-review",
        "new_version": "v1.2.0",
        "old_version": "v1.1.0"
      }
    ]
  },
  "success": true
}
```

### Validate skills

```bash
$ ion --json skill validate
```
```json
{
  "success": true,
  "data": {
    "skills": [
      {
        "errors": 0,
        "findings": [],
        "infos": 0,
        "name": "test-skill",
        "path": "test-skill/SKILL.md",
        "warnings": 0
      }
    ],
    "total_errors": 0,
    "total_infos": 0,
    "total_warnings": 0
  }
}
```

### Configuration

```bash
$ ion --json config list
```
```json
{
  "success": true,
  "data": {
    "targets.claude": ".claude/skills",
    "targets.cursor": ".cursor/skills"
  }
}
```

```bash
$ ion --json config get targets.claude
```
```json
{
  "success": true,
  "data": {
    "key": "targets.claude",
    "value": ".claude/skills"
  }
}
```

```bash
$ ion --json config set targets.claude .claude/commands
```
```json
{
  "success": true,
  "data": {
    "key": "targets.claude",
    "value": ".claude/commands"
  }
}
```

### Cache management

```bash
$ ion --json cache gc --dry-run
```
```json
{
  "data": {
    "dry_run": true,
    "removed": []
  },
  "success": true
}
```

### Self management

```bash
$ ion --json self info
```
```json
{
  "data": {
    "exe": "/usr/local/bin/ion",
    "target": "aarch64-apple-darwin",
    "version": "0.2.1"
  },
  "success": true
}
```

```bash
$ ion --json self check
```
```json
{
  "data": {
    "installed": "0.2.0",
    "latest": "0.2.1",
    "update_available": true
  },
  "success": true
}
```

```bash
$ ion --json self update
```
```json
{
  "data": {
    "exe": "/usr/local/bin/ion",
    "new_version": "0.2.1",
    "old_version": "0.2.0",
    "updated": true
  },
  "success": true
}
```

## Typical Agent Workflow

Here is a complete example showing how to search for and install a skill:


```bash
# 1. Initialize project (if no Ion.toml exists)
$ ion --json project init --target claude
# → {"success": true, "data": {"targets": {"claude": ".claude/skills"}, "manifest": "Ion.toml"}}

# 2. Search for a skill
$ ion --json search "testing"
# → {"success": true, "data": [{"name": "test-driven-development", "source": "obra/skills/test-driven-development", ...}]}

# 3. Install it (use the "source" field from search results)
$ ion --json add obra/skills/test-driven-development
# → {"success": true, "data": {"name": "test-driven-development", "installed_to": ".agents/skills/test-driven-development/", "targets": ["claude"]}}

# 4. If exit code 2 with warnings, re-run with --allow-warnings
$ ion --json add some/skill --allow-warnings
# → {"success": true, "data": {"name": "some-skill", ...}}

# 5. Verify what's installed
$ ion --json skill list
# → {"success": true, "data": [{"name": "test-driven-development", "source": "obra/skills", ...}]}

# 6. Remove a skill when no longer needed
$ ion --json remove old-skill --yes
# → {"success": true, "data": {"removed": ["old-skill"]}}
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