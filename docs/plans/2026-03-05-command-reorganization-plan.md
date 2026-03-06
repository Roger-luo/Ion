# Command Reorganization Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Reorganize 16 flat CLI commands into 5 top-level + 3 subcommand groups (skill, project, cache) for a cleaner help output.

**Architecture:** The `Commands` enum in `main.rs` gets three new nested subcommand enums (`SkillCommands`, `ProjectCommands`, `CacheCommands`), following the existing `ConfigAction` pattern. Command module files stay in `src/commands/` — only the clap routing changes. `ion add` absorbs `ion install` by making `source` optional.

**Tech Stack:** Rust, clap derive API (nested `#[derive(Subcommand)]` enums)

---

### Task 1: Merge `install` into `add` (make source optional)

**Files:**
- Modify: `src/main.rs` (the `Add` variant in `Commands` enum)
- Modify: `src/commands/add.rs` (the `run` function signature and top of function)

**Step 1: Update `Add` variant in `main.rs` to make `source` optional**

In `src/main.rs`, change the `Add` variant from:

```rust
    /// Add a skill to the project
    Add {
        /// Skill source (e.g., owner/repo/skill or git URL)
        source: String,
        /// Pin to a specific git ref (branch, tag, or commit SHA)
        #[arg(long)]
        rev: Option<String>,
        /// Install as a binary CLI skill from GitHub Releases
        #[arg(long)]
        bin: bool,
    },
```

to:

```rust
    /// Add skills to the project, or install all from Ion.toml
    Add {
        /// Skill source (e.g., owner/repo/skill or git URL). Omit to install all from Ion.toml.
        source: Option<String>,
        /// Pin to a specific git ref (branch, tag, or commit SHA)
        #[arg(long)]
        rev: Option<String>,
        /// Install as a binary CLI skill from GitHub Releases
        #[arg(long)]
        bin: bool,
    },
```

**Step 2: Update the dispatch in `main.rs`**

Change the match arm from:

```rust
Commands::Add { source, rev, bin } => commands::add::run(&source, rev.as_deref(), bin),
```

to:

```rust
Commands::Add { source, rev, bin } => {
    match source {
        Some(src) => commands::add::run(&src, rev.as_deref(), bin),
        None => commands::install::run(),
    }
}
```

**Step 3: Remove the `Install` variant from `Commands` enum**

Delete:

```rust
    /// Install all skills from Ion.toml
    Install,
```

and its match arm:

```rust
Commands::Install => commands::install::run(),
```

**Step 4: Run tests to verify**

Run: `cargo test`
Expected: All tests pass except those that invoke `ion install` directly (those will be fixed in Task 6).

**Step 5: Commit**

```bash
git add src/main.rs
git commit -m "feat: merge install into add (source becomes optional)"
```

---

### Task 2: Add `ion skill` subcommand group

**Files:**
- Modify: `src/main.rs`

**Step 1: Add `SkillCommands` enum and replace individual variants**

In `src/main.rs`, add a new enum above `Commands`:

```rust
#[derive(Subcommand)]
enum SkillCommands {
    /// Create a new skill or skill collection
    New {
        /// Target directory (default: current directory)
        #[arg(long)]
        path: Option<String>,
        /// Also run `cargo init --bin` to scaffold a Rust CLI project
        #[arg(long)]
        bin: bool,
        /// Create a multi-skill collection with a skills/ directory
        #[arg(long)]
        collection: bool,
        /// Overwrite existing files
        #[arg(long)]
        force: bool,
    },
    /// Validate local skill definitions
    Validate {
        /// Optional path to a SKILL.md file or skill/workspace directory
        path: Option<String>,
    },
    /// Show detailed info about a skill
    Info {
        /// Skill source or name
        skill: String,
    },
    /// List installed skills
    List,
    /// Link a local skill directory into the project
    Link {
        /// Path to the local skill directory containing SKILL.md
        path: String,
    },
}
```

**Step 2: Replace the five individual variants in `Commands` with a single `Skill` variant**

Remove `New`, `Validate`, `Info`, `List`, `Link` from `Commands`. Add:

```rust
    /// Create, inspect, and validate skills
    Skill {
        #[command(subcommand)]
        action: SkillCommands,
    },
```

**Step 3: Update the dispatch in `main()`**

Remove the five individual match arms. Add:

```rust
Commands::Skill { action } => match action {
    SkillCommands::New { path, bin, collection, force } => commands::new::run(path.as_deref(), bin, collection, force),
    SkillCommands::Validate { path } => commands::validate::run(path.as_deref()),
    SkillCommands::Info { skill } => commands::info::run(&skill),
    SkillCommands::List => commands::list::run(),
    SkillCommands::Link { path } => commands::link::run(&path),
},
```

**Step 4: Run build to verify**

Run: `cargo build`
Expected: Compiles successfully.

**Step 5: Commit**

```bash
git add src/main.rs
git commit -m "feat: add ion skill subcommand group"
```

---

### Task 3: Add `ion project` subcommand group

**Files:**
- Modify: `src/main.rs`

**Step 1: Add `ProjectCommands` enum**

```rust
#[derive(Subcommand)]
enum ProjectCommands {
    /// Initialize Ion.toml with agent tool targets
    Init {
        /// Configure specific targets (e.g. claude, cursor, or name:path)
        #[arg(long, short = 't')]
        target: Vec<String>,
        /// Overwrite existing [options.targets] without prompting
        #[arg(long)]
        force: bool,
    },
    /// Migrate skills from skills-lock.json or existing directories
    Migrate {
        /// Path to skills-lock.json (defaults to ./skills-lock.json)
        #[arg(long)]
        from: Option<String>,
        /// Show what would be migrated without writing files
        #[arg(long)]
        dry_run: bool,
    },
}
```

**Step 2: Replace `Init` and `Migrate` in `Commands` with a single `Project` variant**

```rust
    /// Project setup and migration
    Project {
        #[command(subcommand)]
        action: ProjectCommands,
    },
```

**Step 3: Update dispatch**

```rust
Commands::Project { action } => match action {
    ProjectCommands::Init { target, force } => commands::init::run(&target, force),
    ProjectCommands::Migrate { from, dry_run } => commands::migrate::run(from.as_deref(), dry_run),
},
```

**Step 4: Run build**

Run: `cargo build`
Expected: Compiles.

**Step 5: Commit**

```bash
git add src/main.rs
git commit -m "feat: add ion project subcommand group"
```

---

### Task 4: Add `ion cache` subcommand group

**Files:**
- Modify: `src/main.rs`

**Step 1: Add `CacheCommands` enum**

```rust
#[derive(Subcommand)]
enum CacheCommands {
    /// Garbage collect stale skill repos from global storage
    Gc {
        /// Show what would be cleaned without deleting
        #[arg(long)]
        dry_run: bool,
    },
}
```

**Step 2: Replace `Gc` in `Commands` with `Cache`**

```rust
    /// Manage the skill cache
    Cache {
        #[command(subcommand)]
        action: CacheCommands,
    },
```

**Step 3: Update dispatch**

```rust
Commands::Cache { action } => match action {
    CacheCommands::Gc { dry_run } => commands::gc::run(dry_run),
},
```

**Step 4: Run build**

Run: `cargo build`
Expected: Compiles.

**Step 5: Commit**

```bash
git add src/main.rs
git commit -m "feat: add ion cache subcommand group"
```

---

### Task 5: Verify main.rs is clean

**Files:**
- Review: `src/main.rs`

**Step 1: Verify the final `Commands` enum has exactly these variants**

```rust
#[derive(Subcommand)]
enum Commands {
    /// Add skills to the project, or install all from Ion.toml
    Add { ... },
    /// Remove a skill from the project
    Remove { ... },
    /// Search for skills across registries and GitHub
    Search { ... },
    /// Update skills to their latest versions
    Update { ... },
    /// Run a binary skill
    Run { ... },
    /// Create, inspect, and validate skills
    Skill { ... },
    /// Project setup and migration
    Project { ... },
    /// Manage the skill cache
    Cache { ... },
    /// Manage ion configuration
    Config { ... },
}
```

**Step 2: Run full build and verify help output**

Run: `cargo build && cargo run -- --help`
Expected: Help shows exactly the 9 top-level entries from the design doc (add, remove, search, update, run, skill, project, cache, config).

**Step 3: Verify subcommand help**

Run: `cargo run -- skill --help`
Expected: Shows new, validate, info, list, link.

Run: `cargo run -- project --help`
Expected: Shows init, migrate.

Run: `cargo run -- cache --help`
Expected: Shows gc.

---

### Task 6: Update integration tests

This task updates all test files to use the new command paths. The changes are mechanical: insert the subcommand group name before the old subcommand name.

**Files:**
- Modify: `tests/integration.rs`
- Modify: `tests/validate_integration.rs`
- Modify: `tests/new_integration.rs`
- Modify: `tests/migrate_integration.rs`
- Modify: `tests/update_integration.rs`

**Step 1: Update `tests/integration.rs`**

Changes:
- `.args(["list"])` → `.args(["skill", "list"])`
- `.args(["info", ...])` → `.args(["skill", "info", ...])`
- `.args(["install"])` → `.args(["add"])` (no source = install all)
- `.args(["init", ...])` → `.args(["project", "init", ...])`
- `.args(["link", ...])` → `.args(["skill", "link", ...])`
- `help_shows_all_commands` test: update assertions — remove `install`, `list`, `info`, `validate`, `init`; add `skill`, `project`, `cache`

**Step 2: Update `tests/validate_integration.rs`**

Changes:
- `.args(["validate", ...])` → `.args(["skill", "validate", ...])`
- `validate_help_is_exposed`: `.args(["validate", "--help"])` → `.args(["skill", "validate", "--help"])`

**Step 3: Update `tests/new_integration.rs`**

Changes:
- `.args(["new", ...])` → `.args(["skill", "new", ...])`
- `new_help_is_exposed`: `.args(["new", "--help"])` → `.args(["skill", "new", "--help"])`

**Step 4: Update `tests/migrate_integration.rs`**

Changes:
- `.args(["migrate", ...])` → `.args(["project", "migrate", ...])`
- `help_shows_migrate`: change assertion from `stdout.contains("migrate")` to `stdout.contains("project")`

**Step 5: Update `tests/update_integration.rs`**

Changes:
- `.args(["install"])` → `.args(["add"])` (install-all invocations)
- `update` invocations stay unchanged (still top-level)

**Step 6: Run all tests**

Run: `cargo test`
Expected: All tests pass.

**Step 7: Commit**

```bash
git add tests/
git commit -m "test: update integration tests for command reorganization"
```

---

### Task 7: Clean up — remove dead install command module reference

**Files:**
- Modify: `src/commands/mod.rs`

**Step 1: Check if `install.rs` is still referenced**

`install.rs` is still called from the `Add`/`None` dispatch path in `main.rs`, so the module stays. But verify `pub mod install;` is still needed.

It IS still needed because `commands::install::run()` is called when `ion add` has no source argument. No change needed — just verify.

**Step 2: Run final full test suite**

Run: `cargo test`
Expected: All tests pass.

Run: `cargo clippy`
Expected: No new warnings.

**Step 3: Commit (if any cleanup was needed)**

```bash
git commit -m "chore: clean up after command reorganization"
```
