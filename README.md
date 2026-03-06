# Ion

Agent skill manager for AI coding tools (Claude, Cursor, Windsurf, etc.).

Ion manages installation, validation, and organization of **skills** — reusable prompts and instructions that enhance AI agent capabilities.

## Installation

```bash
cargo install --git https://github.com/Roger-luo/Ion
```

## Quick Start

```bash
# Initialize a project with a target
ion project init --target claude

# Add a skill
ion add anthropics/skills/brainstorming

# Install all skills from Ion.toml
ion add

# Search for skills
ion search "code review"

# Create a new local skill
ion skill new

# List installed skills
ion skill list
```

## Commands

| Command | Description |
|---------|-------------|
| `ion add [source]` | Add a skill, or install all from Ion.toml |
| `ion remove <name>` | Remove a skill |
| `ion search <query>` | Search registries and GitHub |
| `ion update [name]` | Update skills to latest versions |
| `ion run <name>` | Run a binary skill |
| `ion skill new` | Create a new skill |
| `ion skill eject <name>` | Eject a remote skill into an editable local copy |
| `ion skill validate` | Validate skill definitions |
| `ion skill list` | List installed skills |
| `ion skill info <name>` | Show skill details |
| `ion skill link <path>` | Link a local skill directory |
| `ion project init` | Initialize Ion.toml with targets |
| `ion project migrate` | Migrate from legacy formats |
| `ion cache gc` | Clean up stale cached repos |
| `ion config` | View and set configuration |

## License

MIT
