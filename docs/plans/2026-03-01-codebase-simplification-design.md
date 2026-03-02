# Codebase Simplification Design

Date: 2026-03-01

## Goal

Simplify the ion codebase by removing duplication, extracting shared abstractions, and decomposing long functions — without changing any features or UI behavior.

## Approach

Bottom-up: start with small, safe refactors that are independently testable, then build up to larger abstractions.

## Changes

### 1. Shared text wrapping utility

**Problem:** Two text-wrapping implementations exist — `search.rs:print_wrapped()` and `search_ui.rs:wrap_text()`.

**Solution:** Move `wrap_text()` to a shared `tui/util.rs` module. Refactor `print_wrapped()` in `search.rs` to use `wrap_text()` internally, adding truncation/ellipsis logic on top.

**Files:** `src/tui/util.rs` (new), `src/tui/mod.rs`, `src/tui/search_ui.rs`, `src/commands/search.rs`

### 2. TOML file loading helper

**Problem:** `Lockfile::from_file()` and `GlobalConfig::load_from()` duplicate the same read-file → handle-NotFound → parse-TOML pattern.

**Solution:** Add a `load_toml_or_default<T: DeserializeOwned + Default>(path: &Path) -> Result<T>` helper function in the `ion-skill` library. Both callers use this instead of duplicating the pattern.

**Files:** `crates/ion-skill/src/lib.rs` (or a new `util.rs`), `crates/ion-skill/src/lockfile.rs`, `crates/ion-skill/src/config.rs`

### 3. TUI terminal lifecycle helper

**Problem:** `config.rs:run_interactive()` and `search.rs:pick_and_install()` have identical terminal setup/teardown code (enable_raw_mode, EnterAlternateScreen, CrosstermBackend, Terminal, then cleanup).

**Solution:** Create `tui/terminal.rs` with a `run_tui<F, T>(body: F) -> anyhow::Result<T>` function that handles setup, calls the provided closure with a `&mut Terminal`, and ensures cleanup even on error. Both TUI entrypoints become ~5 lines.

**Files:** `src/tui/terminal.rs` (new), `src/tui/mod.rs`, `src/commands/config.rs`, `src/commands/search.rs`

### 4. Config list section printing dedup

**Problem:** `config.rs:run_list()` has nearly identical section-grouping print logic for project vs global config (lines 88-100 and 108-119).

**Solution:** Extract a `print_config_sections(values: &[(String, String)])` helper function. Both branches call it with their respective `.list_values()`.

**Files:** `src/commands/config.rs`

### 5. ProjectContext abstraction

**Problem:** Seven command files repeat: `current_dir()`, `.join("ion.toml")`, `.join("ion.lock")`, `GlobalConfig::load()`, `Manifest::from_file()`, `Lockfile::from_file()`, `resolve_targets()`.

**Solution:** Create `src/context.rs` with:

```rust
pub struct ProjectContext {
    pub project_dir: PathBuf,
    pub manifest_path: PathBuf,
    pub lockfile_path: PathBuf,
    pub global_config: GlobalConfig,
}

impl ProjectContext {
    pub fn load() -> anyhow::Result<Self>;
    pub fn manifest(&self) -> anyhow::Result<Manifest>;
    pub fn manifest_or_empty(&self) -> anyhow::Result<Manifest>;
    pub fn lockfile(&self) -> anyhow::Result<Lockfile>;
    pub fn merged_options(&self, manifest: &Manifest) -> ManifestOptions;
}
```

Manifest and lockfile are loaded lazily (on demand) since not all commands need both. Commands go from ~10 lines of boilerplate to `let ctx = ProjectContext::load()?;`.

**Files:** `src/context.rs` (new), `src/main.rs`, `src/commands/add.rs`, `src/commands/remove.rs`, `src/commands/install.rs`, `src/commands/list.rs`, `src/commands/info.rs`, `src/commands/config.rs`, `src/commands/migrate.rs`

### 6. SkillInstaller abstraction

**Problem:** `install_skill()` handles 5 responsibilities in one function (fetch, validate, copy, symlink, build lock entry). `install_skill()` and `uninstall_skill()` are standalone functions sharing implicit context.

**Solution:** Create a `SkillInstaller` struct:

```rust
pub struct SkillInstaller<'a> {
    project_dir: &'a Path,
    options: &'a ManifestOptions,
}

impl<'a> SkillInstaller<'a> {
    pub fn new(project_dir: &'a Path, options: &'a ManifestOptions) -> Self;
    pub fn install(&self, name: &str, source: &SkillSource) -> Result<LockedSkill>;
    pub fn uninstall(&self, name: &str) -> Result<()>;

    fn fetch(&self, source: &SkillSource) -> Result<PathBuf>;
    fn validate(&self, skill_dir: &Path) -> Result<SkillMetadata>;
    fn deploy(&self, name: &str, skill_dir: &Path) -> Result<()>;
    fn build_locked_entry(&self, name: &str, source: &SkillSource, meta: &SkillMetadata, skill_dir: &Path) -> Result<LockedSkill>;
}
```

This consolidates install and uninstall into one cohesive type with clear single-responsibility methods.

**Files:** `crates/ion-skill/src/installer.rs`, `src/commands/add.rs`, `src/commands/remove.rs`, `src/commands/install.rs`, `crates/ion-skill/src/migrate.rs`

## Implementation Order

1. Shared text wrapping (no dependencies)
2. TOML loading helper (no dependencies)
3. Config list printing dedup (no dependencies)
4. TUI terminal lifecycle helper (no dependencies)
5. ProjectContext (depends on nothing but touches many files)
6. SkillInstaller (depends on nothing but touches many files)

Steps 1-4 can be done in parallel. Steps 5 and 6 are independent of each other but should be done after 1-4 to minimize merge conflicts.

## Verification

- All existing tests must pass after each step
- `cargo clippy` must be clean
- No feature or UI changes — only internal restructuring
