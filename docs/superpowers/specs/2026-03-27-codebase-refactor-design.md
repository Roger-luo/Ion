# Ion Codebase Refactor Design

**Date:** 2026-03-27
**Status:** Draft
**Scope:** Comprehensive refactor across `ion` (CLI) and `ion-skill` (library) crates

## Problem

The codebase has grown organically and accumulated:

- **Type-level ambiguity:** `LockedSkill` and `SkillSource` are flat structs where optional fields represent mutually exclusive variants (git vs. binary vs. local). Callers must check both the source type and whether a field is populated.
- **Duplicated sequences:** The install-commit pipeline (gitignore + registry + manifest + lockfile) is repeated at 6+ call sites. The validate-bucket-prompt-install loop is nearly identical in `add.rs` and `install.rs`.
- **Scattered constants:** `".agents/skills"` default appears in 4 files. Local path detection is reimplemented after `SourceType::Path` already encodes it.
- **Inconsistencies:** `info.rs` and `list.rs` hardcode `.agents/skills/` ignoring `skills-dir` config. `entry.resolve()` errors are silently dropped in `update.rs`/`list.rs` but propagated in `install.rs`. `migrate.rs` bypasses the centralized `git.rs` module.

## Approach

Five sequential tracks, each a standalone shippable unit:

1. **Constants & small fixes** — single source of truth for defaults, bug fixes
2. **Type redesign** — `LockedSkill` enum, `SkillSource` per-type data
3. **`ProjectContext` enrichment** — reduce command preamble boilerplate
4. **Command pipeline extraction** — shared install-commit and validation-bucket flows
5. **Config & writer consolidation** — `manifest_writer` unification, `migrate.rs` cleanup

---

## Track 1: Constants & Small Fixes

### 1.1 `DEFAULT_SKILLS_DIR` constant

Currently `".agents/skills"` is hardcoded in:
- `crates/ion-skill/src/installer.rs:70` — `skill_dir()` method (most impactful: `deploy()`, `uninstall()`, `install_binary()` all route through this)
- `crates/ion-skill/src/update/binary.rs:53-57` — constructs skill_dir path directly
- `src/commands/new.rs:8`
- `src/commands/eject.rs:30`
- `src/commands/install.rs:76`
- `src/tui/app.rs:152`

**Change:** Add a public constant to `ion-skill`:

```rust
// crates/ion-skill/src/manifest.rs
pub const DEFAULT_SKILLS_DIR: &str = ".agents/skills";
```

Add a convenience method to `ManifestOptions`:

```rust
impl ManifestOptions {
    pub fn skills_dir(&self) -> &str {
        self.skills_dir_field.as_deref().unwrap_or(DEFAULT_SKILLS_DIR)
    }
}
```

Note: the existing `skills_dir` field must be renamed to `skills_dir_field` (or similar) to avoid name collision with the method, or use a different method name like `resolved_skills_dir()`. Prefer the renamed method approach: keep the field as `skills_dir` with serde rename, add method `skills_dir_or_default()`.

Replace all 6 hardcoded sites with `DEFAULT_SKILLS_DIR` or the method as appropriate. In particular, `SkillInstaller::skill_dir()` should use the `ManifestOptions` it already holds to resolve the skills directory, and `BinaryUpdater` should use `installer.skill_dir()` instead of constructing the path directly.

### 1.2 Replace local path detection with `SourceType::Path`

Currently `add.rs:41-43` and `installer.rs:229-231` both check:
```rust
source.source.starts_with('/') || source.source.starts_with("./") || source.source.starts_with("../")
```

After `SkillSource::infer()` runs, the answer is already in `source.source_type`. Replace both sites with:
```rust
source.source_type == SourceType::Path
```

For `add.rs:41-43` this check happens *before* binary auto-detection mutates `source_type`, so the check is equivalent — `infer()` returns `SourceType::Path` for these prefixes.

For `installer.rs:229-231` (inside `install_binary`), the source has already been inferred. However, `install_binary` is only called when `source_type == Binary`, and the check is asking "was this *originally* a local path before being upgraded to binary?" This means we need to preserve the original path-ness. Two options:

- **Option A:** Add a `SkillSource::is_local_path()` method that checks the raw string. Simple, preserves current semantics.
- **Option B:** After track 2's type redesign, `SkillSourceKind::Binary` will carry a `local_path: Option<PathBuf>` field, making this check type-safe.

**Decision:** Track 1 uses Option A (add `is_local_path()` method). Track 2 will make it redundant.

### 1.3 Fix hardcoded paths in `info.rs` and `list.rs`

`info.rs:48-53` constructs `.agents/skills/{name}/SKILL.md` directly. `list.rs:39-44` and `list.rs:86-91` check `.agents/skills/{name}` for install status.

**Change:** Both commands need the merged options to resolve `skills_dir`. Both already load the manifest. Add a helper or inline the `skills_dir_or_default()` call:

```rust
// info.rs
let merged_options = ctx.merged_options(&manifest);
let skill_md = ctx.project_dir
    .join(merged_options.skills_dir_or_default())
    .join(name)
    .join("SKILL.md");
```

Same pattern for `list.rs`.

### 1.4 Fix silent `entry.resolve()` errors

`update.rs:24-25` uses `entry.resolve().ok()?` inside `filter_map`, silently dropping malformed manifest entries. `list.rs:26` does the same in JSON mode.

**Change:** Log a warning and continue, matching the principle that `ion update`/`ion list` shouldn't crash on one bad entry but shouldn't silently ignore it:

```rust
// update.rs
.filter_map(|(skill_name, entry)| {
    match entry.resolve() {
        Ok(source) => Some((skill_name.clone(), source)),
        Err(e) => {
            eprintln!("Warning: skipping '{}': {}", skill_name, e);
            None
        }
    }
})
```

### 1.5 Add `unregister_from_registry()` to `install_shared`

`remove.rs:117-127` manually computes the URL hash and calls `registry.unregister()`. This is the mirror of `register_in_registry()` which already exists in `install_shared.rs`.

**Change:** Add to `install_shared.rs`:

```rust
pub fn unregister_from_registry(
    source: &SkillSource,
    project_dir: &Path,
) -> anyhow::Result<()> {
    if matches!(source.source_type, SourceType::Github | SourceType::Git) {
        if let Ok(url) = source.git_url() {
            let repo_hash = format!("{:x}", hash_simple(&url));
            let project_str = project_dir.display().to_string();
            let mut registry = Registry::load()?;
            registry.unregister(&repo_hash, &project_str);
            registry.save()?;
        }
    }
    Ok(())
}
```

Replace the inline code in `remove.rs` with a call to this.

---

## Track 2: Type Redesign

### 2.1 `LockedSkill` → enum-based kind

**Current** (lockfile.rs:7-27):
```rust
pub struct LockedSkill {
    pub name: String,
    pub source: String,
    pub path: Option<String>,
    pub version: Option<String>,
    pub commit: Option<String>,
    pub checksum: Option<String>,
    pub binary: Option<String>,
    pub binary_version: Option<String>,
    pub binary_checksum: Option<String>,
    pub dev: Option<bool>,
}
```

**Serde constraint:** The `toml` crate does not support `#[serde(flatten)]` combined with `#[serde(tag)]`. We use a flat intermediate struct (`RawLockedSkill`) for serialization, and the proper enum-based `LockedSkill` for the API. Conversion happens via `From`/`Into` impls.

**API type (used by all code):**
```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LockedSkillKind {
    Git {
        commit: String,
        checksum: String,
    },
    Binary {
        binary_name: String,
        /// None for dev-mode builds where version is unknown/0.0.0.
        binary_version: Option<String>,
        /// None for dev-mode builds (no release artifact to checksum).
        binary_checksum: Option<String>,
        dev: bool,
    },
    Local {
        checksum: Option<String>,
    },
    Http {
        checksum: Option<String>,
    },
    Path {
        checksum: Option<String>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LockedSkill {
    pub name: String,
    pub source: String,
    pub path: Option<String>,
    pub version: Option<String>,
    pub kind: LockedSkillKind,
}
```

**Serde intermediate (private, used only for lockfile I/O):**
```rust
#[derive(Serialize, Deserialize)]
struct RawLockedSkill {
    name: String,
    source: String,
    kind: String,  // "git", "binary", "local", "http", "path"
    #[serde(default, skip_serializing_if = "Option::is_none")]
    path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    version: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    commit: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    checksum: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    binary_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    binary_version: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    binary_checksum: Option<String>,
    #[serde(default, skip_serializing_if = "is_false")]
    dev: bool,
}

fn is_false(v: &bool) -> bool { !v }

impl TryFrom<RawLockedSkill> for LockedSkill {
    type Error = String;
    fn try_from(raw: RawLockedSkill) -> Result<Self, String> {
        // Match on raw.kind to construct the correct LockedSkillKind variant.
        // Unknown kind values produce: Err("unknown locked skill kind '...'")
        ...
    }
}
impl From<LockedSkill> for RawLockedSkill { ... }
```

The `Lockfile` struct uses `#[serde(try_from = "RawLockfile", into = "RawLockfile")]` to route through the flat representation. The conversion is fallible: `impl TryFrom<RawLockedSkill> for LockedSkill` with `type Error = String` matches on the `kind` string to construct the correct `LockedSkillKind` variant. Unknown `kind` values produce a descriptive error ("unknown locked skill kind '...' — you may need to update Ion"). This error surfaces as a serde deserialization error, which `Lockfile::from_file()` catches and wraps with the "run `ion install` to regenerate" guidance message.

Note: `version` stays on the outer struct because both git and binary skills can have a SKILL.md-declared version.

**Builder methods** to replace the 8+ manual construction sites:

```rust
impl LockedSkill {
    pub fn git(name: impl Into<String>, source: impl Into<String>, commit: String, checksum: String) -> Self {
        Self {
            name: name.into(),
            source: source.into(),
            path: None,
            version: None,
            kind: LockedSkillKind::Git { commit, checksum },
        }
    }

    pub fn binary(
        name: impl Into<String>,
        source: impl Into<String>,
        binary_name: impl Into<String>,
        binary_version: Option<String>,
        binary_checksum: Option<String>,
    ) -> Self {
        Self {
            name: name.into(),
            source: source.into(),
            path: None,
            version: None,
            kind: LockedSkillKind::Binary {
                binary_name: binary_name.into(),
                binary_version,
                binary_checksum,
                dev: false,
            },
        }
    }

    pub fn local(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            source: String::new(),
            path: None,
            version: None,
            kind: LockedSkillKind::Local { checksum: None },
        }
    }

    // Builder-style setters
    pub fn with_path(mut self, path: impl Into<String>) -> Self { ... }
    pub fn with_version(mut self, version: impl Into<String>) -> Self { ... }

    /// Mark a binary skill as dev-mode. Panics if kind is not Binary.
    pub fn with_dev(mut self) -> Self {
        match &mut self.kind {
            LockedSkillKind::Binary { dev, .. } => *dev = true,
            _ => panic!("with_dev() called on non-binary LockedSkill"),
        }
        self
    }
}
```

**Where `with_dev()` is called:** `installer.rs::install_binary_dev()` currently sets `dev: Some(true)`. After the redesign, this becomes:

```rust
// installer.rs::install_binary_dev
let version = Some(info.version).filter(|v| v != "0.0.0");
Ok(LockedSkill::binary(name, &source.source, binary_name, version, None)
    .with_version(...)
    .with_dev())
```

Dev-mode binary skills have `binary_version: None` (or filtered 0.0.0) and `binary_checksum: None` since there's no release artifact. `BinaryUpdater::check()` handles `None` version by always reporting an update available.
```

**Convenience accessors** for common cross-variant queries:

```rust
impl LockedSkill {
    pub fn is_binary(&self) -> bool { matches!(self.kind, LockedSkillKind::Binary { .. }) }
    pub fn binary_name(&self) -> Option<&str> { ... }
    pub fn binary_version(&self) -> Option<&str> { ... }
    pub fn commit(&self) -> Option<&str> { ... }
    pub fn checksum(&self) -> Option<&str> { ... }
}
```

**Lockfile format change:** Since we chose to break the format (option b), the new TOML will use `kind = "git"` / `kind = "binary"` / `kind = "local"` tags. Running `ion install` regenerates the lockfile. Existing `Ion.lock` files without a `kind` field will fail to parse — this is acceptable.

**Migration path:** When `Lockfile::from_file()` encounters a parse error, emit a clear message:

```
Ion.lock format has changed. Run `ion install` to regenerate it.
```

### 2.2 `SkillSource` → per-type kind

**Current** (source.rs:18-29):
```rust
pub struct SkillSource {
    pub source_type: SourceType,
    pub source: String,
    pub path: Option<String>,
    pub rev: Option<String>,
    pub version: Option<String>,
    pub binary: Option<String>,
    pub asset_pattern: Option<String>,
    pub forked_from: Option<String>,
    pub dev: bool,
}
```

**New:**
```rust
/// Per-source-type data that only makes sense for that variant.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SkillSourceKind {
    Github,
    Git,
    Http,
    Path,
    Binary {
        binary_name: String,
        asset_pattern: Option<String>,
        /// If set, this is a local binary project (build from source).
        /// None means a remote binary (download from GitHub Releases / URL).
        local_project: Option<PathBuf>,
        dev: bool,
    },
    Local {
        forked_from: Option<String>,
    },
}

/// A fully resolved skill source.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkillSource {
    /// The raw source string (URL, path, or owner/repo shorthand).
    pub source: String,
    /// Subdirectory path within the source (for multi-skill repos).
    pub path: Option<String>,
    /// Pinned revision (git commit, tag, or branch).
    pub rev: Option<String>,
    /// Required SKILL.md version.
    pub version: Option<String>,
    /// Source-type-specific data.
    pub kind: SkillSourceKind,
}
```

**Key changes:**
- `source_type` field replaced by `kind` enum (carry data, not just a tag)
- `binary`, `asset_pattern`, `dev` move into `SkillSourceKind::Binary`
- `forked_from` moves into `SkillSourceKind::Local`
- Shared fields (`source`, `path`, `rev`, `version`) stay on the outer struct

**`SourceType` enum repurposed.** The existing `SourceType` enum is kept as a lightweight serde-only type for `Ion.toml` deserialization (it appears in `SkillEntry::Full { source_type: Option<SourceType>, ... }`). However, it is no longer carried on `SkillSource` — `SkillEntry::resolve()` converts `SourceType` → `SkillSourceKind` during resolution. Code that previously matched on `source.source_type` matches on `source.kind` instead. Convenience methods bridge common checks:

```rust
impl SkillSource {
    pub fn is_github(&self) -> bool { matches!(self.kind, SkillSourceKind::Github) }
    pub fn is_git_based(&self) -> bool { matches!(self.kind, SkillSourceKind::Github | SkillSourceKind::Git) }
    pub fn is_binary(&self) -> bool { matches!(self.kind, SkillSourceKind::Binary { .. }) }
    pub fn is_local(&self) -> bool { matches!(self.kind, SkillSourceKind::Local { .. }) }
    pub fn is_path(&self) -> bool { matches!(self.kind, SkillSourceKind::Path) }
    pub fn is_local_path(&self) -> bool {
        // True if source points to a filesystem path (Path type, or local Binary project)
        matches!(self.kind, SkillSourceKind::Path)
            || matches!(self.kind, SkillSourceKind::Binary { local_project: Some(_), .. })
    }
}
```

**`infer()` updated** to return the correct variant. `SourceType` references throughout the codebase replaced with pattern matches on `SkillSourceKind`.

**Manifest serialization impact:** `SkillEntry::resolve()` constructs `SkillSource`. The `type = "local"` / `type = "binary"` entries in `Ion.toml` map to the new enum variants. `SkillEntry` itself doesn't change — the `resolve()` method output changes.

### 2.3 `repo_dir_for_source()` helper

The hash-to-repo-path computation is duplicated in:
- `installer.rs:433-435` (`fetch_skill_base`)
- `update/git.rs:18-20` (`GitUpdater::check`)
- `update/git.rs:44-46` (`GitUpdater::apply`)

**Change:** Add a public function to `installer.rs`:

```rust
/// Compute the cached repo directory for a git-based source.
pub fn repo_dir_for_source(source: &SkillSource) -> Result<PathBuf> {
    let url = source.git_url()?;
    let repo_hash = format!("{:x}", hash_simple(&url));
    Ok(data_dir().join(&repo_hash))
}
```

Replace all three sites. `cached_repo_path()` becomes a thin wrapper:

```rust
pub fn cached_repo_path(source: &SkillSource) -> Option<PathBuf> {
    repo_dir_for_source(source).ok().filter(|p| p.exists())
}
```

---

## Track 3: `ProjectContext` Enrichment

### 3.1 `ctx.paint()` method

Every command does `let p = Paint::new(&ctx.global_config);` after loading context.

**Change:**
```rust
impl ProjectContext {
    pub fn paint(&self) -> Paint {
        Paint::new(&self.global_config)
    }
}
```

### 3.2 `ctx.skills_dir()` method

Commands that need the resolved skills directory currently do:
```rust
let merged_options = ctx.merged_options(&manifest);
let skills_dir = merged_options.skills_dir.as_deref().unwrap_or(".agents/skills");
```

**Change:**
```rust
impl ProjectContext {
    /// Resolved skills directory path (absolute).
    pub fn skills_dir(&self, manifest: &Manifest) -> PathBuf {
        let options = self.merged_options(manifest);
        self.project_dir.join(options.skills_dir_or_default())
    }

    /// Absolute path to a specific skill's directory.
    pub fn skill_path(&self, manifest: &Manifest, name: &str) -> PathBuf {
        self.skills_dir(manifest).join(name)
    }
}
```

### 3.3 `ctx.installer()` method

Every command that installs skills creates an installer the same way:
```rust
let merged_options = ctx.merged_options(&manifest);
let installer = SkillInstaller::new(&ctx.project_dir, &merged_options);
```

Since `SkillInstaller` borrows `project_dir` and `options`, and `merged_options` is computed from `manifest`, we can provide a convenience method. However, lifetime management is tricky — the installer borrows `options` which is a local. Instead, provide a helper that takes the pre-computed options:

```rust
impl ProjectContext {
    pub fn installer<'a>(&'a self, options: &'a ManifestOptions) -> SkillInstaller<'a> {
        SkillInstaller::new(&self.project_dir, options)
    }
}
```

This is a small win but used in 8+ commands, so it reduces noise.

---

## Track 4: Command Pipeline Extraction

### 4.1 Post-install bookkeeping — two separate helpers

The post-install sequence varies between two contexts:
- **`ion add` / `ion link`** (adding new skills): needs gitignore + registry + **manifest write** + lockfile upsert
- **`ion install`** (installing existing skills): needs gitignore + registry + lockfile upsert (skills are already in Ion.toml)

These are NOT the same operation. Extract two helpers to `install_shared.rs`:

The core building block is `finalize_skill_install`, which takes an options struct to control which steps run:

```rust
pub struct FinalizeOptions {
    /// Write a new entry to Ion.toml (false for `ion install` since skills are already declared).
    pub write_manifest: bool,
    /// Register in global registry (false when caller handles registry once at batch end).
    pub register_in_registry: bool,
}

/// Post-install bookkeeping: conditionally does gitignore, registry, manifest, lockfile.
pub fn finalize_skill_install(
    ctx: &ProjectContext,
    merged_options: &ManifestOptions,
    name: &str,
    source: &SkillSource,
    locked: LockedSkill,
    lockfile: &mut Lockfile,
    opts: &FinalizeOptions,
) -> anyhow::Result<()> {
    add_gitignore_entries(&ctx.project_dir, name, source, merged_options)?;
    if opts.register_in_registry {
        register_in_registry(source, &ctx.project_dir)?;
    }
    if opts.write_manifest {
        manifest_writer::add_skill(&ctx.manifest_path, name, source)?;
    }
    lockfile.upsert(locked);
    Ok(())
}
```

Common presets:

```rust
impl FinalizeOptions {
    /// For `ion add` (single skill): write manifest, register.
    pub const ADD: Self = Self { write_manifest: true, register_in_registry: true };
    /// For `ion install` (skills already in Ion.toml): no manifest write, register.
    pub const INSTALL: Self = Self { write_manifest: false, register_in_registry: true };
    /// For `ion add` collection loop: write manifest per-skill, registry once at end.
    pub const ADD_COLLECTION: Self = Self { write_manifest: true, register_in_registry: false };
}
```

Note: lockfile is mutated but not written. The caller writes once at the end after processing all skills. This matches the deferred-write pattern used in `install.rs` and `add.rs::install_collection`.

For single-skill commands (`add` single path, `link`), add a convenience that also writes:

```rust
pub fn finalize_skill_install_and_write(
    ctx: &ProjectContext,
    merged_options: &ManifestOptions,
    name: &str,
    source: &SkillSource,
    locked: LockedSkill,
    opts: &FinalizeOptions,
) -> anyhow::Result<()> {
    let mut lockfile = ctx.lockfile()?;
    finalize_skill_install(ctx, merged_options, name, source, locked, &mut lockfile, opts)?;
    lockfile.write_to(&ctx.lockfile_path)?;
    Ok(())
}
```

**Call sites:**
- `add.rs::finish_single_install` → `finalize_skill_install_and_write(opts: ADD)`
- `add.rs::install_collection` install loops → `finalize_skill_install(opts: ADD_COLLECTION)` per-skill, then `register_in_registry()` once at end
- `install.rs` install loops → `finalize_skill_install(opts: INSTALL)`
- `link.rs` → `finalize_skill_install_and_write(opts: ADD)`

### 4.2 `ValidationBuckets` — shared validation phase

The validate-bucket pattern is duplicated between `add.rs::install_collection` (lines 270-300) and `install.rs::run` (lines 63-130).

**Important:** `install.rs` has a special code path for `SourceType::Local` skills (lines 72-106) that bypasses validation entirely — local skills are deployed directly from the project tree without fetch or validation. This pre-validation filter must be preserved. Callers must exclude local skills BEFORE passing entries to `ValidationBuckets::collect()`. The install command's local skill handling stays inline in `install.rs::run()`, and only non-local skills are passed to the shared pipeline.

```rust
/// Results of validating a batch of skills (excludes local skills — handle those separately).
pub struct ValidationBuckets {
    pub clean: Vec<SkillEntry>,
    pub warned: Vec<(SkillEntry, ValidationReport)>,
    pub errored: Vec<(String, ValidationReport)>,
}

impl ValidationBuckets {
    /// Validate a set of (name, source) pairs, bucketing by result.
    /// Callers MUST filter out Local skills before calling this — local skills
    /// bypass validation and use a separate deploy path.
    pub fn collect(
        installer: &SkillInstaller,
        skills: impl IntoIterator<Item = (String, SkillSource)>,
    ) -> anyhow::Result<Self> {
        let mut clean = Vec::new();
        let mut warned = Vec::new();
        let mut errored = Vec::new();

        for (name, source) in skills {
            match installer.validate(&name, &source) {
                Ok(report) if report.warning_count > 0 => {
                    warned.push((SkillEntry { name, source }, report));
                }
                Ok(_) => {
                    clean.push(SkillEntry { name, source });
                }
                Err(SkillError::ValidationFailed { report, .. }) => {
                    errored.push((name, report));
                }
                Err(e) => return Err(e.into()),
            }
        }

        Ok(Self { clean, warned, errored })
    }

    pub fn is_empty(&self) -> bool {
        self.clean.is_empty() && self.warned.is_empty() && self.errored.is_empty()
    }
}
```

### 4.3 `print_validation_summary()` — shared summary display

The "print clean/warned/errored status lines" block is duplicated in `add.rs` (lines 335-364) and `install.rs` (lines 133-155):

```rust
/// Print the validation summary for a batch.
pub fn print_validation_summary(
    p: &Paint,
    buckets: &ValidationBuckets,
) {
    for entry in &buckets.clean {
        println!("  {} {} - passed", p.success("✓"), p.bold(&entry.name));
    }
    for (entry, report) in &buckets.warned {
        println!(
            "  {} {} - {} warning(s)",
            p.warn("⚠"), p.bold(&entry.name), report.warning_count
        );
        for finding in &report.findings {
            println!("      {} [{}] {}", finding.severity, finding.checker, finding.message);
        }
    }
    for (name, report) in &buckets.errored {
        println!(
            "  {} {} - {} error(s), will be skipped",
            "✗", p.bold(name), report.error_count
        );
        for finding in &report.findings {
            println!("      {} [{}] {}", finding.severity, finding.checker, finding.message);
        }
    }
    println!();
}
```

### 4.4 `install_approved_skills()` — shared install phase

The "install clean + warned selections" loop is duplicated:

```rust
/// Install approved skills from validation buckets.
/// Returns the number of skills installed.
/// The `finalize` callback controls post-install bookkeeping — callers pass
/// either `finalize_new_skill` (for `add`) or `finalize_existing_skill` (for `install`).
pub fn install_approved_skills(
    installer: &SkillInstaller,
    buckets: &ValidationBuckets,
    warned_selections: &[bool],
    p: &Paint,
    json: bool,
    mut finalize: impl FnMut(&str, &SkillSource, LockedSkill) -> anyhow::Result<()>,
) -> anyhow::Result<usize> {
    let mut installed = 0;

    for entry in &buckets.clean {
        if !json {
            println!("  Installing {}...", p.bold(&format!("'{}'", entry.name)));
        }
        let locked = installer.install_with_options(
            &entry.name, &entry.source,
            InstallValidationOptions::default(),
        )?;
        finalize(&entry.name, &entry.source, locked)?;
        installed += 1;
    }

    for (i, (entry, _)) in buckets.warned.iter().enumerate() {
        if !warned_selections.get(i).copied().unwrap_or(false) {
            if !json {
                println!("  Skipping '{}' (deselected)", entry.name);
            }
            continue;
        }
        if !json {
            println!("  Installing {}...", p.bold(&format!("'{}'", entry.name)));
        }
        let locked = installer.install_with_options(
            &entry.name, &entry.source,
            InstallValidationOptions { skip_validation: false, allow_warnings: true },
        )?;
        finalize(&entry.name, &entry.source, locked)?;
        installed += 1;
    }

    Ok(installed)
}
```

Usage in `add.rs::install_collection`:
```rust
let installed = install_approved_skills(
    &installer, &buckets, &warned_selections, &p, json,
    |name, source, locked| {
        finalize_skill_install(ctx, merged_options, name, source, locked, &mut lockfile, &FinalizeOptions::ADD_COLLECTION)
    },
)?;
// Register once for the whole collection
register_in_registry(base_source, &ctx.project_dir)?;
```

Usage in `install.rs::run`:
```rust
let installed = install_approved_skills(
    &installer, &buckets, &warned_selections, &p, json,
    |name, source, locked| {
        finalize_skill_install(ctx, merged_options, name, source, locked, &mut lockfile, &FinalizeOptions::INSTALL)
    },
)?;
```

After this extraction:
- `add.rs::install_collection` drops from ~290 lines to ~80 (discovery + call shared functions + JSON output)
- `install.rs::run` drops from ~265 lines to ~80 (local skill handling + call shared functions + JSON output)

---

## Track 5: Config & Writer Consolidation

### 5.1 Move `set_project_value` into `manifest_writer`

`config.rs:143-198` contains `set_value_in_file()` which handles both global config *and* project `Ion.toml` writes. The `--project` flag in `config.rs` command routes through this. The project-specific part should live in `manifest_writer`:

**Change:** Add to `manifest_writer.rs`:

```rust
pub fn set_option(manifest_path: &Path, key: &str, value: &str) -> Result<()> { ... }
pub fn delete_option(manifest_path: &Path, key: &str) -> Result<()> { ... }
```

These handle the `[options]` section of `Ion.toml`. The `config` command routes project writes through these instead of `GlobalConfig::set_value_in_file`.

### 5.2 Centralize `migrate.rs` git operations

`migrate.rs:474-530` calls git directly via `std::process::Command` for `git add`, `git diff --cached`, `git commit`, `git rev-parse HEAD`.

**Change:** Add to `crates/ion-skill/src/git.rs`:

```rust
pub fn stage_files(repo_dir: &Path, files: &[&str]) -> Result<()> { ... }
pub fn has_staged_changes(repo_dir: &Path) -> Result<bool> { ... }
pub fn create_commit(repo_dir: &Path, message: &str) -> Result<String> { ... }
```

Replace the raw `Command` calls in `migrate.rs` with these. This keeps all git subprocess management in one place.

### 5.3 Normalize warning output

Currently both `eprintln!("Warning: ...")` and `log::warn!(...)` are used for non-fatal side effects. Standardize:

- **`log::warn!`** for internal issues (builtin skill deploy failure, agent symlink failure)
- **`eprintln!` via `p.warn()`** for user-visible warnings (validation findings, binary install warnings)

Audit all `eprintln!("Warning:` calls and route through `Paint::warn()` or `log::warn!` as appropriate.

---

## Files Modified Per Track

### Track 1
- `crates/ion-skill/src/manifest.rs` — add `DEFAULT_SKILLS_DIR`, `skills_dir_or_default()`
- `crates/ion-skill/src/source.rs` — add `is_local_path()` method
- `crates/ion-skill/src/installer.rs` — fix `skill_dir()` to use `skills_dir_or_default()` from options
- `crates/ion-skill/src/update/binary.rs` — use `installer.skill_dir()` instead of hardcoded path
- `src/commands/new.rs` — use `DEFAULT_SKILLS_DIR`
- `src/commands/eject.rs` — use `skills_dir_or_default()`
- `src/commands/install.rs` — use `skills_dir_or_default()`
- `src/tui/app.rs` — use `DEFAULT_SKILLS_DIR`
- `src/commands/info.rs` — fix hardcoded path
- `src/commands/list.rs` — fix hardcoded path
- `src/commands/update.rs` — warn on resolve errors
- `src/commands/install_shared.rs` — add `unregister_from_registry()`
- `src/commands/remove.rs` — use `unregister_from_registry()`

### Track 2
- `crates/ion-skill/src/lockfile.rs` — `LockedSkill` redesign + builders + `RawLockedSkill` serde bridge
- `crates/ion-skill/src/source.rs` — `SkillSource` redesign with `SkillSourceKind` enum
- `crates/ion-skill/src/installer.rs` — update all `LockedSkill` construction, add `repo_dir_for_source()`
- `crates/ion-skill/src/update/git.rs` — use `repo_dir_for_source()`, update `LockedSkill` construction
- `crates/ion-skill/src/update/binary.rs` — update `LockedSkill` construction
- `crates/ion-skill/src/manifest.rs` — `SourceType` kept for serde only, `SkillEntry::resolve()` returns new `SkillSource`
- `crates/ion-skill/src/manifest_writer.rs` — update `add_skill()` for new `SkillSource`
- `src/commands/remove.rs` — binary cleanup reads `locked.binary_name()` / `locked.binary_version()` accessors instead of direct field access (lines 130-142)
- `src/commands/update.rs` — `LockedSkill` fallback construction uses builder (lines 96-107)
- `src/commands/install.rs` — local skill `LockedSkill` construction uses `LockedSkill::local()` builder (lines 92-103)
- `src/commands/eject.rs` — `LockedSkill` update uses pattern match on kind (lines 114-120)
- Every other command file — update pattern matches from `SourceType` to `SkillSourceKind`
- All tests — update `LockedSkill` and `SkillSource` construction

### Track 3
- `src/context.rs` — add `paint()`, `skills_dir()`, `skill_path()`, `installer()`
- All command files — replace preamble boilerplate with new methods

### Track 4
- `src/commands/install_shared.rs` — add `commit_skill_install()`, `ValidationBuckets`, `install_approved_skills()`
- `src/commands/validation.rs` — add `print_validation_summary()`
- `src/commands/add.rs` — simplify using shared pipeline
- `src/commands/install.rs` — simplify using shared pipeline
- `src/commands/link.rs` — use `commit_skill_install_and_write()`
- `src/commands/migrate.rs` — use shared pipeline where applicable

### Track 5
- `crates/ion-skill/src/manifest_writer.rs` — add `set_option()`, `delete_option()`
- `crates/ion-skill/src/config.rs` — remove project-specific write logic
- `crates/ion-skill/src/git.rs` — add `stage_files()`, `has_staged_changes()`, `create_commit()`
- `src/commands/migrate.rs` — use centralized git ops
- Various command files — normalize warning output

---

## Testing Strategy

- **Track 1:** Existing integration tests should pass after constant/path fixes. Add a unit test for `skills_dir_or_default()`. Add a test that `info` and `list` work with custom `skills-dir`.
- **Track 2:** All existing `LockedSkill` and `SkillSource` unit tests must be rewritten for new constructors. Integration tests (`/tests/`) should pass after updating lockfile expectations. Add a test that old-format lockfiles produce a clear error message.
- **Track 3:** No new tests needed — this is pure refactoring. Existing tests validate behavior.
- **Track 4:** Extract tests from integration suite that cover the multi-skill install path. Verify the shared pipeline works for both `add` collection and `install` bulk cases.
- **Track 5:** Add unit tests for new `git.rs` functions. Existing `migrate.rs` integration tests validate the migration path.

## Risks

- **Track 2 blast radius:** Changing `LockedSkill` and `SkillSource` touches nearly every file. Mitigation: thorough `cargo clippy` + `cargo nextest run` between each sub-change within the track.
- **Lockfile format break:** Users with existing `Ion.lock` files will need to re-run `ion install`. Mitigation: clear error message in `Lockfile::from_file()` when parse fails — "Ion.lock format has changed. Run `ion install` to regenerate it."
- **`SourceType` kept for serde compatibility:** `SkillEntry::Full` in `Ion.toml` uses `type = "local"` / `type = "binary"` / etc. `SourceType` is kept as a serde-only enum for this purpose, but no longer appears on `SkillSource`. `SkillEntry::resolve()` converts `SourceType` → `SkillSourceKind`. Risk: if any code outside `manifest.rs` still references `SourceType` for non-serde purposes, it needs to switch to `SkillSourceKind` matching.
- **`RawLockedSkill` serde bridge complexity:** The `TryFrom<RawLockedSkill> for LockedSkill` conversion must handle unknown `kind` values gracefully (e.g., from a newer Ion version). Mitigation: `TryFrom` returns a descriptive error, and `#[serde(try_from)]` surfaces it as a deserialization error wrapped with guidance to update Ion or regenerate the lockfile.
- **Track 4 behavioral difference:** `install_approved_skills` with `finalize_new_skill` calls `register_in_registry` per-skill. For collections, this is redundant (same repo registered N times). This is harmless (registry is idempotent) but slightly wasteful. Acceptable for code simplicity.
- **Error handling policy:** `install.rs` uses `entry.resolve()?` (hard failure on bad entry), while `update.rs` and `list.rs` will use warn-and-skip after Track 1. Track 4's `ValidationBuckets::collect()` must decide which policy to use — it should match its caller (`install` = hard fail, `add` collection = hard fail). Document this in the function's contract.
