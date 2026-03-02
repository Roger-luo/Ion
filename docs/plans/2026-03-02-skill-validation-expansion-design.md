# Skill Validation Expansion Design

## Problem

Current validation only enforces SKILL.md schema/frontmatter constraints. We need broader pre-install and local validation to detect security risks, structural integrity issues, and invalid code examples before skills are deployed.

## Goals

1. Extend validation beyond schema into security, structure, and code-block checks.
2. Reuse one validation engine for both install-time checks and local developer checks.
3. Provide a dedicated `ion validate` command for recursive project-wide validation.
4. Keep validation static-only: do not execute any skill code.

## Decisions from Brainstorming

1. Warning policy: interactive prompt (`Install anyway? [y/N]`) for install flows.
2. Execution policy: static/offline validation only.
3. Scope: security + structural quality + code-block checks.
4. Blocking policy: security findings and clearly invalid syntax can be `Error` and block.
5. Parsing stack for fenced code blocks: `tree-sitter` with language grammars.

## Approaches Considered

1. Lean hardcoded validator set.
Pros: fastest to implement.
Cons: poor extensibility, checker coupling.

2. Modular checker pipeline (selected).
Pros: maps to existing `SkillChecker` architecture, scales cleanly by checker domain, easier testing.
Cons: more modules and initial scaffolding.

3. Policy-driven configurable rules engine.
Pros: highly flexible.
Cons: too much complexity for v1.

## Architecture

Validation runs in this order:

`fetch/locate skill -> parse SKILL.md schema -> run domain checkers -> summarize findings -> apply policy -> deploy/report`

Checker domains under `crates/ion-skill/src/validate/`:

1. `security/*`
2. `structure/*`
3. `codeblock/*`

Severity contract:

1. `Error`: high-confidence harmful content or clear parse/syntax failure.
2. `Warning`: suspicious or ambiguous issues.
3. `Info`: advisory findings.

Policy behavior:

1. Install flows (`ion add`, `ion install`):
- `Error` present: block.
- No errors, warnings present: prompt `Install anyway? [y/N]`.
- Info only: proceed.
2. `ion validate`: non-interactive, never prompts; exits non-zero if any `Error`.

## Validation Scope (v1)

### Security

1. Prompt-injection indicators (override phrases, hidden/invisible control characters, suspicious hidden directives).
2. Dangerous command patterns in prose or code fences (for example `curl|sh`, `wget|sh`, high-risk broad filesystem commands).
3. Sensitive path/token references (`~/.ssh`, cloud credential paths, key/token file patterns).
4. Suspicious packaged files (unexpected binaries, executable files in disallowed locations, nested repo artifacts/hooks).

### Structure

1. Referenced local files in instructions must exist (`scripts/`, `references/`, `assets/`, etc.).
2. Detect broken local markdown links/anchors.
3. Flag path traversal references escaping skill root.
4. Detect frontmatter/body mismatches (for example tool mentions with missing/weak declarations).

### Code blocks (static, `tree-sitter`)

1. Parse fenced blocks for supported languages (`bash`, `sh`, `python`, `rust`, plus structured data fences where feasible).
2. Report clear parser failures as `Error` for runnable/explicit blocks.
3. Report lower-confidence issues as `Warning`.
4. Never execute snippets.

## New Command: `ion validate`

1. `ion validate`
- Treat current directory as a project/workspace.
- Recursively discover all `SKILL.md` files.
- Validate each discovered skill.

2. `ion validate <path>`
- If `<path>` is a `SKILL.md` file: validate exactly that skill.
- If `<path>` is a directory containing `SKILL.md`: validate that single skill.
- Otherwise recursively search under `<path>` for `SKILL.md` files and validate all matches.

Output and status:

1. Group findings by skill and severity.
2. Provide aggregate summary totals.
3. Exit `0` when no errors, `1` when any error exists.

## Reliability and Performance

1. Do not abort whole runs for single-skill failures; emit per-skill findings and continue.
2. Ignore common heavy directories (`.git`, `node_modules`, `target`, caches) during recursion.
3. Enforce scan caps (file size/block length) and report truncation as `Info`.
4. Keep deterministic ordering: sort skills and findings for stable output/tests.

## Testing Strategy

1. Unit tests per checker domain and severity classification.
2. Integration tests for install gating (`Error` blocks, `Warning` prompts).
3. Integration tests for `ion validate` path modes, recursive discovery, and exit codes.
4. Snapshot tests for human-readable report formatting.

## Out of Scope (v1)

1. Executing snippets or scripts.
2. Full style linting using external tools (`shellcheck`, `ruff`, `clippy`).
3. User-configurable checker policy in `ion.toml`.

## Success Criteria

1. Installer and CLI both use the same validator pipeline.
2. `ion validate` supports workspace recursion and targeted path validation.
3. High-confidence malicious/invalid conditions are blocked.
4. Warnings are surfaced with explicit install-time user choice.
