# skills/

Version-controlled home for the skills this project authors and maintains.

| Skill | Role | How it reaches an agent |
|-------|------|-------------------------|
| [`ion-cli`](ion-cli/SKILL.md.j2) | **Shipped with the binary.** Teaches an agent Ion's `--json` interface. | `SKILL.md.j2` is a [minijinja](https://docs.rs/minijinja) template rendered at build time by `build.rs` (JSON examples generated programmatically), embedded into the binary via `include_str!`, and deployed on `ion init` / `ion add` as the built-in skill (`src/builtin_skill.rs`). Edit the template here — never the deployed copy under `.agents/skills/ion-cli/`. |
| [`dogfooding-ion`](dogfooding-ion/SKILL.md) | **Dev workflow.** Drives the `ion` CLI end-to-end against a real project to find and fix UX / progressive-prompting / ergonomics gaps. | Not shipped. For local discovery it's symlinked into `.agents/skills/` (which is gitignored). |

## Restoring the dev-discovery symlink

`.agents/skills/` is gitignored, so a fresh clone won't have the `dogfooding-ion`
symlink that makes the skill discoverable in a Claude Code session. Recreate it
from the repo root:

```bash
ln -sfn ../../skills/dogfooding-ion .agents/skills/dogfooding-ion
```

(The `ion-cli` skill needs no symlink — the binary deploys it from its embedded
copy.)
