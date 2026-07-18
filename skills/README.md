# skills/

Version-controlled home for the skills this project authors and maintains.

| Skill | Role | How it reaches an agent |
|-------|------|-------------------------|
| [`ion-cli`](ion-cli/SKILL.md.j2) | **Shipped with the binary.** Teaches an agent Ion's `--json` interface. | `SKILL.md.j2` is a [minijinja](https://docs.rs/minijinja) template rendered at build time by `build.rs` (JSON examples generated programmatically), embedded into the binary via `include_str!`, and deployed on `ion init` / `ion add` as the built-in skill (`src/builtin_skill.rs`). Edit the template here — never the deployed copy under `.agents/skills/ion-cli/`. |
| [`dogfooding-ion`](dogfooding-ion/SKILL.md) | **Dev workflow.** Drives the `ion` CLI end-to-end against a real project to find and fix UX / progressive-prompting / ergonomics gaps. | Not shipped. Registered in `Ion.toml` as a `path` skill (`dogfooding-ion = "skills/dogfooding-ion"`); `ion add` deploys the `.agents/skills/` symlink that makes it discoverable in a Claude Code session. |

## Restoring the dev-discovery symlink

`.agents/skills/` is gitignored, so a fresh clone won't have the `dogfooding-ion`
symlink that makes the skill discoverable in a Claude Code session. Since the
skill is tracked in `Ion.toml`, ion recreates it — just run install-all from the
repo root:

```bash
ion add
```

(The `ion-cli` skill needs no entry — the binary deploys it from its embedded
copy.)
