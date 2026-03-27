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
- `path` specifies where `AGENTS.md` lives within the source repo. Defaults to `AGENTS.md` at the repo root. Supports repos that bundle templates alongside skills. Note: unlike skill `path` (which resolves to a directory containing `SKILL.md`), this `path` resolves to a specific file.
- When no `[agents]` section exists, Ion manages symlinks only (no template tracking).

**Rust struct — add to `Manifest`:**

```rust
/// Parsed from [agents] in Ion.toml
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AgentsConfig {
    pub template: Option<String>,
    pub rev: Option<String>,
    pub path: Option<String>,  // default: "AGENTS.md"
}

// In Manifest:
pub struct Manifest {
    // ... existing fields ...
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agents: Option<AgentsConfig>,
}
```

**Source resolution:** `AgentsConfig` is NOT a `SkillEntry`. To resolve the source, construct a `SkillSource` manually from the `template` string using the existing `SourceType::parse()` / source resolution utilities in `ion-skill`. Do not route through `SkillEntry::resolve()`. The fetch path (git clone, HTTP download) is shared; only the extraction differs — skills extract a directory, agents extract a single file.

### Ion.lock — `[agents]` entry

```toml
[agents]
template = "org/agents-templates"
rev = "abc123def"                    # resolved commit hash
checksum = "sha256:..."              # hash of the fetched AGENTS.md
updated-at = "2026-03-27T00:00:00Z"  # ISO 8601 string
```

Tracks the last-synced state so `ion update` can detect upstream changes.

**Rust struct — add to `Lockfile`:**

```rust
/// Lock entry for the agents template
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentsLockEntry {
    pub template: String,
    pub rev: Option<String>,
    pub checksum: String,
    pub updated_at: String,  // ISO 8601, stored as plain string
}

// In Lockfile:
pub struct Lockfile {
    // ... existing fields ...
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agents: Option<AgentsLockEntry>,
}
```

The `Option` type with `skip_serializing_if` ensures backward compatibility — existing Ion.lock files without `[agents]` deserialize without error.

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

Note: symlinks are committed to git. On Windows, git symlinks require Developer Mode or elevated permissions. Ion does not currently support Windows (release targets are macOS and Linux only), so no Windows-specific handling is needed. Revisit if Windows support is added.

### Symlink creation helper

Extract symlink creation into a shared helper function (`ensure_agent_symlinks`) so it can be called from multiple code paths:

```rust
/// For each configured target that has an entry in AGENT_FILE_SYMLINKS,
/// create the symlink if AGENTS.md exists and the symlink doesn't.
fn ensure_agent_symlinks(project_dir: &Path, targets: &BTreeMap<String, String>) -> Result<()>
```

This function is called from: `ion init`, `ion add` (install-all), and `ion agents init`.

Symlinks are only created for targets the user has configured in `[options.targets]` (or global config). If a user uses Claude but hasn't added `claude` as a target, no `CLAUDE.md` symlink is created. This is intentional — the target configuration is the source of truth for which tools the project uses. To get the symlink, add the target via `ion init --target claude` or manually in Ion.toml.

### Behavior during `ion init`

1. After writing Ion.toml and setting up target directories, call `ensure_agent_symlinks`.
2. If `AGENTS.md` doesn't exist, no symlinks are created.
3. If the target filename already exists as a regular file (not a symlink), warn and skip. Do not clobber.
4. If the target filename exists as a symlink pointing elsewhere, warn and skip.

### Behavior during `ion add` (install-all)

Call `ensure_agent_symlinks` after skill installation completes. Insert the call after the install loop and before the final summary output. Acts as a repair mechanism if someone deleted the symlink.

### No dedicated command

Symlink creation is automatic, part of the init, install, and agents-init flows. Not user-invoked.

## Template Sourcing

### Command: `ion agents init`

```bash
ion agents init org/agents-templates                          # from GitHub
ion agents init --rev v2.0 org/agents-templates               # pinned
ion agents init --path templates/AGENTS.md org/agents-templates  # custom path in repo
ion agents init ./path/to/template                            # local path
```

### Flow

1. Resolve the source by constructing a `SkillSource` from the template string (using `SourceType::parse()` and the same alias/shorthand expansion from `GlobalConfig`).
2. Fetch the repository via the shared git/HTTP fetch infrastructure.
3. Extract the single `AGENTS.md` file from the configured path (default: repo root). Error if the file doesn't exist: `AGENTS.md not found in <source>`.
4. If local `AGENTS.md` doesn't exist: copy upstream as the starting point.
5. If local `AGENTS.md` already exists: copy upstream to `.agents/templates/AGENTS.md.upstream` and inform the user they can merge.
6. Write `[agents]` section to Ion.toml (using `toml_edit` to preserve formatting, similar to `manifest_writer`).
7. Write lock entry to Ion.lock with resolved rev and checksum.
8. Add `.agents/templates/AGENTS.md.upstream` to `.gitignore` (managed-by-ion section). Use the specific file path, not the directory, to avoid gitignoring other user content in `.agents/templates/`.
9. Call `ensure_agent_symlinks` — so symlinks are created immediately if the user just set up a template (without requiring a separate `ion init`).

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

Ion fetches the repo using the same git clone/cache infrastructure as skill installation. When the template source matches an installed skill's source, no duplicate clone occurs — the cached repo is reused. The `path` field handles cases where the template lives in a subdirectory.

## Template Update Flow

### Detection during `ion update`

Agents template update runs **after** skill updates complete. This order ensures that if both skills and the template come from the same repo, the repo is already cached.

1. If `[agents]` exists in Ion.toml with a `template`, fetch the latest from the source.
2. Compare the fetched `AGENTS.md` against the checksum in Ion.lock.
3. If unchanged: no-op (print nothing for agents).
4. If changed: write the new version to `.agents/templates/AGENTS.md.upstream`, update Ion.lock with new rev/checksum, and print:

```
agents: upstream template updated (abc123 -> def456)
  upstream saved to .agents/templates/AGENTS.md.upstream
  run your agent to merge, or manually diff:
    diff AGENTS.md .agents/templates/AGENTS.md.upstream
```

A template fetch failure is non-fatal for `ion update` — print a warning and continue. The exit code of `ion update` is determined by skill update results only. This matches the principle that the template is advisory, not blocking.

### Dedicated command: `ion agents update`

Runs just the agents template update without updating skills. A fetch failure here IS fatal (non-zero exit).

### Manual merge path

The user can:
- Run `ion agents diff` to see what changed.
- Manually edit `AGENTS.md`.
- Ask their agent to do the merge.

### `ion agents diff` behavior

- If `.agents/templates/AGENTS.md.upstream` exists: run `diff AGENTS.md .agents/templates/AGENTS.md.upstream` and display the output.
- If `.agents/templates/AGENTS.md.upstream` does not exist: error: `no upstream template staged, run 'ion agents update' first`.
- If local and upstream are identical: print `AGENTS.md is up to date with upstream`.

### Built-in skill: `agents-update`

Ion ships an `agents-update` skill as a SKILL.md embedded in the binary (same mechanism as `ion-cli`). It is deployed only when `[agents]` is configured in Ion.toml — specifically, `ion agents init` deploys it, and `ensure_builtin_skill` checks for it during `ion init` / `ion add` only if `manifest.agents` is `Some`. This avoids deploying an irrelevant skill in projects without template tracking.

The skill instructs the agent to:

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
| `ion init` | After target setup, call `ensure_agent_symlinks` |
| `ion update` | After skill updates, check for upstream template changes |
| `ion add` (install-all) | After skill installation, call `ensure_agent_symlinks` |

## Edge Cases and Error Handling

### Symlink conflicts

- Target filename already exists as a regular file: warn, don't clobber. Print: `CLAUDE.md already exists as a file, skipping symlink (remove it manually if you want ion to manage it)`
- Target filename exists as a symlink pointing elsewhere: warn, skip.
- `AGENTS.md` doesn't exist when init runs: no symlinks created.

### Template conflicts

- `ion agents init` when `[agents]` already exists: error: `template already configured, use ion agents update`. To switch templates, the user should manually edit `[agents]` in Ion.toml and then run `ion agents update`.
- Template repo doesn't contain `AGENTS.md` at the configured path: error: `AGENTS.md not found in <source>`
- Network failure during fetch: error with standard retry messaging.

### Lock/state consistency

- Ion.lock has `[agents]` but Ion.toml doesn't: stale lock entry, ignore it.
- Ion.toml has `[agents]` but no lock entry: treat as first fetch on next `ion agents update`.
- `.agents/templates/AGENTS.md.upstream` exists but is stale: overwritten on next update.

### Git hygiene

- `.agents/templates/AGENTS.md.upstream` is gitignored (staged upstream file is transient).
- `AGENTS.md` is NOT gitignored (user-owned, committed to git).
- `CLAUDE.md` symlink IS committed to git (teammates get it too).
