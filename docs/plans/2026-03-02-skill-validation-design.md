# Skill Validation Design

## Problem

Ion installs skills from untrusted sources (GitHub, git, HTTP) into the user's agent environment. Currently, only spec validation runs (name format, required fields, YAML structure). There are no checks for prompt injection, permission overreach, or malicious payloads. Users need a way to detect vulnerable skills before they're deployed.

## Decision

Add a pre-install validation gate with rule-based security checkers. Validation runs after fetching and parsing SKILL.md but before deploying to `.agents/skills/`. Errors block installation. Warnings prompt the user for confirmation. A `--skip-validation` flag provides an escape hatch.

## Integration Point

```
fetch → parse SKILL.md (existing spec validation) → security validation (new) → deploy
```

- **Errors**: block installation, print findings, return error
- **Warnings**: print findings, prompt "Install anyway? [y/N]"
- **Info**: print findings, proceed
- `--skip-validation` on `ion add` bypasses all security checks

## Architecture

Rule-based checkers implementing a common trait:

```rust
pub trait SkillChecker {
    fn name(&self) -> &str;
    fn check(&self, skill_dir: &Path, meta: &SkillMetadata, body: &str) -> Vec<Finding>;
}

pub struct Finding {
    pub severity: Severity,    // Error, Warning, Info
    pub checker: String,
    pub message: String,
    pub location: Option<String>,
}
```

`run_all_checkers()` runs every registered checker and returns findings sorted by severity.

## Checkers

### 1. PromptInjectionChecker

Scans SKILL.md body for prompt injection indicators.

| Severity | Check |
|----------|-------|
| Error | Invisible Unicode characters (U+200B zero-width space, U+202E RTL override, U+200F right-to-left mark, etc.) |
| Warning | Known injection phrases: `ignore previous`, `you are now`, `disregard`, `new instructions`, `override`, `forget your` |
| Warning | HTML comments containing instruction-like content |
| Info | Base64-encoded strings |

### 2. ToolPermissionChecker

Compares SKILL.md body tool references against `allowed-tools` frontmatter.

| Severity | Check |
|----------|-------|
| Warning | Body references tools not declared in `allowed-tools` |
| Warning | `allowed-tools` missing but body contains tool references |
| Info | `allowed-tools` includes `Bash` (highest privilege) |

Known tools to scan for: Bash, Read, Write, Edit, WebFetch, WebSearch, Agent, Glob, Grep.

### 3. SensitivePathChecker

Scans SKILL.md body for sensitive path references.

| Severity | Check |
|----------|-------|
| Warning | References to `~/.ssh/`, `~/.aws/`, `~/.gnupg/`, `.env`, `credentials`, `id_rsa`, `/etc/passwd`, `/etc/shadow`, token/secret file patterns |
| Warning | Home directory wildcard patterns (`~/.*`) |

### 4. SuspiciousFileChecker

Scans the skill directory for unexpected files.

| Severity | Check |
|----------|-------|
| Error | Executable bit set on files outside `scripts/` |
| Warning | Script files (`.sh`, `.py`, `.rb`, `.js`) outside `scripts/` |
| Warning | Binary/compiled files (`.exe`, `.dll`, `.so`, `.dylib`) |
| Info | Files not matching expected types (`.md`, `.txt`, `.yaml`, `.yml`, `.toml`, `.json`, images) |

### 5. ExternalUrlChecker

Scans SKILL.md body for external URL references.

| Severity | Check |
|----------|-------|
| Warning | `curl \| sh` or `wget \| sh` patterns |
| Warning | URL shorteners (bit.ly, tinyurl, t.co, etc.) |
| Info | Any external URLs |

## Output Format

```
⚠ Validating skill 'code-reviewer'...

  ERROR [prompt-injection] Invisible Unicode characters found at line 42
        Zero-width space (U+200B) detected — may hide instructions

  WARN  [tool-permission] Body references 'Bash' but allowed-tools is not declared
        Consider adding 'allowed-tools: Bash, Read' to frontmatter

  WARN  [sensitive-path] References to ~/.ssh/ found at line 87
        Skill instructions mention sensitive credential paths

  INFO  [external-url] External URL found: https://example.com/setup.sh
        Verify this URL is trusted

  Found: 1 error, 2 warnings, 1 info

  ✗ Installation blocked — resolve errors before installing.
```

## Code Structure

```
crates/ion-skill/src/validate/
├── mod.rs              # SkillChecker trait, Finding, Severity, run_all_checkers()
├── prompt_injection.rs # PromptInjectionChecker
├── tool_permission.rs  # ToolPermissionChecker
├── sensitive_path.rs   # SensitivePathChecker
├── suspicious_file.rs  # SuspiciousFileChecker
└── external_url.rs     # ExternalUrlChecker
```

## Changes to Existing Code

- `installer.rs`: Call `run_all_checkers()` after `validate()` (spec), before `deploy()`. Handle findings based on severity.
- `src/commands/add.rs`: Add `--skip-validation` flag to `AddArgs`.
- `crates/ion-skill/src/lib.rs`: Export `validate` module.
