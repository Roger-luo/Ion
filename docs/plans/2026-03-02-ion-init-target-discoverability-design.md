# Design: `ion init` and Target Discoverability

## Problem

Ion supports configuring alternative install targets (e.g. `.claude/skills`, `.cursor/skills`) via `[options.targets]` in `Ion.toml` or `[targets]` in global config. However, users don't know this exists — there's no onboarding flow, no help text during `ion add`, and the section name isn't intuitive.

## Solution

Two complementary touchpoints:

1. **`ion init` command** — interactive project setup that configures targets
2. **Contextual hints during `ion add`** — nudge users toward `ion init` when no targets are configured

## `ion init` Command

### Interactive mode (default)

```
$ ion init
Detected: .claude/ .cursor/

Which tools do you use?
  [x] Claude Code (.claude/skills)
  [ ] Cursor (.cursor/skills)
  [ ] Windsurf (.windsurf/skills)
  [ ] Custom path

Created Ion.toml with 1 target:
  claude → .claude/skills
```

- Scans current directory for known tool directories (`.claude/`, `.cursor/`, `.windsurf/`)
- Pre-checks any that are detected
- User toggles with arrow keys / space
- "Custom path" lets them type a name and relative path
- Writes `Ion.toml` with `[options.targets]` section

### Flag mode (scriptable)

```
$ ion init --target claude              # uses built-in default path
$ ion init --target claude:.claude/commands/skills  # custom path
$ ion init --target claude --target cursor          # multiple
```

- `--target <name>` uses the built-in lookup table
- `--target <name>:<path>` overrides the default path
- Skips all prompts

### Known targets lookup table

| Name       | Default path       |
|------------|--------------------|
| `claude`   | `.claude/skills`   |
| `cursor`   | `.cursor/skills`   |
| `windsurf` | `.windsurf/skills` |

### Conflict handling

**Ion.toml exists, no `[options.targets]`:**
- Add the `[options.targets]` section to the existing file
- Preserve all existing content
- Print: `Updated Ion.toml with targets`

**Ion.toml exists with `[options.targets]`:**
- Interactive: show current targets, ask to replace or merge
- Flag mode: error unless `--force` is passed

**Legacy `ion.toml` exists (lowercase):**
- Rename `ion.toml` → `Ion.toml`, then proceed
- Print: `Renamed ion.toml → Ion.toml`
- If both `ion.toml` and `Ion.toml` exist: error asking user to remove one

**Legacy `ion.lock` exists:**
- Rename `ion.lock` → `Ion.lock`

## Contextual Hints During `ion add`

When no `[options.targets]` is configured (neither project nor global):

```
$ ion add anthropics/skills/brainstorming
  Resolved brainstorming from anthropics/skills
  Installed to .agents/skills/brainstorming
  Updated Ion.toml
  Updated Ion.lock

  hint: skills are only installed to .agents/skills/ (the default location)
        To also install to .claude/skills/ or other tools, run: ion init
```

- Only shown when the merged targets map is empty
- Printed after install succeeds (non-blocking)
- Not shown with `--quiet`
- Shown every time targets are empty (persistent nudge until configured)

## Error Handling

- No git repo required — `ion init` works in any directory
- No tool directories detected and no flags → show picker with nothing pre-checked; if user selects nothing, create Ion.toml with empty `[skills]` and no targets
- Absolute target paths → reject with `Target paths must be relative to the project directory`

## Testing

- Unit tests for known-targets lookup table
- Integration tests:
  - `ion init` creates Ion.toml with correct targets
  - `ion init --target claude` flag mode
  - `ion init` with existing Ion.toml (merge targets into existing file)
  - `ion init` with legacy `ion.toml` (rename + merge)
  - Hint shown during `ion add` when no targets configured
  - Hint not shown when targets are configured

## Scope boundaries

No changes to:
- `ion install`, `ion link`, `ion remove` — already use merged targets correctly
- The canonical `.agents/skills/` install path
- Global config format
