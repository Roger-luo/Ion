---
name: dogfooding-ion
description: >-
  Use when hardening Ion's user experience by driving the ion CLI end-to-end
  against a real project and improving what feels rough — i.e. "dogfood ion on
  <project>", "use ion in X and fix the UX gaps", "is ion actually pleasant to
  use", "does ion guide the agent to the next step", or any shakedown of the CLI
  from a user's seat. Dogfooding judges three things at once: (1) human terminal
  UX — is plain (non-JSON) output friendly and readable; (2) progressive
  prompting — does every command's output (human text AND the --json envelope)
  make the next step obvious so a user or agent never stalls; (3) workflow
  ergonomics — does a real goal take too many commands, calling for a new or
  consolidated command rather than another hint. Covers driving the full skill
  lifecycle on a scratch repo and an established one, logging each rough edge,
  and FIXING the CLI (next-step hints, richer action_required envelopes,
  friendlier output, new/merged commands) with a failing-test-first discipline
  using the scenario crate + integration tests. Not for plain library changes
  verifiable by nextest alone — just run nextest.
allowed-tools: Bash, Read, Write, Edit
---

# Dogfooding Ion on a real project

## Overview

Unit tests prove the machinery works. Dogfooding proves Ion is *pleasant and
legible to drive* — for both the human at the keyboard and the agent operating
it programmatically. You use the real `ion` binary to accomplish real goals in a
real repo, notice every place you (or an agent) hesitate, guess, or run an
avoidable extra command, and then improve the CLI so the next person doesn't.

Judge three lenses at once. All three matter:

1. **Human terminal UX** — when a user runs a plain `ion <cmd>` (no `--json`),
   is the output *friendly to read*? Scannable, correctly emphasized, not noisy,
   colors and spacing carrying meaning rather than decoration. A wall of
   undifferentiated text, or a bare `Done!` with no orientation, is a gap.
2. **Progressive prompting** — does each command's output *guide the reader to
   the next step*, so neither a human nor an agent is ever left at a dead end?
   This spans **both channels**: the human text AND the `--json` envelope. After
   `ion init` you should know to run `ion add`; after `ion add` you should know
   the skill is live and how to use or commit it; when a command needs more
   input it should name the exact recovery command and flags. The brief: *the
   printings teach the next move as the command runs.*
3. **Workflow ergonomics** — does accomplishing one real goal take an awkward
   *sequence* of commands? If "start using team skills in my repo" is
   `init` → `search` → `add` → edit `.gitignore` → commit, ask whether a step
   should be folded in (a sensible default, an auto-init, a combined command) or
   a new command should exist. Some gaps are fixed with a better *sentence*;
   others with a better *command surface*. Don't paper a missing command over
   with another hint.

The loop:

```
build the ion binary under test → pick a goal a real user has → drive it end to
  end on a scratch repo AND an established repo, reading output as a human would
   → at each step judge the three lenses: friendly? next step obvious (text +
     JSON)? too many commands?
   → log the rough edge (what you saw, what was missing, who stalls)
   → FIX it: a friendlier line, a next-step hint, a richer action_required
     envelope, OR a new/consolidated command — failing test FIRST
   → rebuild → re-drive that step → confirm it now lands
   → repeat across the command surface until the journeys feel obvious
```

## When to use

- The user says "dogfood ion on <project>", "use ion for real and fix what's
  clunky", "make ion guide the agent through the next steps", or asks to shake
  out the CLI's UX.
- After changing command output, flags, or the JSON envelope, to confirm the
  end-to-end journeys still read well.
- To evaluate whether an agent could drive Ion *from the output alone* — the
  north star is that the `ion-cli` skill becomes almost unnecessary because the
  commands teach their own next steps.

Do **not** use for changes verifiable by `cargo nextest run` alone (pure library
logic, parsing, resolution). This skill is about the felt experience of using
the tool, which unit tests can't see.

## Setup

1. **Build the binary under test.** The point is to exercise *your* code, so
   invoke the freshly built debug binary — not whatever `ion` is on PATH:
   ```bash
   cargo build            # in the ion repo
   ION=$PWD/target/debug/ion   # invoke this, not `ion`
   ```
   **Rebuild after every fix.** A stale binary is the #1 way to "fix" a gap that
   the run still shows. (If you'd rather test the exact thing users run,
   `cargo install --path . --force` puts it on PATH as `ion` — but then you must
   reinstall after each change; the built-binary path is faster and has no stale
   trap.)

2. **Prepare two target repos** — you need both to see the whole journey:
   - **A scratch repo** for the *cold start* — the first-run experience, where
     next-step guidance matters most:
     ```bash
     mkdir /tmp/ion-dogfood-scratch && cd /tmp/ion-dogfood-scratch && git init
     ```
     Greenfield surfaces `init` → `search` → `add` → `new` → `validate`, and the
     "how do I even begin" moment a new user actually hits.
   - **An established repo** that already uses Ion (has `Ion.toml`/`Ion.lock`) —
     e.g. a sibling project like `autotune2`. This surfaces the *maintenance*
     journeys: install-all, `update`, `migrate`, `skill eject`, `remove`.
     **Work in a throwaway git worktree or a copy**, never the user's live
     checkout — dogfooding mutates `Ion.toml`, `.gitignore`, and target dirs.

3. **Read the intended agent contract.** Skim `.agents/skills/ion-cli/SKILL.md`
   — it documents the JSON interface an agent is *supposed* to rely on. Part of
   dogfooding is checking whether the CLI's actual output lets an agent succeed
   *without* that cheat sheet.

## Drive the lifecycle

Walk real goals, not a checklist of commands. For each command in a journey, run
it the way a **human** would (plain, no `--json`) and read the output as if you'd
never seen it — then separately inspect the **agent** channel (`--json`) where
the programmatic next step matters (anything that prompts, confirms, errors, or
returns IDs the next call needs). Two representative journeys:

**Journey A — cold start (scratch repo).** "I want to start using skills here."
1. `$ION init` — choose targets. Does it confirm what was created *and point to
   what's next* (`ion search` / `ion add`)? Or does it stop at a file list?
2. `$ION search "code review"` — do results tell you how to install one
   (`ion add <source>`), or just list names you must know what to do with?
3. `$ION add <source>` — does it confirm the skill is installed, where, to which
   targets, *and* what to do now (use it / add more / commit these files)? Or a
   bare `Done!`?
4. `$ION new --path .agents/skills/my-skill` — does it point to editing
   `SKILL.md` / validating / where the skill lives?
5. `$ION validate` — when there's a warning or error, does the message name the
   *fix* (which field, or `--allow-warnings`), not just the complaint?
6. `$ION list` — does empty read as "here's how to add one" rather than silence?

**Journey B — established repo (real, in a worktree).** "I inherited this repo."
1. `$ION add` (install-all from `Ion.toml`) — does the summary orient you
   (installed / skipped / pinned) and say what to do next?
2. `$ION update` — do up-to-date vs updated vs skipped(pinned) read clearly, and
   does it tell you how to act on each (e.g. how to update a pinned skill)?
3. `$ION migrate`, `$ION skill eject <name>`, `$ION remove <name>` — for each,
   does the output explain *what changed* and *what now*? Does `remove` say how
   to undo, and does its confirmation flow (`action_required` → `--yes`) read
   well in both channels?

At every step, capture the actual output (copy it into your findings) — you're
reviewing artifacts, not memories.

## The rubric — judge each step

For every command's output, ask this short set of questions. They're the
operational definition of the three lenses; a "no" is a finding.

**Orientation (human):** Reading only this output, do I know *what just
happened* — what state changed, where files landed?

**Readability (human):** Is it scannable? Is emphasis (bold names, cyan paths,
green success, yellow hints, dim secondary — see `src/style.rs`) carrying
meaning, or is it a flat/noisy block? Does it degrade gracefully when piped (no
color, no TTY)?

**Next step (both channels):** Does the output name the *natural next action*
with the *exact command* to run? And is that guidance present in **both** the
human text and the `--json` data — an agent reading JSON must not be told less
than a human reading text. Inconsistency between channels is itself a gap.

**Recovery (both channels):** If the command needs input, failed, or hit a
warning, does it name the exact recovery command and flags — `action_required`
in JSON *and* a plain-text equivalent for the human? An error that says what
broke but not what to do is only half-written.

**Path length (ergonomics):** How many commands did this *goal* take? If a
common goal needs a rote sequence every time, that's a signal to fold a step in
(a default, an auto-step) or add/consolidate a command — not to add a third
hint. Note the sequence you actually ran.

**Agent self-sufficiency (the north star):** Could an agent with *no* `ion-cli`
cheat sheet — only this output — construct the correct next call? If it would
have to guess a flag or command name, the printing isn't teaching enough.

## Log findings

Keep a running findings log (in the workspace, or inline for the user). One
entry per rough edge so fixes are traceable:

```
FINDING: `ion add <source>` ends at "Done!" with no next step.
  Lens: progressive-prompting + human UX
  Saw (human):  "Done!"
  Saw (--json): {"success":true,"data":{"name":"code-review","installed_to":"…","targets":["claude"]}}
  Missing: human text doesn't confirm where it landed or what to do next; JSON
    data has the facts but no next-step signal. A new user doesn't know the
    skill is now live for Claude, or that these files should be committed.
  Who stalls: new human user; agent has enough in JSON to proceed but nothing
    tells it the install is "usable now".
  Fix: after the summary, print the target paths + a "the skill is now available
    to <targets>; commit .agents/ and Ion.lock" line; mirror a `next` field or
    equivalent in JSON.
```

Separate the *small* fixes (a missing line, a clearer error) from *design*
findings (this goal needs a new/merged command). Design findings deserve a short
proposal — what the command would be, what it replaces, why the current sequence
is too long — before you build, since they change the CLI's surface.

## Fix — test-first

Once you've reproduced a gap, fix it with a failing test **first**, so the
improvement is pinned and can't silently regress. Ion gives you two testing
tools; pick by what you're asserting.

- **`scenario` crate** (`crates/scenario/`) — purpose-built to test CLI output
  under controlled terminal conditions: with/without a TTY, specific widths,
  color on/off, and interactive `expect`/`send_line` sessions. This is the right
  tool for *human-channel* assertions — "the output contains this next-step
  line", "the hint survives when piped (no color)", "the confirm prompt appears
  in a PTY". Write the scenario asserting the guidance you want, watch it fail
  against the current binary, then add the printing.
- **Integration tests** (`tests/`, invoking `env!("CARGO_BIN_EXE_ion")`) — for
  the *agent channel* and exit codes: assert the `--json` envelope shape, the
  `action_required` string, exit code 2 on needs-input, and that a `data` field
  the next call needs is present. Assert on both channels when a fix touches
  both (it usually should — see the consistency rubric).

Where the printings live:
- **Human text & hints** — per-command `println!` in `src/commands/<name>.rs`,
  styled via `crate::style::Paint` (`bold`/`success`/`info`/`warn`/`dim`).
  Reusable hints follow the model of `print_no_targets_hint` /
  `print_codex_hint` in `src/commands/init.rs` — factor a shared hint fn rather
  than copy-pasting a line into every command.
- **Agent envelope** — `src/json.rs`: `print_success` (exit 0),
  `print_action_required(action, data)` (exit 2), `print_error` / `print_failure`
  (exit 1). Enriching the agent's next-step signal usually means adding to the
  `data` payload or introducing/strengthening an `action_required` case.
- **New / consolidated commands** — the clap `Commands` enum and dispatch in
  `src/main.rs`, with the implementation in a new `src/commands/<name>.rs`. A
  consolidation may instead mean changing a default or chaining an existing
  step; prefer the smallest change that removes the extra command.

Then green the checklist (CI enforces all three):
```bash
cargo fmt --all
cargo clippy --all-targets --all-features -- -D warnings
cargo nextest run
```
Rebuild the binary and re-drive the exact step to confirm the guidance now lands
for a first-time reader — not just that the test passes.

## Common gap signatures

Recognize these fast — each is a recurring shape, not a one-off:

| What you see | Lens | Usual fix |
|---|---|---|
| Command ends at bare `Done!` / a file list, no next action | prompting + human | append a next-step line naming the exact command; mirror in JSON |
| `action_required` in `--json`, but the plain-text run doesn't say how to recover | prompting (channel mismatch) | add the human-readable recovery line alongside the existing action |
| Error prints *what* broke, not *what to do* | human + prompting | name the fix command/flag in the message (e.g. `--allow-warnings`, which field to edit) |
| `search` lists results but never says `ion add <source>` | prompting | print the install command using the `source` field of each result |
| `--json` `data` lacks a field the next call needs (agent must guess) | agent | add the identifier to the payload so the next call is constructible |
| Nice colored hint vanishes / becomes noise when piped (no TTY) | human | route through `Paint` (already TTY-aware); assert the plain form in a `scenario` no-TTY test |
| A routine goal always takes the same 3-4 commands in order | ergonomics | fold a step into a default/auto-step, or add/consolidate a command — not another hint |
| Human text and JSON tell the reader *different* next steps | prompting (consistency) | make one source of truth for the next-step and render it to both |
| Output is a flat block; names/paths/status not visually distinct | human | apply `Paint` emphasis so the eye finds the skill name, the path, the outcome |

## Confirmed & fixed (first pass, 2026-07)

The first dogfooding pass surfaced these concrete gaps and fixed them (commit
`753f3af`). Recognize a *regression* of any of these instantly — and treat the
list as "already covered," so a fresh pass spends its effort on new territory
rather than re-deriving them.

| Symptom | Lens | Fix + where it lives |
|---|---|---|
| `ion --json add` (and install-all) printed human progress lines *before* the JSON envelope → `ion --json add \| jq` failed to parse | agent | gate every progress `println!` behind `if !json`; fold local skills into the JSON `installed`/`skipped` arrays — `src/commands/add.rs`, `src/commands/install.rs` |
| `--allow-warnings` was honored only under `--json`; in human/non-TTY mode the documented recovery flag silently did nothing (prompt fired, empty stdin aborted) | prompting (channel mismatch) | short-circuit the flag in both channels across single-skill / collection / install-all paths; name it in the prompt, the cancel error, and the JSON `hint` — `install_shared.rs`, `add.rs`, `install.rs` |
| `ion init` with no `--target` and no TTY crashed with raw `Device not configured (os error 6)`, while `--json` returned a clean `action_required` | prompting (channel mismatch) | guard the interactive selector with `IsTerminal`; print the target list + `--target` hint and exit non-zero — `src/commands/init.rs` |
| `ion search` panicked (`byte index N is not a char boundary`) truncating a multi-byte (CJK) description | human (robustness) | snap truncation to a char boundary — `src/commands/search.rs` `truncate_for_ellipsis` |
| `SkillSource::infer` misparsed `ssh://` / `git://` / `file://` / scp-style `git@host:owner/repo` remotes as GitHub `owner/repo` shorthand | functional | recognize the standard git transports before the shorthand parse — `crates/ion-skill/src/source.rs` |
| `ion init` didn't link already-installed skills to a *newly*-configured target — they stayed invisible to that tool, no error | ergonomics (dead-end) | re-`deploy()` tracked skills after a target is added — `src/commands/init.rs` |
| `ion list` rendered local skills as `vunknown` with a blank `source:` | human | show `(local)` / `source: local` — `src/commands/list.rs` |

## Confirmed & fixed (second pass, 2026-07 — `ion init` journey)

Driving `ion init` on a real project (a repo that already had `.claude/` and
the built-in `ion-cli` deployed but no manifest) surfaced these and fixed them.

| Symptom | Lens | Fix + where it lives |
|---|---|---|
| `ion init` ended at a bare "Created Ion.toml with N target(s)" file-list with **no next step** — neither the human text nor the `--json` envelope told the reader to add a skill | prompting (both channels) | state-aware next-step: fresh project → `ion add <source>` / `ion search <query>`; skills already declared → `ion add`. One source of truth (`next_steps_after_init`) rendered to human text *and* a new JSON `data.next` array — `src/commands/init.rs` |
| No-TTY `ion init` (no `--target`) exposed a per-target `detected` flag to the agent in `--json`, but the human plain-text target list gave no such signal and the recovery example hard-coded `--target claude` | prompting (channel mismatch) | mark `(detected)` on targets whose dir already exists in the repo, and prefer a detected target in the `--target <name>` example — `src/commands/init.rs` |

Note on `ion-cli = { type = "local" }`: the built-in skill is *intentionally*
registered as `local` (see `builtin_skill::ensure_installed`) so it's skipped by
fetch/validate/update and refreshed via `refresh_global` — not a bug. Callers
reasoning about *user* skills should exclude `builtin_skill::SKILL_NAME` (the
init next-step counter now does).

## Confirmed & fixed (third pass, 2026-07 — `ion init` AGENTS.md handling)

`ion init` only offered AGENTS.md setup interactively (TTY-gated) *and* only
when a language template was detected — so on a piped/agent run, or a project
with no recognized stack, it silently created no AGENTS.md and never migrated an
existing CLAUDE.md. Reworked so init *always* ensures an ion-managed AGENTS.md.

| Symptom | Lens | Fix + where it lives |
|---|---|---|
| `ion init` created no AGENTS.md unless run in a TTY *and* a language template matched; agent/CI runs and generic projects got nothing | ergonomics (dead-end) | always ensure AGENTS.md in every channel; new `builtin:generic` fallback template — `crates/ion-skill/src/templates/generic.md`, `templates.rs`, `src/commands/init.rs` `ensure_agents_md` |
| `ion init` never migrated an existing `CLAUDE.md` (only `ion migrate` did) — a repo with hand-written CLAUDE.md stayed unmanaged after init | ergonomics | init reconciles CLAUDE.md silently: rename→AGENTS.md + symlink back, or skip a genuine two-file conflict with an init-appropriate note — `ensure_agents_md` + `migrate_claude_md` gains a `rename_without_prompt` param |
| After scaffolding from a template, nothing told the human/agent the file has placeholders to fill in | prompting (both channels) | when a template was applied, the first `next` step (human list + JSON `data.next`) is "Fill in AGENTS.md …" — `next_steps_after_init` keyed on `AgentsMdOutcome::created_from_template()` |
| init's JSON success lacked any AGENTS.md signal for agents | agent | added `data.agents_md` = `{action: created\|migrated\|existing\|skipped\|disabled, template?}` mirroring the human summary — `src/commands/init.rs`, contract doc `skills/ion-cli/SKILL.md.j2` + `build.rs` |

Design notes: `agents init`'s file-work was extracted into a non-printing
`agents::apply_template` so both `agents init` and `ion init` share it without
double-emitting a JSON envelope (the first-pass purity trap). An existing
hand-written AGENTS.md is made ion-managed by symlink only — no `[agents]`
template is attached to the user's own content (that would produce a misleading
`ion agents diff`). `--no-agents` opts out of the whole thing. `ion migrate`'s
conservative `--yes` behavior (skip the rename) is preserved via the new param.

Still open (documented, not yet fixed): `ion add`/`ion new` cold start still end
without a next-step line (only `init` is now covered), empty `ion list` gives no
pointer to `ion init`, and `ion migrate` re-prompts for skills already registered
as local. These remain the natural next targets — the *progressive-prompting*
core — and want careful, general next-step logic (that respects actual state),
not a quick hard-coded line.

## Guidelines

- **Read as a stranger.** The whole value is noticing where *you* hesitate. If
  you only knew what this one command printed, would you know what to do? Trust
  that hesitation — it's the finding.
- **Both channels, always.** A fix that helps the human but leaves `--json`
  silent (or vice versa) is half a fix. The agent and the human deserve the same
  next step, phrased for their medium.
- **Hints are seasoning, not the meal.** More hints ≠ better UX. A command
  drowning in advisory lines is as hard to read as one with none. Prefer *one*
  clear next step. When the real problem is "too many commands," fix the command
  surface, not the prose.
- **Prefer the smallest change that removes the friction.** A missing sentence
  beats a new flag; a better default beats a new command. Reach for new surface
  only when a sequence is genuinely a recurring tax.
- **Pin every fix with a test.** Output is easy to regress silently. A
  `scenario` or integration test for each next-step line / envelope field / exit
  code keeps the UX from eroding as the code changes.
- **Don't overfit to one journey.** You iterate on two repos, but the guidance
  ships to every user. Write next-step logic that's correct in general (respects
  actual state — targets configured or not, pinned vs updatable), not a hard-coded
  line that lies in some states.
- **Stay out of the user's live repo.** Established-repo dogfooding runs in a
  worktree or copy; it mutates manifests, `.gitignore`, and target dirs.
- **Capture new signatures.** When you fix a non-obvious rough edge, add a row to
  the table above so the next dogfooding pass recognizes it instantly.
