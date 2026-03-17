---
title: JSON Mode
description: Use Ion programmatically with structured JSON output.
---

Every Ion command supports `--json` mode for structured, non-interactive output. This enables AI agents and CI scripts to operate Ion programmatically.

## Usage

```bash
ion --json <command>
```

The `--json` flag goes before the subcommand.

## Examples

### Search

```bash
ion --json search "testing"
```

Returns structured search results that agents can parse and act on.

### Two-Stage Commands

Some commands support a preview/execute pattern:

```bash
# Preview what would be removed (exit code 2)
ion --json remove my-skill

# Execute the removal (exit code 0)
ion --json remove my-skill --yes
```

### Skill Information

```bash
ion --json skill list
ion --json skill info my-skill
```

## Output Envelope

JSON output follows a consistent envelope format, making it reliable for automated parsing. Exit codes indicate the operation status:

- **0** — Success
- **1** — Error
- **2** — Preview/confirmation needed (use `--yes` to proceed)

## Use Cases

- **AI agents** operating Ion as a tool (e.g., via Claude's `ion-cli` skill)
- **CI/CD pipelines** that install and validate skills automatically
- **Scripts** that query skill metadata or check for updates
