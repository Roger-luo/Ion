# Deduplication Refactor Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Eliminate duplicated code across the install pipeline by extracting shared abstractions for validation-warning handling, registry registration, Finding serialization, resolve_skill_dir, and binary install core logic.

**Architecture:** New shared modules (`src/commands/install_shared.rs` for CLI-side helpers, utility methods on existing types in `ion-skill`) centralize repeated patterns. Commands become thin wrappers that call shared functions. The `install_binary_core` internal function unifies the two binary install paths.

**Tech Stack:** Rust, ion-skill crate, serde/serde_json

---

## File Structure

**New files:**
- `src/commands/install_shared.rs` — shared CLI helpers for install commands (warning prompt, registry, gitignore+lockfile post-install, SkillEntry type, validate-and-partition pipeline)

**Modified files:**
- `src/commands/mod.rs` — add `pub mod install_shared;`
- `src/commands/add.rs` — replace duplicated code with calls to `install_shared`
- `src/commands/install.rs` — replace duplicated code with calls to `install_shared`
- `src/commands/link.rs` — use shared lockfile helper
- `crates/ion-skill/src/validate/mod.rs` — add `Serialize` derive to `Finding`, `Severity`, `ValidationReport`
- `crates/ion-skill/src/installer.rs` — extract `resolve_skill_dir` as pub function
- `crates/ion-skill/src/update/git.rs` — use `installer::resolve_skill_dir` instead of local copy
- `crates/ion-skill/src/binary.rs` — extract `install_binary_core` to unify `install_binary_from_github` and `install_binary_from_url`

---

## Chunk 1: Foundation — Serialize on Finding + resolve_skill_dir dedup

### Task 1: Add Serialize to Finding, Severity, and ValidationReport

This eliminates 4+ hand-rolled JSON serializations of findings across `add.rs`, `install.rs`, and `validate.rs`.

**Files:**
- Modify: `crates/ion-skill/src/validate/mod.rs:17-101`

- [ ] **Step 1: Add Serialize derive to Severity**

In `crates/ion-skill/src/validate/mod.rs`, change:
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Severity {
```
to:
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize)]
pub enum Severity {
```

Also add a custom serialization that matches the existing `.to_string()` output (INFO/WARN/ERROR) so JSON output doesn't change. The simplest approach: keep the `Display` impl and use `#[serde(rename_all = "UPPERCASE")]` won't work since we use "WARN" not "WARNING". Instead, use `#[serde(serialize_with)]` or just let `Serialize` use variant names and then adjust. The cleanest path: add `#[serde(rename = "INFO")]`, `#[serde(rename = "WARN")]`, `#[serde(rename = "ERROR")]` to each variant.

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize)]
pub enum Severity {
    #[serde(rename = "INFO")]
    Info,
    #[serde(rename = "WARN")]
    Warning,
    #[serde(rename = "ERROR")]
    Error,
}
```

- [ ] **Step 2: Add Serialize derive to Finding**

```rust
#[derive(Debug, Clone, serde::Serialize)]
pub struct Finding {
    pub severity: Severity,
    pub checker: String,
    pub message: String,
    pub detail: Option<String>,
}
```

- [ ] **Step 3: Add Serialize derive to ValidationReport**

```rust
#[derive(Debug, Clone, serde::Serialize)]
pub struct ValidationReport {
    pub findings: Vec<Finding>,
    pub error_count: usize,
    pub warning_count: usize,
    pub info_count: usize,
}
```

- [ ] **Step 4: Run tests to verify nothing breaks**

Run: `cargo test -p ion-skill`
Expected: All existing tests pass. Serialization is additive.

- [ ] **Step 5: Commit**

```bash
git add crates/ion-skill/src/validate/mod.rs
git commit -m "refactor: add Serialize derives to Finding, Severity, ValidationReport"
```

### Task 2: Deduplicate resolve_skill_dir

**Files:**
- Modify: `crates/ion-skill/src/installer.rs:370-390`
- Modify: `crates/ion-skill/src/update/git.rs:104-121`

- [ ] **Step 1: Extract resolve_skill_dir as a pub function in installer.rs**

In `crates/ion-skill/src/installer.rs`, rename the inline logic in `fetch_skill` to call a new pub function. Add this function right after `fetch_skill_base`:

```rust
/// Resolve the skill directory within a repo, handling subdirectory skills.
/// Tries `repo_dir/path` first, then falls back to `repo_dir/skills/path`.
pub fn resolve_skill_dir(repo_dir: &Path, path: Option<&str>) -> Result<PathBuf> {
    match path {
        None => Ok(repo_dir.to_path_buf()),
        Some(p) => {
            let direct = repo_dir.join(p);
            if direct.exists() {
                return Ok(direct);
            }
            let fallback = repo_dir.join("skills").join(p);
            if fallback.exists() {
                return Ok(fallback);
            }
            Err(Error::Source(format!(
                "Skill path '{p}' not found in repository (also tried 'skills/{p}')"
            )))
        }
    }
}
```

Then simplify `fetch_skill` to use it:

```rust
fn fetch_skill(source: &SkillSource) -> Result<PathBuf> {
    let base_dir = fetch_skill_base(source)?;
    resolve_skill_dir(&base_dir, source.path.as_deref())
}
```

- [ ] **Step 2: Update git.rs to use installer::resolve_skill_dir**

In `crates/ion-skill/src/update/git.rs`, remove the local `resolve_skill_dir` function (lines 103-121) and update the import to use `crate::installer::resolve_skill_dir`:

Change line 3 from:
```rust
use crate::installer::{SkillInstaller, data_dir, hash_simple};
```
to:
```rust
use crate::installer::{SkillInstaller, data_dir, hash_simple, resolve_skill_dir};
```

Remove the local `resolve_skill_dir` function entirely (lines 103-121).

- [ ] **Step 3: Run tests**

Run: `cargo test -p ion-skill`
Expected: All tests pass, including the `resolve_skill_dir_*` tests in `git.rs`. Move those tests to `installer.rs` tests module or keep them in `git.rs` — they call the same function now.

Actually, the tests in `git.rs` call the local function. After removing it, they need to call `resolve_skill_dir` from `installer`. Update the test imports:

In `git.rs` tests, the tests `resolve_skill_dir_none_returns_repo`, etc. should still work since they call `resolve_skill_dir(...)` which now resolves to the imported function.

- [ ] **Step 4: Commit**

```bash
git add crates/ion-skill/src/installer.rs crates/ion-skill/src/update/git.rs
git commit -m "refactor: deduplicate resolve_skill_dir into installer module"
```

---

## Chunk 2: Shared install helpers — install_shared.rs

### Task 3: Create install_shared.rs with SkillEntry and register_in_registry

**Files:**
- Create: `src/commands/install_shared.rs`
- Modify: `src/commands/mod.rs`

- [ ] **Step 1: Create install_shared.rs with shared types and register_in_registry**

```rust
use ion_skill::installer::hash_simple;
use ion_skill::registry::Registry;
use ion_skill::source::{SkillSource, SourceType};

/// A skill ready for installation (validated, source resolved).
pub struct SkillEntry {
    pub name: String,
    pub source: SkillSource,
}

/// Register a git-based skill source in the global registry.
pub fn register_in_registry(
    source: &SkillSource,
    project_dir: &std::path::Path,
) -> anyhow::Result<()> {
    if matches!(source.source_type, SourceType::Github | SourceType::Git)
        && let Ok(url) = source.git_url()
    {
        let repo_hash = format!("{:x}", hash_simple(&url));
        let project_str = project_dir.display().to_string();
        let mut registry = Registry::load()?;
        registry.register(&repo_hash, &url, &project_str);
        registry.save()?;
    }
    Ok(())
}
```

- [ ] **Step 2: Add module to mod.rs**

In `src/commands/mod.rs`, add:
```rust
pub mod install_shared;
```

- [ ] **Step 3: Run `cargo check`**

Run: `cargo check`
Expected: Compiles cleanly.

- [ ] **Step 4: Commit**

```bash
git add src/commands/install_shared.rs src/commands/mod.rs
git commit -m "refactor: create install_shared module with SkillEntry and register_in_registry"
```

### Task 4: Add install_with_warning_prompt to install_shared.rs

This unifies the 3 duplicated "install → catch ValidationWarning → prompt → reinstall" sequences.

**Files:**
- Modify: `src/commands/install_shared.rs`

- [ ] **Step 1: Add the function**

```rust
use ion_skill::Error as SkillError;
use ion_skill::installer::{InstallValidationOptions, SkillInstaller};
use ion_skill::lockfile::LockedSkill;
use ion_skill::validate::ValidationReport;

use crate::commands::validation::{confirm_install_on_warnings, print_validation_report};

/// Install a skill, handling validation warnings interactively.
///
/// If the install produces warnings:
/// - In JSON mode (without allow_warnings): prints action_required and exits
/// - In interactive mode: shows the report and prompts for confirmation
/// - Then retries with allow_warnings=true
pub fn install_with_warning_prompt(
    installer: &SkillInstaller,
    name: &str,
    source: &SkillSource,
    json: bool,
    allow_warnings: bool,
) -> anyhow::Result<LockedSkill> {
    match installer.install(name, source) {
        Ok(locked) => Ok(locked),
        Err(SkillError::ValidationWarning { report, .. }) => {
            handle_validation_warnings(name, &report, json, allow_warnings)?;
            let locked = installer.install_with_options(
                name,
                source,
                InstallValidationOptions {
                    skip_validation: false,
                    allow_warnings: true,
                },
            )?;
            Ok(locked)
        }
        Err(err) => Err(err.into()),
    }
}

/// Display validation warnings and prompt for confirmation (or exit in JSON mode).
fn handle_validation_warnings(
    name: &str,
    report: &ValidationReport,
    json: bool,
    allow_warnings: bool,
) -> anyhow::Result<()> {
    if json && !allow_warnings {
        crate::json::print_action_required(
            "validation_warnings",
            serde_json::json!({
                "skill": name,
                "warnings": report.findings,
            }),
        );
    }
    if !json {
        print_validation_report(name, report);
        if !confirm_install_on_warnings()? {
            anyhow::bail!("Installation cancelled due to validation warnings.");
        }
    }
    Ok(())
}
```

Note: Now that `Finding` implements `Serialize`, we can just use `report.findings` directly in the JSON — no more hand-rolled mapping.

- [ ] **Step 2: Run `cargo check`**

Run: `cargo check`
Expected: Compiles cleanly.

- [ ] **Step 3: Commit**

```bash
git add src/commands/install_shared.rs
git commit -m "refactor: add install_with_warning_prompt to install_shared"
```

### Task 5: Add post-install helpers to install_shared.rs

Consolidate gitignore + registry + lockfile write patterns.

**Files:**
- Modify: `src/commands/install_shared.rs`

- [ ] **Step 1: Add add_gitignore_entries helper**

```rust
use ion_skill::manifest::ManifestOptions;

/// Add gitignore entries for a remote skill (skips Path/Local sources).
pub fn add_gitignore_entries(
    project_dir: &std::path::Path,
    name: &str,
    source: &SkillSource,
    merged_options: &ManifestOptions,
) -> anyhow::Result<()> {
    if !matches!(source.source_type, SourceType::Path | SourceType::Local) {
        let target_paths: Vec<&str> = merged_options
            .targets
            .values()
            .map(|s| s.as_str())
            .collect();
        ion_skill::gitignore::add_skill_entries(project_dir, name, &target_paths)?;
    }
    Ok(())
}
```

- [ ] **Step 2: Run `cargo check`**

Run: `cargo check`
Expected: Compiles cleanly.

- [ ] **Step 3: Commit**

```bash
git add src/commands/install_shared.rs
git commit -m "refactor: add gitignore helper to install_shared"
```

---

## Chunk 3: Wire up add.rs to use install_shared

### Task 6: Refactor add.rs single-skill paths to use install_shared

**Files:**
- Modify: `src/commands/add.rs`

- [ ] **Step 1: Replace the two single-install warning paths**

In `add.rs`, replace the duplicated warning-handling blocks (lines 78-127 for no-path, lines 154-186 for with-path) with calls to `install_with_warning_prompt`.

For the no-path case (lines 78-127), replace:
```rust
match installer.install(&name, &source) {
    Ok(locked) => {
        return finish_single_install(...);
    }
    Err(SkillError::ValidationWarning { report, .. }) => {
        // ... 25 lines of warning handling ...
    }
    Err(SkillError::InvalidSkill(msg)) if msg.contains("No SKILL.md found") => {
        return install_collection(...);
    }
    Err(err) => return Err(err.into()),
}
```

With:
```rust
match installer.install(&name, &source) {
    Ok(locked) => {
        return finish_single_install(&ctx, &p, &merged_options, &name, &source, locked, json);
    }
    Err(SkillError::ValidationWarning { report, .. }) => {
        crate::commands::install_shared::handle_validation_warnings(
            &name, &report, json, allow_warnings,
        )?;
        let locked = installer.install_with_options(
            &name,
            &source,
            InstallValidationOptions {
                skip_validation: false,
                allow_warnings: true,
            },
        )?;
        return finish_single_install(&ctx, &p, &merged_options, &name, &source, locked, json);
    }
    Err(SkillError::InvalidSkill(msg)) if msg.contains("No SKILL.md found") => {
        return install_collection(&ctx, &p, &merged_options, &source, source_str, json, allow_warnings, skills_filter);
    }
    Err(err) => return Err(err.into()),
}
```

For the with-path case (lines 154-186), replace with `install_with_warning_prompt`:
```rust
let locked = crate::commands::install_shared::install_with_warning_prompt(
    &installer, &name, &source, json, allow_warnings,
)?;
```

- [ ] **Step 2: Replace register_in_registry with shared version**

Remove the local `register_in_registry` function (lines 582-593) and update all call sites to use `crate::commands::install_shared::register_in_registry`.

- [ ] **Step 3: Replace finish_collection_skill_install gitignore logic with shared helper**

In `finish_collection_skill_install` (lines 496-522) and `finish_single_install` (lines 524-580), replace the inline gitignore logic with `install_shared::add_gitignore_entries`.

- [ ] **Step 4: Run tests**

Run: `cargo test`
Expected: All tests pass.

- [ ] **Step 5: Commit**

```bash
git add src/commands/add.rs
git commit -m "refactor: use install_shared helpers in add command"
```

### Task 7: Refactor add.rs collection path to use SkillEntry from install_shared

**Files:**
- Modify: `src/commands/add.rs`

- [ ] **Step 1: Remove local SkillEntry struct definition**

In `install_collection` (around line 227-230), remove:
```rust
struct SkillEntry {
    name: String,
    source: SkillSource,
}
```

Replace all uses with `crate::commands::install_shared::SkillEntry` (add a `use` at the top).

- [ ] **Step 2: Replace inline gitignore calls in collection install loop**

In the phase 3 install loops (lines 380-440), replace the gitignore logic in `finish_collection_skill_install` with `install_shared::add_gitignore_entries`.

- [ ] **Step 3: Run tests**

Run: `cargo test`
Expected: All tests pass.

- [ ] **Step 4: Commit**

```bash
git add src/commands/add.rs
git commit -m "refactor: use shared SkillEntry in add collection path"
```

---

## Chunk 4: Wire up install.rs to use install_shared

### Task 8: Refactor install.rs to use install_shared

**Files:**
- Modify: `src/commands/install.rs`

- [ ] **Step 1: Remove local SkillEntry and register_in_registry**

Remove the local `SkillEntry` struct (lines 56-59) and `register_in_registry` function (lines 300-314). Add imports:
```rust
use crate::commands::install_shared::{
    SkillEntry, add_gitignore_entries, register_in_registry,
};
```

- [ ] **Step 2: Replace Finding serialization with direct Serialize**

In the JSON warning output (lines 157-184), replace the hand-rolled findings mapping:
```rust
let findings: Vec<serde_json::Value> = report
    .findings
    .iter()
    .map(|f| {
        serde_json::json!({
            "severity": f.severity.to_string(),
            "checker": f.checker,
            "message": f.message,
        })
    })
    .collect();
```

With just `&report.findings` (since `Finding` now implements `Serialize`):
```rust
serde_json::json!({
    "name": entry.name,
    "warning_count": report.warning_count,
    "findings": &report.findings,
})
```

Note: This changes the JSON shape slightly — `detail` field will now be included (as `null` when absent). If this is unacceptable, use `#[serde(skip_serializing_if = "Option::is_none")]` on `Finding::detail`. Add that annotation in Task 1.

- [ ] **Step 3: Replace inline gitignore blocks with shared helper**

Replace the two identical gitignore blocks (in clean install loop lines 220-230 and warned install loop lines 259-269) with:
```rust
add_gitignore_entries(&ctx.project_dir, &entry.name, &entry.source, &merged_options)?;
```

- [ ] **Step 4: Replace register_in_registry calls**

Already imported from install_shared, just ensure call sites match.

- [ ] **Step 5: Run tests**

Run: `cargo test`
Expected: All tests pass.

- [ ] **Step 6: Commit**

```bash
git add src/commands/install.rs
git commit -m "refactor: use install_shared helpers in install command"
```

---

## Chunk 5: Binary install deduplication

### Task 9: Extract install_binary_core in binary.rs

**Files:**
- Modify: `crates/ion-skill/src/binary.rs:430-646`

- [ ] **Step 1: Extract the shared core**

Add a new internal function that handles the common download → extract → install → SKILL.md → validate pipeline:

```rust
/// Core binary installation from a resolved download URL.
/// Shared between `install_binary_from_github` and `install_binary_from_url`.
fn install_binary_core(
    binary_name: &str,
    version: &str,
    download_url: &str,
    asset_name: &str,
    skill_dir: &Path,
) -> crate::Result<BinaryInstallResult> {
    let tmp_dir = tempfile::tempdir()
        .map_err(|e| crate::Error::Other(format!("Failed to create temp dir: {}", e)))?;
    let archive_path = tmp_dir.path().join(asset_name);
    download_file(download_url, &archive_path)?;

    let extract_dir = tmp_dir.path().join("extracted");
    extract_tar_gz(&archive_path, &extract_dir)?;

    let found_binary = find_binary_in_dir(&extract_dir, binary_name)?;
    let bin_root = bin_dir();
    install_binary_file(&found_binary, binary_name, version, &bin_root)?;

    let installed_binary = binary_path(binary_name, version);
    let checksum = file_checksum(&installed_binary)?;

    fs::create_dir_all(skill_dir)
        .map_err(|e| crate::Error::Other(format!("Failed to create skill dir: {}", e)))?;

    let skill_md_content = if let Some(bundled) = find_bundled_skill_md(&extract_dir) {
        fs::read_to_string(&bundled)
            .map_err(|e| crate::Error::Other(format!("Failed to read bundled SKILL.md: {}", e)))?
    } else {
        generate_skill_md(&installed_binary)?
    };

    fs::write(skill_dir.join("SKILL.md"), &skill_md_content)
        .map_err(|e| crate::Error::Other(format!("Failed to write SKILL.md: {}", e)))?;

    let mut warnings = Vec::new();
    if let Ok(validation) = validate_binary(&installed_binary) {
        if !validation.is_executable {
            warnings.push(format!("Binary '{}' may not be executable", binary_name));
        }
        if !validation.has_skill_command {
            warnings.push(format!(
                "Binary '{}' does not have a 'skill' subcommand",
                binary_name
            ));
        }
    }

    Ok(BinaryInstallResult {
        version: version.to_string(),
        binary_checksum: checksum,
        warnings,
    })
}
```

- [ ] **Step 2: Extract check_already_installed helper**

```rust
/// Check cache and return early result if binary is already installed.
fn check_already_installed(
    binary_name: &str,
    version: &str,
    skill_dir: &Path,
) -> crate::Result<Option<BinaryInstallResult>> {
    if !is_binary_installed(binary_name, version) {
        return Ok(None);
    }
    let installed_binary = binary_path(binary_name, version);
    let checksum = file_checksum(&installed_binary)?;

    fs::create_dir_all(skill_dir)
        .map_err(|e| crate::Error::Other(format!("Failed to create skill dir: {}", e)))?;

    if !skill_dir.join("SKILL.md").exists() {
        let skill_md_content = generate_skill_md(&installed_binary)?;
        fs::write(skill_dir.join("SKILL.md"), &skill_md_content)
            .map_err(|e| crate::Error::Other(format!("Failed to write SKILL.md: {}", e)))?;
    }

    Ok(Some(BinaryInstallResult {
        version: version.to_string(),
        binary_checksum: checksum,
        warnings: Vec::new(),
    }))
}
```

- [ ] **Step 3: Simplify install_binary_from_github**

```rust
pub fn install_binary_from_github(
    repo: &str,
    binary_name: &str,
    rev: Option<&str>,
    skill_dir: &Path,
    asset_pattern: Option<&str>,
) -> crate::Result<BinaryInstallResult> {
    let platform = Platform::detect();
    let release = fetch_github_release(repo, rev)?;
    let version = parse_version_from_tag(&release.tag_name).to_string();

    if let Some(result) = check_already_installed(binary_name, &version, skill_dir)? {
        return Ok(result);
    }

    let asset_names: Vec<String> = release.assets.iter().map(|a| a.name.clone()).collect();

    let asset_name = if let Some(pattern) = asset_pattern {
        let expanded = expand_url_template(pattern, binary_name, &version);
        if asset_names.contains(&expanded) {
            expanded
        } else {
            return Err(crate::Error::Other(format!(
                "Asset pattern expanded to '{}' but no matching asset found in {:?}",
                expanded, asset_names
            )));
        }
    } else {
        platform
            .match_asset(binary_name, &asset_names)
            .ok_or_else(|| {
                crate::Error::Other(format!(
                    "No matching release asset for platform {} in {:?}",
                    platform.target_triple(),
                    asset_names
                ))
            })?
    };
    let asset = release
        .assets
        .iter()
        .find(|a| a.name == asset_name)
        .ok_or_else(|| {
            crate::Error::Other(format!(
                "Asset '{}' not found in release (this is a bug — asset was matched but not found)",
                asset_name
            ))
        })?;

    install_binary_core(
        binary_name,
        &version,
        &asset.browser_download_url,
        &asset_name,
        skill_dir,
    )
}
```

- [ ] **Step 4: Simplify install_binary_from_url**

```rust
pub fn install_binary_from_url(
    url_template: &str,
    binary_name: &str,
    version: &str,
    skill_dir: &Path,
) -> crate::Result<BinaryInstallResult> {
    if let Some(result) = check_already_installed(binary_name, version, skill_dir)? {
        return Ok(result);
    }

    let url = expand_url_template(url_template, binary_name, version);

    if !url.ends_with(".tar.gz") && !url.ends_with(".tgz") {
        return Err(crate::Error::Other(format!(
            "URL-based binary sources currently only support .tar.gz archives, got: {url}"
        )));
    }

    install_binary_core(
        binary_name,
        version,
        &url,
        &format!("{}.tar.gz", binary_name),
        skill_dir,
    )
}
```

- [ ] **Step 5: Run tests**

Run: `cargo test -p ion-skill`
Expected: All existing binary tests pass.

- [ ] **Step 6: Commit**

```bash
git add crates/ion-skill/src/binary.rs
git commit -m "refactor: extract install_binary_core to deduplicate binary install paths"
```

---

## Chunk 6: Clean up remaining callers

### Task 10: Update validate.rs to use Serialize for Finding

**Files:**
- Modify: `src/commands/validate.rs:52-64`

- [ ] **Step 1: Simplify JSON findings in validate command**

Replace the hand-rolled findings mapping (lines 53-64):
```rust
let findings: Vec<serde_json::Value> = report
    .findings
    .iter()
    .map(|f| {
        serde_json::json!({
            "severity": f.severity.to_string(),
            "checker": f.checker,
            "message": f.message,
            "detail": f.detail,
        })
    })
    .collect();
```

With direct serialization:
```rust
let findings = &report.findings;
```

And update the json_skills push to use it directly:
```rust
json_skills.push(serde_json::json!({
    "path": skill_md.display().to_string(),
    "name": meta.name,
    "findings": findings,
    "errors": report.error_count,
    "warnings": report.warning_count,
    "infos": report.info_count,
}));
```

- [ ] **Step 2: Run tests**

Run: `cargo test`
Expected: All tests pass.

- [ ] **Step 3: Commit**

```bash
git add src/commands/validate.rs
git commit -m "refactor: use Finding's Serialize impl in validate command"
```

### Task 11: Update add.rs collection JSON to use Serialize for findings

**Files:**
- Modify: `src/commands/add.rs`

- [ ] **Step 1: Simplify the collection warning JSON block**

In `install_collection`, replace the hand-rolled findings JSON (around lines 344-351):
```rust
"warnings": r.findings.iter().map(|f| serde_json::json!({
    "severity": f.severity.to_string(),
    "checker": &f.checker,
    "message": &f.message,
})).collect::<Vec<_>>(),
```

With:
```rust
"warnings": &r.findings,
```

- [ ] **Step 2: Run tests**

Run: `cargo test`
Expected: All tests pass.

- [ ] **Step 3: Commit**

```bash
git add src/commands/add.rs
git commit -m "refactor: use Finding's Serialize impl in add collection path"
```

### Task 12: Add skip_serializing_if to Finding::detail

This ensures the `detail: null` field doesn't appear in JSON output where it wasn't previously included, maintaining backward compatibility.

**Files:**
- Modify: `crates/ion-skill/src/validate/mod.rs`

- [ ] **Step 1: Add skip_serializing_if annotation**

```rust
#[derive(Debug, Clone, serde::Serialize)]
pub struct Finding {
    pub severity: Severity,
    pub checker: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}
```

- [ ] **Step 2: Run tests**

Run: `cargo test`
Expected: All tests pass.

- [ ] **Step 3: Commit**

```bash
git add crates/ion-skill/src/validate/mod.rs
git commit -m "refactor: skip null detail field in Finding JSON serialization"
```

### Task 13: Final cleanup and full test run

- [ ] **Step 1: Run full test suite**

Run: `cargo test`
Expected: All tests pass.

- [ ] **Step 2: Run clippy**

Run: `cargo clippy`
Expected: No new warnings.

- [ ] **Step 3: Run fmt**

Run: `cargo fmt`

- [ ] **Step 4: Verify no dead code**

Check that the removed local functions (register_in_registry in add.rs and install.rs, resolve_skill_dir in git.rs, SkillEntry in both files) don't leave orphaned imports.

Run: `cargo check 2>&1 | grep "unused"`
Expected: No unused warnings from our changes.

- [ ] **Step 5: Final commit if needed**

```bash
git add -A
git commit -m "refactor: clean up unused imports from deduplication refactor"
```
