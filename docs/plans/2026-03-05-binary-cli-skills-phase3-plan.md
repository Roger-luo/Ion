# Binary CLI Skills Phase 3: Extended Sources & Polish

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add generic URL downloads, configurable asset naming, binary validation, and improve `ion list` and `ion info` to show binary skill details.

**Architecture:** Extend `binary.rs` with URL template expansion and binary validation. Add `asset_pattern` field to manifest/source. Modify `list.rs` and `info.rs` to check lockfile binary fields.

**Tech Stack:** Same as Phase 1/2 — Rust, reqwest, flate2/tar, clap, serde/toml

---

## Task 1: Add generic URL download support to binary.rs

**Files:**
- Modify: `crates/ion-skill/src/binary.rs`

Add URL template expansion and a generic download installer:

```rust
/// Expand URL template placeholders: {version}, {target}, {os}, {arch}, {binary}
pub fn expand_url_template(template: &str, binary_name: &str, version: &str) -> String {
    let platform = Platform::detect();
    template
        .replace("{version}", version)
        .replace("{target}", &platform.target_triple())
        .replace("{os}", &platform.os)
        .replace("{arch}", &platform.arch)
        .replace("{binary}", binary_name)
}

/// Install a binary skill from a generic URL template.
/// The source string is the URL template with placeholders.
/// `version` must be provided (no API to query latest for generic URLs).
pub fn install_binary_from_url(
    url_template: &str,
    binary_name: &str,
    version: &str,
    skill_dir: &Path,
) -> crate::Result<BinaryInstallResult> {
    // Cache check
    if is_binary_installed(binary_name, version) {
        // ... same as github path: ensure SKILL.md, return cached result
    }

    let url = expand_url_template(url_template, binary_name, version);
    // Download, extract, install (same steps as github path)
    // ...
}
```

**Tests:**
- `test_expand_url_template` — verify all placeholders expand correctly
- `test_expand_url_template_partial` — only some placeholders present

---

## Task 2: Wire generic URL into installer pipeline

**Files:**
- Modify: `crates/ion-skill/src/installer.rs`
- Modify: `crates/ion-skill/src/source.rs`

Update `install_binary()` in `installer.rs`:
- If `source.source` looks like a URL (starts with `http://` or `https://`), use `install_binary_from_url`
- Otherwise use existing `install_binary_from_github`

The `version` for URL sources comes from `source.rev` (required for URL-based binary sources since there's no API to query latest).

Also update `SkillSource::infer()` — when the user does `ion add https://example.com/releases/{version}/tool-{target}.tar.gz --bin`, ensure it gets `SourceType::Binary` (not `Http`). This is already handled by the `--bin` flag in `add.rs`, so no change needed in `infer()`.

**Tests:**
- Unit test in `installer.rs`: verify URL source calls `install_binary_from_url` path

---

## Task 3: Add asset_pattern field for configurable asset naming

**Files:**
- Modify: `crates/ion-skill/src/manifest.rs` — add `asset_pattern: Option<String>` to `SkillEntry::Full`
- Modify: `crates/ion-skill/src/source.rs` — add `asset_pattern: Option<String>` to `SkillSource`
- Modify: `crates/ion-skill/src/binary.rs` — use pattern in `match_asset` when provided

Ion.toml format:
```toml
[skills]
mytool = { type = "binary", source = "owner/mytool", binary = "mytool", asset_pattern = "mytool-{version}-{os}-{arch}.tar.gz" }
```

When `asset_pattern` is provided, skip the heuristic matching in `match_asset` and instead:
1. Expand the pattern using `expand_url_template`
2. Look for an exact match in the asset list

Update `Platform::match_asset` signature to accept `Option<&str>` pattern.

**Tests:**
- Test pattern-based matching when pattern is provided
- Test fallback to heuristic when no pattern

---

## Task 4: Add binary validation after installation

**Files:**
- Modify: `crates/ion-skill/src/binary.rs`

Add validation functions:

```rust
/// Validate that a binary is executable and has the expected `skill` subcommand.
pub fn validate_binary(binary_path: &Path) -> crate::Result<BinaryValidation> {
    // 1. Check file exists and is executable (unix permissions)
    // 2. Try running `<binary> --version` — capture version string (optional, don't fail)
    // 3. Try running `<binary> skill` — verify it produces output starting with `---`
    // Return a struct with results
}

pub struct BinaryValidation {
    pub is_executable: bool,
    pub version_output: Option<String>,
    pub has_skill_command: bool,
}
```

Call `validate_binary()` in `install_binary_from_github` after installing but before returning. Only warn (don't fail) if `skill` command doesn't work — the bundled SKILL.md fallback already handles this.

**Tests:**
- Test with a shell script that has both `--version` and `skill`
- Test with a shell script that has neither (should still succeed with warnings)

---

## Task 5: Show binary indicator in `ion list`

**Files:**
- Modify: `src/commands/list.rs`

After the existing output for each skill, check if the lockfile entry has `binary` set. If so, append a binary indicator:

```
  mytool v1.2.0 (abc12345) [installed] (binary)
    source: owner/mytool
```

Changes:
- Check `locked.and_then(|l| l.binary.as_deref())`
- If Some, append `p.info(" (binary)")` after the status
- For binary skills, show `binary_version` instead of `version` if available

**Tests:**
- Integration test: create manifest with binary skill, verify list output contains "(binary)"

---

## Task 6: Show binary details in `ion info`

**Files:**
- Modify: `src/commands/info.rs`

When showing info for an installed binary skill, add binary-specific details:

```
Skill: mytool
Description: A tool that does X
Version: 1.2.0
Binary: mytool
Binary version: 1.2.0
Binary path: ~/.local/share/ion/bin/mytool/1.2.0/mytool
Binary size: 12.5 MB
```

Changes to `show_info_from_installed()`:
1. Load lockfile to check for binary fields
2. If `locked.binary` is Some:
   - Print binary name
   - Print binary version from lockfile
   - Resolve binary path and print it
   - Get file size and print it formatted (KB/MB)
3. Print invocation hint: `Run with: ion run mytool [args]`

**Tests:**
- Integration test verifying binary info output

---

## Task 7: Integration tests for Phase 3

**Files:**
- Modify: `tests/binary_integration.rs`

Add tests for:
- URL template expansion (all placeholders)
- Asset pattern matching (pattern-based vs heuristic)
- Binary validation struct
- Manifest roundtrip with `asset_pattern` field

These are unit-level integration tests that don't require network access.
