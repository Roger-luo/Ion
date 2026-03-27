# AGENTS.md Management

Ion feature for managing `AGENTS.md` template sourcing, upstream syncing, and agent-tool symlinks.

## Problem

Organizations want to share standard `AGENTS.md` templates across repositories. Users need to fork these templates, customize them, and incorporate upstream changes over time. Additionally, some agent tools (e.g. Claude) use differently-named files (`CLAUDE.md`) instead of `AGENTS.md`, requiring symlink management.

## Approach: Fork Model

The user gets a copy of the upstream template and owns it. When the upstream changes, Ion stages the new version and the user (or their agent) merges changes into their local copy. This is the fork model — no overlays, no managed sections.

## Data Model

### Ion.toml — `[agents]` section

```toml
[agents]
template = "org/agents-templates"   # GitHub source (same resolution as skills)
rev = "v2.0"                         # optional: pin to a specific revision
path = "templates/AGENTS.md"         # optional: path within repo (default: "AGENTS.md")
```

- `template` uses the same source resolution as skills (GitHub shorthand, full Git URLs, HTTP, local path).
- `rev` optionally pins the template to a specific revision.
- `path` specifies where `AGENTS.md` lives within the source repo. Defaults to `AGENTS.md` at the repo root. Supports repos that bundle templates alongside skills.
- When no `[agents]` section exists, Ion manages symlinks only (no template tracking).

### Ion.lock — `[agents]` entry

```toml
[agents]
template = "org/agents-templates"
rev = "abc123def"                    # resolved commit hash
checksum = "sha256:..."              # hash of the fetched AGENTS.md
updated-at = "2026-03-27T..."        # last fetch timestamp
```

Tracks the last-synced state so `ion update` can detect upstream changes.

### File layout

```
project/
  AGENTS.md                              # user's file (owned, committed to git)
  CLAUDE.md -> AGENTS.md                 # symlink (created by ion init)
  .agents/
    templates/
      AGENTS.md.upstream                 # latest fetched upstream template (gitignored)
  Ion.toml
  Ion.lock
```

`.agents/templates/AGENTS.md.upstream` is the staging area for upstream changes. It is gitignored and managed entirely by Ion.

## Symlink Management

### Hardcoded mapping

```rust
const AGENT_FILE_SYMLINKS: &[(&str, &str)] = &[
    ("claude", "CLAUDE.md"),
];
```

Only `claude` needs a symlink today. Other targets (`cursor`, `windsurf`) read `AGENTS.md` natively. New entries are added to this table as needed.

### Behavior during `ion init`

1. After writing Ion.toml and setting up target directories, check if `AGENTS.md` exists in the project root.
2. For each configured target, check `AGENT_FILE_SYMLINKS` for a match.
3. If a match exists and the symlink doesn't already exist, create it (e.g. `CLAUDE.md -> AGENTS.md`).
4. If the target filename already exists as a regular file (not a symlink), warn and skip. Do not clobber.

### Behavior during `ion add` (install-all)

Same symlink check runs after skill installation. Acts as a repair mechanism if someone deleted the symlink.

### No dedicated command

Symlink creation is automatic, part of the init and install flows. Not user-invoked.

## Template Sourcing

### Command: `ion agents init`

```bash
ion agents init org/agents-templates                          # from GitHub
ion agents init --rev v2.0 org/agents-templates               # pinned
ion agents init --path templates/AGENTS.md org/agents-templates  # custom path in repo
ion agents init ./path/to/template                            # local path
```

### Flow

1. Resolve the source (reuse existing `SkillSource` resolution).
2. Fetch the repository, find `AGENTS.md` at the configured path (default: repo root).
3. If local `AGENTS.md` doesn't exist: copy upstream as the starting point.
4. If local `AGENTS.md` already exists: copy upstream to `.agents/templates/AGENTS.md.upstream` and inform the user they can merge.
5. Write `[agents]` section to Ion.toml.
6. Write lock entry to Ion.lock with resolved rev and checksum.
7. Add `.agents/templates/` to `.gitignore` (managed-by-ion section).

### Shared repos

An org can bundle templates and skills in one repo:

```
myorg/agent-standards/
  AGENTS.md
  code-review/
    SKILL.md
  testing-standards/
    SKILL.md
```

Ion fetches the repo (which may already be cached from skill installs) and extracts `AGENTS.md`. The `path` field handles cases where the template lives in a subdirectory.

## Template Update Flow

### Detection during `ion update`

1. If `[agents]` exists in Ion.toml with a `template`, fetch the latest from the source.
2. Compare the fetched `AGENTS.md` against the checksum in Ion.lock.
3. If unchanged: no-op.
4. If changed: write the new version to `.agents/templates/AGENTS.md.upstream`, update Ion.lock with new rev/checksum, and print:

```
agents: upstream template updated (abc123 -> def456)
  upstream saved to .agents/templates/AGENTS.md.upstream
  run your agent to merge, or manually diff:
    diff AGENTS.md .agents/templates/AGENTS.md.upstream
```

### Dedicated command: `ion agents update`

Runs just the agents template update without updating skills.

### Manual merge path

The user can:
- Run `ion agents diff` to see what changed.
- Manually edit `AGENTS.md`.
- Ask their agent to do the merge.

### Built-in skill: `agents-update`

Ion ships an `agents-update` skill (deployed alongside `ion-cli` during init). This is a SKILL.md with instructions for the agent — no binary, no code. It tells the agent:

1. Read `.agents/templates/AGENTS.md.upstream` (new upstream version).
2. Read `AGENTS.md` (current local version).
3. Intelligently merge: incorporate upstream changes while preserving local customizations.
4. Write the updated `AGENTS.md`.

## Command Structure

### New subcommand group: `ion agents`

| Command | Description |
|---|---|
| `ion agents init <source>` | Set up template sourcing (fetch, copy/stage, write Ion.toml) |
| `ion agents update` | Fetch latest upstream, stage to `.agents/templates/AGENTS.md.upstream` |
| `ion agents diff` | Diff local `AGENTS.md` vs staged upstream |

### Modified existing commands

| Command | Change |
|---|---|
| `ion init` | After target setup, create symlinks (`CLAUDE.md -> AGENTS.md`) if `AGENTS.md` exists |
| `ion update` | Also checks for upstream template changes (in addition to skill updates) |
| `ion add` (install-all) | Ensures symlinks are in place after installation |

## Edge Cases and Error Handling

### Symlink conflicts

- Target filename already exists as a regular file: warn, don't clobber. Print: `CLAUDE.md already exists as a file, skipping symlink (remove it manually if you want ion to manage it)`
- Target filename exists as a symlink pointing elsewhere: warn, skip.
- `AGENTS.md` doesn't exist when init runs: no symlinks created.

### Template conflicts

- `ion agents init` when `[agents]` already exists: error: `template already configured, use ion agents update`
- Template repo doesn't contain `AGENTS.md` at the configured path: error: `AGENTS.md not found in <source>`
- Network failure during fetch: error with standard retry messaging.

### Lock/state consistency

- Ion.lock has `[agents]` but Ion.toml doesn't: stale lock entry, ignore it.
- Ion.toml has `[agents]` but no lock entry: treat as first fetch on next `ion agents update`.
- `.agents/templates/AGENTS.md.upstream` exists but is stale: overwritten on next update.

### Git hygiene

- `.agents/templates/` is gitignored (upstream staging area is transient).
- `AGENTS.md` is NOT gitignored (user-owned, committed to git).
- `CLAUDE.md` symlink IS committed to git (teammates get it too).
