# Config Command Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add `ion config` command with interactive TUI (ratatui) and non-interactive get/set/list subcommands.

**Architecture:** The config command has two modes: (1) non-interactive subcommands (`get`, `set`, `list`) that read/write config via `toml_edit` to preserve formatting, and (2) an interactive full-screen TUI built with `ratatui`/`crossterm` that lets users browse and edit config with keyboard navigation. The TUI has two tabs (Global/Project) switchable with arrow keys.

**Tech Stack:** Rust, clap (CLI), ratatui + crossterm (TUI), toml_edit (config mutation), existing ion-skill config/manifest modules.

**Design doc:** `docs/plans/2026-03-01-config-command-design.md`

---

## Task 1: Add ratatui and crossterm dependencies

**Files:**
- Modify: `Cargo.toml` (root workspace)

**Step 1: Add dependencies**

Add `ratatui` and `crossterm` to the root `Cargo.toml` `[dependencies]` section:

```toml
[dependencies]
anyhow = "1"
clap = { version = "4.5.60", features = ["string", "derive"] }
crossterm = "0.28"
ion-skill = { path = "crates/ion-skill" }
ratatui = "0.29"
```

**Step 2: Verify it compiles**

Run: `cargo check`
Expected: Compiles with no errors (dependencies download and resolve).

**Step 3: Commit**

```bash
git add Cargo.toml Cargo.lock
git commit -m "feat: add ratatui and crossterm dependencies for config TUI"
```

---

## Task 2: Add dot-notation get/set/delete to GlobalConfig

**Files:**
- Modify: `crates/ion-skill/src/config.rs`

This task adds three methods to `GlobalConfig` for programmatic config access using dot-notation keys like `targets.claude` or `ui.color`. These use `toml_edit` for format-preserving writes.

**Step 1: Write the failing tests**

Add these tests at the bottom of the `#[cfg(test)] mod tests` block in `crates/ion-skill/src/config.rs`:

```rust
#[test]
fn get_value_dot_notation() {
    let mut config = GlobalConfig::default();
    config.targets.insert("claude".to_string(), ".claude/skills".to_string());
    config.cache.max_age_days = Some(30);
    config.ui.color = Some(true);

    assert_eq!(config.get_value("targets.claude"), Some(".claude/skills".to_string()));
    assert_eq!(config.get_value("cache.max-age-days"), Some("30".to_string()));
    assert_eq!(config.get_value("ui.color"), Some("true".to_string()));
    assert_eq!(config.get_value("targets.nonexistent"), None);
    assert_eq!(config.get_value("invalid"), None);
}

#[test]
fn set_value_preserves_formatting() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.toml");
    std::fs::write(&path, "# My config\n[targets]\nclaude = \".claude/skills\"\n").unwrap();

    GlobalConfig::set_value_in_file(&path, "targets.cursor", ".cursor/skills").unwrap();

    let content = std::fs::read_to_string(&path).unwrap();
    // Comment preserved
    assert!(content.contains("# My config"));
    // New value added
    assert!(content.contains("cursor"));
    assert!(content.contains(".cursor/skills"));
    // Old value still there
    assert!(content.contains("claude"));
}

#[test]
fn set_value_creates_section() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.toml");
    std::fs::write(&path, "").unwrap();

    GlobalConfig::set_value_in_file(&path, "targets.claude", ".claude/skills").unwrap();

    let reloaded = GlobalConfig::load_from(&path).unwrap();
    assert_eq!(reloaded.targets["claude"], ".claude/skills");
}

#[test]
fn set_value_cache_and_ui() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.toml");
    std::fs::write(&path, "").unwrap();

    GlobalConfig::set_value_in_file(&path, "cache.max-age-days", "7").unwrap();
    GlobalConfig::set_value_in_file(&path, "ui.color", "false").unwrap();

    let reloaded = GlobalConfig::load_from(&path).unwrap();
    assert_eq!(reloaded.cache.max_age_days, Some(7));
    assert_eq!(reloaded.ui.color, Some(false));
}

#[test]
fn delete_value_from_file() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.toml");
    std::fs::write(&path, "[targets]\nclaude = \".claude/skills\"\ncursor = \".cursor/skills\"\n").unwrap();

    GlobalConfig::delete_value_in_file(&path, "targets.cursor").unwrap();

    let reloaded = GlobalConfig::load_from(&path).unwrap();
    assert_eq!(reloaded.targets.len(), 1);
    assert!(reloaded.targets.contains_key("claude"));
}

#[test]
fn set_value_invalid_key_format() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.toml");
    std::fs::write(&path, "").unwrap();

    let result = GlobalConfig::set_value_in_file(&path, "invalid", "value");
    assert!(result.is_err());
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo test -p ion-skill -- config::tests`
Expected: FAIL — methods `get_value`, `set_value_in_file`, `delete_value_in_file` don't exist.

**Step 3: Implement the methods**

Add to the `impl GlobalConfig` block in `crates/ion-skill/src/config.rs`, after the `save_to` method:

```rust
/// Get a config value by dot-notation key (e.g., "targets.claude", "ui.color").
/// Returns None if the section or key doesn't exist.
pub fn get_value(&self, key: &str) -> Option<String> {
    let (section, field) = key.split_once('.')?;
    match section {
        "targets" => self.targets.get(field).cloned(),
        "sources" => self.sources.get(field).cloned(),
        "cache" => match field {
            "max-age-days" => self.cache.max_age_days.map(|v| v.to_string()),
            _ => None,
        },
        "ui" => match field {
            "color" => self.ui.color.map(|v| v.to_string()),
            _ => None,
        },
        _ => None,
    }
}

/// Set a config value in a TOML file by dot-notation key, preserving formatting.
/// Creates the file and parent directories if they don't exist.
pub fn set_value_in_file(path: &Path, key: &str, value: &str) -> Result<()> {
    use toml_edit::{DocumentMut, Item, Table};

    let (section, field) = key.split_once('.').ok_or_else(|| {
        Error::Manifest(format!("Invalid key format '{key}': expected 'section.key'"))
    })?;

    // Validate section name
    match section {
        "targets" | "sources" | "cache" | "ui" => {}
        _ => {
            return Err(Error::Manifest(format!(
                "Unknown config section '{section}'. Valid sections: targets, sources, cache, ui"
            )));
        }
    }

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(Error::Io)?;
    }

    let content = std::fs::read_to_string(path).unwrap_or_default();
    let mut doc: DocumentMut = content.parse().map_err(Error::TomlEdit)?;

    if !doc.contains_key(section) {
        doc[section] = Item::Table(Table::new());
    }

    // For cache/ui, parse the value to the correct type
    match (section, field) {
        ("cache", "max-age-days") => {
            let num: i64 = value.parse().map_err(|_| {
                Error::Manifest(format!("'{value}' is not a valid integer for {key}"))
            })?;
            doc[section][field] = toml_edit::value(num);
        }
        ("ui", "color") => {
            let b: bool = value.parse().map_err(|_| {
                Error::Manifest(format!("'{value}' is not a valid boolean for {key}"))
            })?;
            doc[section][field] = toml_edit::value(b);
        }
        _ => {
            doc[section][field] = toml_edit::value(value);
        }
    }

    std::fs::write(path, doc.to_string()).map_err(Error::Io)?;
    Ok(())
}

/// Delete a config value from a TOML file by dot-notation key, preserving formatting.
pub fn delete_value_in_file(path: &Path, key: &str) -> Result<()> {
    use toml_edit::DocumentMut;

    let (section, field) = key.split_once('.').ok_or_else(|| {
        Error::Manifest(format!("Invalid key format '{key}': expected 'section.key'"))
    })?;

    let content = std::fs::read_to_string(path).map_err(Error::Io)?;
    let mut doc: DocumentMut = content.parse().map_err(Error::TomlEdit)?;

    if let Some(table) = doc.get_mut(section).and_then(|item| item.as_table_mut()) {
        table.remove(field);
    }

    std::fs::write(path, doc.to_string()).map_err(Error::Io)?;
    Ok(())
}

/// List all config values as a Vec of (dot-key, value) pairs.
pub fn list_values(&self) -> Vec<(String, String)> {
    let mut entries = Vec::new();
    for (k, v) in &self.targets {
        entries.push((format!("targets.{k}"), v.clone()));
    }
    for (k, v) in &self.sources {
        entries.push((format!("sources.{k}"), v.clone()));
    }
    if let Some(days) = self.cache.max_age_days {
        entries.push(("cache.max-age-days".to_string(), days.to_string()));
    }
    if let Some(color) = self.ui.color {
        entries.push(("ui.color".to_string(), color.to_string()));
    }
    entries
}
```

**Step 4: Run tests to verify they pass**

Run: `cargo test -p ion-skill -- config::tests`
Expected: All PASS.

**Step 5: Commit**

```bash
git add crates/ion-skill/src/config.rs
git commit -m "feat: add dot-notation get/set/delete methods to GlobalConfig"
```

---

## Task 3: Add dot-notation get/set for project-level config (ManifestOptions)

**Files:**
- Modify: `crates/ion-skill/src/manifest.rs`

Project config currently only has `[options.targets]`. We need get/set/delete/list for the project scope.

**Step 1: Write the failing tests**

Add to the `#[cfg(test)] mod tests` block in `crates/ion-skill/src/manifest.rs`:

```rust
#[test]
fn get_project_value() {
    let toml_str = "[skills]\n\n[options.targets]\nclaude = \".claude/skills\"\n";
    let manifest = Manifest::parse(toml_str).unwrap();
    assert_eq!(
        manifest.options.get_value("targets.claude"),
        Some(".claude/skills".to_string())
    );
    assert_eq!(manifest.options.get_value("targets.nonexistent"), None);
}

#[test]
fn list_project_values() {
    let toml_str = "[skills]\n\n[options.targets]\nclaude = \".claude/skills\"\ncursor = \".cursor/skills\"\n";
    let manifest = Manifest::parse(toml_str).unwrap();
    let values = manifest.options.list_values();
    assert_eq!(values.len(), 2);
    assert!(values.contains(&("targets.claude".to_string(), ".claude/skills".to_string())));
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo test -p ion-skill -- manifest::tests`
Expected: FAIL — methods don't exist.

**Step 3: Implement the methods**

Add an `impl ManifestOptions` block in `crates/ion-skill/src/manifest.rs`, after the struct definition:

```rust
impl ManifestOptions {
    /// Get a project config value by dot-notation key. Currently only targets are supported.
    pub fn get_value(&self, key: &str) -> Option<String> {
        let (section, field) = key.split_once('.')?;
        match section {
            "targets" => self.targets.get(field).cloned(),
            _ => None,
        }
    }

    /// List all project config values as (dot-key, value) pairs.
    pub fn list_values(&self) -> Vec<(String, String)> {
        self.targets
            .iter()
            .map(|(k, v)| (format!("targets.{k}"), v.clone()))
            .collect()
    }
}
```

For set/delete on the project file, we'll use `toml_edit` directly in the config command (similar to `manifest_writer.rs`), since the manifest file has a different structure (`[options.targets]` nested under `[options]`).

**Step 4: Run tests to verify they pass**

Run: `cargo test -p ion-skill -- manifest::tests`
Expected: All PASS.

**Step 5: Commit**

```bash
git add crates/ion-skill/src/manifest.rs
git commit -m "feat: add dot-notation get/list methods to ManifestOptions"
```

---

## Task 4: Wire up the Config command in clap and implement non-interactive subcommands

**Files:**
- Modify: `src/main.rs`
- Modify: `src/commands/mod.rs`
- Create: `src/commands/config.rs`

**Step 1: Add the Config variant to the Commands enum**

In `src/main.rs`, add to the `Commands` enum after `Migrate`:

```rust
/// Manage ion configuration
Config {
    #[command(subcommand)]
    action: Option<ConfigAction>,
},
```

Add a new enum below `Commands`:

```rust
#[derive(Subcommand)]
enum ConfigAction {
    /// Get a config value
    Get {
        /// Key in dot notation (e.g., targets.claude)
        key: String,
        /// Read from project config (ion.toml) instead of global
        #[arg(long)]
        project: bool,
    },
    /// Set a config value
    Set {
        /// Key in dot notation (e.g., targets.claude)
        key: String,
        /// Value to set
        value: String,
        /// Write to project config (ion.toml) instead of global
        #[arg(long)]
        project: bool,
    },
    /// List all config values
    List {
        /// Show project config (ion.toml) instead of global
        #[arg(long)]
        project: bool,
    },
}
```

Add the match arm in the `main()` function:

```rust
Commands::Config { action } => commands::config::run(action),
```

Update the function signature to accept: `run(action: Option<ConfigAction>)`. Since `ConfigAction` is defined in `main.rs`, pass it as the enum. Alternatively, define `ConfigAction` in `commands/config.rs` and re-export it. The cleaner approach: define the enum in `src/commands/config.rs` and import it in `main.rs`.

**Step 2: Create `src/commands/config.rs`**

```rust
use clap::Subcommand;
use ion_skill::config::GlobalConfig;
use ion_skill::manifest::Manifest;

#[derive(Subcommand)]
pub enum ConfigAction {
    /// Get a config value
    Get {
        /// Key in dot notation (e.g., targets.claude)
        key: String,
        /// Read from project config (ion.toml) instead of global
        #[arg(long)]
        project: bool,
    },
    /// Set a config value
    Set {
        /// Key in dot notation (e.g., targets.claude)
        key: String,
        /// Value to set
        value: String,
        /// Write to project config (ion.toml) instead of global
        #[arg(long)]
        project: bool,
    },
    /// List all config values
    List {
        /// Show project config (ion.toml) instead of global
        #[arg(long)]
        project: bool,
    },
}

pub fn run(action: Option<ConfigAction>) -> anyhow::Result<()> {
    match action {
        None => run_interactive(),
        Some(ConfigAction::Get { key, project }) => run_get(&key, project),
        Some(ConfigAction::Set { key, value, project }) => run_set(&key, &value, project),
        Some(ConfigAction::List { project }) => run_list(project),
    }
}

fn run_get(key: &str, project: bool) -> anyhow::Result<()> {
    if project {
        let manifest_path = std::env::current_dir()?.join("ion.toml");
        let manifest = Manifest::from_file(&manifest_path)?;
        match manifest.options.get_value(key) {
            Some(value) => println!("{value}"),
            None => {
                eprintln!("Key '{key}' not found in project config");
                std::process::exit(1);
            }
        }
    } else {
        let config = GlobalConfig::load()?;
        match config.get_value(key) {
            Some(value) => println!("{value}"),
            None => {
                eprintln!("Key '{key}' not found in global config");
                std::process::exit(1);
            }
        }
    }
    Ok(())
}

fn run_set(key: &str, value: &str, project: bool) -> anyhow::Result<()> {
    if project {
        let manifest_path = std::env::current_dir()?.join("ion.toml");
        set_project_value(&manifest_path, key, value)?;
        println!("Set {key} = \"{value}\" in project config");
    } else {
        let config_path = GlobalConfig::config_path()
            .ok_or_else(|| anyhow::anyhow!("Could not determine global config path"))?;
        GlobalConfig::set_value_in_file(&config_path, key, value)?;
        println!("Set {key} = \"{value}\" in global config");
    }
    Ok(())
}

fn run_list(project: bool) -> anyhow::Result<()> {
    if project {
        let manifest_path = std::env::current_dir()?.join("ion.toml");
        let manifest = Manifest::from_file(&manifest_path)?;
        let values = manifest.options.list_values();
        if values.is_empty() {
            println!("No project config values set.");
        } else {
            let mut current_section = "";
            for (key, value) in &values {
                let (section, field) = key.split_once('.').unwrap();
                if section != current_section {
                    if !current_section.is_empty() {
                        println!();
                    }
                    println!("[{section}]");
                    current_section = section;
                }
                println!("{field} = \"{value}\"");
            }
        }
    } else {
        let config = GlobalConfig::load()?;
        let values = config.list_values();
        if values.is_empty() {
            println!("No global config values set.");
        } else {
            let mut current_section = "";
            for (key, value) in &values {
                let (section, field) = key.split_once('.').unwrap();
                if section != current_section {
                    if !current_section.is_empty() {
                        println!();
                    }
                    println!("[{section}]");
                    current_section = section;
                }
                println!("{field} = \"{value}\"");
            }
        }
    }
    Ok(())
}

/// Set a value in the project manifest's [options] section using toml_edit.
fn set_project_value(
    manifest_path: &std::path::Path,
    key: &str,
    value: &str,
) -> anyhow::Result<()> {
    use toml_edit::{DocumentMut, Item, Table};

    let (section, field) = key.split_once('.').ok_or_else(|| {
        anyhow::anyhow!("Invalid key format '{key}': expected 'section.key'")
    })?;

    if section != "targets" {
        anyhow::bail!("Project config only supports 'targets' section, got '{section}'");
    }

    let content = std::fs::read_to_string(manifest_path)?;
    let mut doc: DocumentMut = content.parse()?;

    if !doc.contains_key("options") {
        doc["options"] = Item::Table(Table::new());
    }
    let options = doc["options"]
        .as_table_mut()
        .ok_or_else(|| anyhow::anyhow!("[options] is not a table"))?;

    if !options.contains_key(section) {
        options[section] = Item::Table(Table::new());
    }

    options[section][field] = toml_edit::value(value);
    std::fs::write(manifest_path, doc.to_string())?;
    Ok(())
}

fn run_interactive() -> anyhow::Result<()> {
    // Placeholder — implemented in Task 5+
    println!("Interactive config TUI not yet implemented. Use 'ion config get/set/list' subcommands.");
    Ok(())
}
```

**Step 3: Update `src/commands/mod.rs`**

```rust
pub mod add;
pub mod config;
pub mod info;
pub mod install;
pub mod list;
pub mod migrate;
pub mod remove;
```

**Step 4: Update `src/main.rs`**

The full updated `main.rs`:

```rust
use clap::{Parser, Subcommand};

mod commands;

#[derive(Parser)]
#[command(name = "ion", about = "Agent skill manager")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Add a skill to the project
    Add {
        /// Skill source (e.g., owner/repo/skill or git URL)
        source: String,
        /// Pin to a specific git ref (branch, tag, or commit SHA)
        #[arg(long)]
        rev: Option<String>,
    },
    /// Remove a skill from the project
    Remove {
        /// Name of the skill to remove
        name: String,
    },
    /// Install all skills from ion.toml
    Install,
    /// List installed skills
    List,
    /// Show detailed info about a skill
    Info {
        /// Skill source or name
        skill: String,
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
    /// Manage ion configuration
    Config {
        #[command(subcommand)]
        action: Option<commands::config::ConfigAction>,
    },
}

fn main() {
    let cli = Cli::parse();

    let result = match cli.command {
        Commands::Add { source, rev } => commands::add::run(&source, rev.as_deref()),
        Commands::Remove { name } => commands::remove::run(&name),
        Commands::Install => commands::install::run(),
        Commands::List => commands::list::run(),
        Commands::Info { skill } => commands::info::run(&skill),
        Commands::Migrate { from, dry_run } => commands::migrate::run(from.as_deref(), dry_run),
        Commands::Config { action } => commands::config::run(action),
    };

    if let Err(e) = result {
        eprintln!("Error: {e}");
        std::process::exit(1);
    }
}
```

**Step 5: Verify it compiles and works**

Run: `cargo check`
Expected: Compiles with no errors.

Run: `cargo run -- config list`
Expected: Prints config values (or "No global config values set.").

**Step 6: Commit**

```bash
git add src/main.rs src/commands/mod.rs src/commands/config.rs
git commit -m "feat: add config command with get/set/list subcommands"
```

---

## Task 5: Build TUI app state module

**Files:**
- Create: `src/tui/mod.rs`
- Create: `src/tui/app.rs`

**Step 1: Create `src/tui/mod.rs`**

```rust
pub mod app;
pub mod event;
pub mod ui;
```

**Step 2: Create `src/tui/app.rs`**

This is the core state for the TUI. It holds data for both tabs and tracks navigation.

```rust
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use ion_skill::config::GlobalConfig;
use ion_skill::manifest::Manifest;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    Global,
    Project,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputMode {
    Normal,
    EditingValue,
    AddingKey,
    AddingValue,
    ConfirmDelete,
    ConfirmQuit,
}

/// A section with its entries displayed in the TUI.
#[derive(Debug, Clone)]
pub struct ConfigSection {
    pub name: String,
    pub entries: Vec<(String, String)>,
}

/// The full TUI application state.
pub struct App {
    pub tab: Tab,
    pub input_mode: InputMode,
    pub global_sections: Vec<ConfigSection>,
    pub project_sections: Vec<ConfigSection>,
    pub cursor: usize,
    pub input_buffer: String,
    pub adding_key_buffer: String,
    pub dirty: bool,
    pub has_project: bool,
    pub global_config_path: PathBuf,
    pub manifest_path: Option<PathBuf>,
    pub status_message: Option<String>,
    pub should_quit: bool,
}

impl App {
    pub fn new(global_config_path: PathBuf, manifest_path: Option<PathBuf>) -> anyhow::Result<Self> {
        let global_config = GlobalConfig::load_from(&global_config_path).unwrap_or_default();
        let global_sections = Self::build_global_sections(&global_config);

        let (project_sections, has_project) = match &manifest_path {
            Some(mp) if mp.exists() => {
                let manifest = Manifest::from_file(mp)?;
                (Self::build_project_sections(&manifest), true)
            }
            _ => (Vec::new(), false),
        };

        Ok(Self {
            tab: Tab::Global,
            input_mode: InputMode::Normal,
            global_sections,
            project_sections,
            cursor: 0,
            input_buffer: String::new(),
            adding_key_buffer: String::new(),
            dirty: false,
            has_project,
            global_config_path,
            manifest_path,
            status_message: None,
            should_quit: false,
        })
    }

    fn build_global_sections(config: &GlobalConfig) -> Vec<ConfigSection> {
        let mut sections = Vec::new();

        if !config.targets.is_empty() {
            sections.push(ConfigSection {
                name: "targets".to_string(),
                entries: config.targets.iter().map(|(k, v)| (k.clone(), v.clone())).collect(),
            });
        }

        if !config.sources.is_empty() {
            sections.push(ConfigSection {
                name: "sources".to_string(),
                entries: config.sources.iter().map(|(k, v)| (k.clone(), v.clone())).collect(),
            });
        }

        // Always show cache and ui sections (with defaults)
        sections.push(ConfigSection {
            name: "cache".to_string(),
            entries: vec![(
                "max-age-days".to_string(),
                config.cache.max_age_days.map_or("(unset)".to_string(), |v| v.to_string()),
            )],
        });

        sections.push(ConfigSection {
            name: "ui".to_string(),
            entries: vec![(
                "color".to_string(),
                config.ui.color.map_or("(unset)".to_string(), |v| v.to_string()),
            )],
        });

        sections
    }

    fn build_project_sections(manifest: &Manifest) -> Vec<ConfigSection> {
        let mut sections = Vec::new();

        sections.push(ConfigSection {
            name: "targets".to_string(),
            entries: manifest.options.targets.iter().map(|(k, v)| (k.clone(), v.clone())).collect(),
        });

        sections
    }

    /// Get the sections for the current tab.
    pub fn current_sections(&self) -> &[ConfigSection] {
        match self.tab {
            Tab::Global => &self.global_sections,
            Tab::Project => &self.project_sections,
        }
    }

    /// Get mutable sections for the current tab.
    pub fn current_sections_mut(&mut self) -> &mut Vec<ConfigSection> {
        match self.tab {
            Tab::Global => &mut self.global_sections,
            Tab::Project => &mut self.project_sections,
        }
    }

    /// Total number of navigable entries across all sections in the current tab.
    pub fn total_entries(&self) -> usize {
        self.current_sections().iter().map(|s| s.entries.len()).sum()
    }

    /// Get the section index and entry index for the current cursor position.
    pub fn cursor_position(&self) -> Option<(usize, usize)> {
        let mut remaining = self.cursor;
        for (si, section) in self.current_sections().iter().enumerate() {
            if remaining < section.entries.len() {
                return Some((si, remaining));
            }
            remaining -= section.entries.len();
        }
        None
    }

    /// Get the dot-notation key and value at the current cursor position.
    pub fn current_entry(&self) -> Option<(String, String)> {
        let (si, ei) = self.cursor_position()?;
        let section = &self.current_sections()[si];
        let (key, value) = &section.entries[ei];
        Some((format!("{}.{}", section.name, key), value.clone()))
    }

    /// Get the section name at the current cursor position.
    pub fn current_section_name(&self) -> Option<String> {
        let (si, _) = self.cursor_position()?;
        Some(self.current_sections()[si].name.clone())
    }

    /// Save all changes to disk.
    pub fn save(&mut self) -> anyhow::Result<()> {
        // Rebuild global config from sections and save
        let config = self.sections_to_global_config();
        config.save_to(&self.global_config_path)?;

        // Save project config if applicable
        if let Some(ref mp) = self.manifest_path {
            if mp.exists() {
                self.save_project_config(mp)?;
            }
        }

        self.dirty = false;
        self.status_message = Some("Saved!".to_string());
        Ok(())
    }

    fn sections_to_global_config(&self) -> GlobalConfig {
        let mut config = GlobalConfig::default();
        for section in &self.global_sections {
            match section.name.as_str() {
                "targets" => {
                    for (k, v) in &section.entries {
                        config.targets.insert(k.clone(), v.clone());
                    }
                }
                "sources" => {
                    for (k, v) in &section.entries {
                        config.sources.insert(k.clone(), v.clone());
                    }
                }
                "cache" => {
                    for (k, v) in &section.entries {
                        if k == "max-age-days" && v != "(unset)" {
                            config.cache.max_age_days = v.parse().ok();
                        }
                    }
                }
                "ui" => {
                    for (k, v) in &section.entries {
                        if k == "color" && v != "(unset)" {
                            config.ui.color = v.parse().ok();
                        }
                    }
                }
                _ => {}
            }
        }
        config
    }

    fn save_project_config(&self, manifest_path: &Path) -> anyhow::Result<()> {
        use toml_edit::{DocumentMut, Item, Table};

        let content = std::fs::read_to_string(manifest_path)?;
        let mut doc: DocumentMut = content.parse()?;

        // Rebuild [options.targets] from project_sections
        if !doc.contains_key("options") {
            doc["options"] = Item::Table(Table::new());
        }
        let options = doc["options"].as_table_mut()
            .ok_or_else(|| anyhow::anyhow!("[options] is not a table"))?;

        // Clear existing targets and rebuild
        options["targets"] = Item::Table(Table::new());
        let targets_table = options["targets"].as_table_mut().unwrap();

        for section in &self.project_sections {
            if section.name == "targets" {
                for (k, v) in &section.entries {
                    targets_table[k.as_str()] = toml_edit::value(v.as_str());
                }
            }
        }

        std::fs::write(manifest_path, doc.to_string())?;
        Ok(())
    }
}
```

**Step 3: Update `src/main.rs` to declare the tui module**

Add `mod tui;` after `mod commands;` in `src/main.rs`.

**Step 4: Verify it compiles**

Run: `cargo check`
Expected: Compiles (the tui module is declared but event.rs and ui.rs don't exist yet, so we need stub files).

Create stub files:

`src/tui/event.rs`:
```rust
// Event handling — implemented in Task 7
```

`src/tui/ui.rs`:
```rust
// UI rendering — implemented in Task 6
```

Run: `cargo check`
Expected: Compiles with no errors.

**Step 5: Commit**

```bash
git add src/main.rs src/tui/
git commit -m "feat: add TUI app state module for config command"
```

---

## Task 6: Build TUI rendering (ui.rs)

**Files:**
- Modify: `src/tui/ui.rs`

**Step 1: Implement the rendering function**

Replace the stub in `src/tui/ui.rs` with:

```rust
use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Tabs};

use super::app::{App, InputMode, Tab};

pub fn render(frame: &mut Frame, app: &App) {
    let area = frame.area();

    // Layout: tabs bar, main content, status/input bar, help bar
    let chunks = Layout::vertical([
        Constraint::Length(3), // tabs
        Constraint::Min(3),   // content
        Constraint::Length(1), // status / input
        Constraint::Length(2), // help
    ])
    .split(area);

    render_tabs(frame, app, chunks[0]);
    render_content(frame, app, chunks[1]);
    render_status(frame, app, chunks[2]);
    render_help(frame, app, chunks[3]);
}

fn render_tabs(frame: &mut Frame, app: &App, area: Rect) {
    let titles = vec!["Global", "Project"];
    let selected = match app.tab {
        Tab::Global => 0,
        Tab::Project => 1,
    };

    let tabs = Tabs::new(titles)
        .block(Block::default().borders(Borders::ALL).title(" Ion Config "))
        .select(selected)
        .style(Style::default().fg(Color::White))
        .highlight_style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
        .divider("|");

    frame.render_widget(tabs, area);
}

fn render_content(frame: &mut Frame, app: &App, area: Rect) {
    let sections = app.current_sections();

    if app.tab == Tab::Project && !app.has_project {
        let msg = Paragraph::new("No ion.toml found in current directory.")
            .style(Style::default().fg(Color::DarkGray))
            .block(Block::default().borders(Borders::LEFT | Borders::RIGHT));
        frame.render_widget(msg, area);
        return;
    }

    if sections.is_empty() || app.total_entries() == 0 {
        let msg = Paragraph::new("No config values set. Press 'a' to add one.")
            .style(Style::default().fg(Color::DarkGray))
            .block(Block::default().borders(Borders::LEFT | Borders::RIGHT));
        frame.render_widget(msg, area);
        return;
    }

    let mut lines: Vec<Line> = Vec::new();
    let mut entry_index = 0;

    for section in sections {
        // Section header
        lines.push(Line::from(Span::styled(
            format!("  [{}]", section.name),
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
        )));

        for (key, value) in &section.entries {
            let is_selected = entry_index == app.cursor;

            let prefix = if is_selected { "  > " } else { "    " };
            let dots = ".".repeat(30usize.saturating_sub(key.len()));

            let style = if is_selected {
                Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };

            let value_style = if value == "(unset)" {
                Style::default().fg(Color::DarkGray)
            } else if is_selected {
                Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::Green)
            };

            lines.push(Line::from(vec![
                Span::styled(format!("{prefix}{key} "), style),
                Span::styled(format!("{dots} "), Style::default().fg(Color::DarkGray)),
                Span::styled(value.to_string(), value_style),
            ]));

            entry_index += 1;
        }

        lines.push(Line::from(""));
    }

    let content = Paragraph::new(lines)
        .block(Block::default().borders(Borders::LEFT | Borders::RIGHT));
    frame.render_widget(content, area);
}

fn render_status(frame: &mut Frame, app: &App, area: Rect) {
    let content = match app.input_mode {
        InputMode::EditingValue => {
            let (key, _) = app.current_entry().unwrap_or_default();
            Line::from(vec![
                Span::styled("Edit value for ", Style::default().fg(Color::Yellow)),
                Span::styled(key, Style::default().fg(Color::Cyan)),
                Span::styled(": ", Style::default().fg(Color::Yellow)),
                Span::raw(&app.input_buffer),
                Span::styled("█", Style::default().fg(Color::White)),
            ])
        }
        InputMode::AddingKey => {
            Line::from(vec![
                Span::styled("New key: ", Style::default().fg(Color::Yellow)),
                Span::raw(&app.input_buffer),
                Span::styled("█", Style::default().fg(Color::White)),
            ])
        }
        InputMode::AddingValue => {
            Line::from(vec![
                Span::styled(
                    format!("Value for {}: ", app.adding_key_buffer),
                    Style::default().fg(Color::Yellow),
                ),
                Span::raw(&app.input_buffer),
                Span::styled("█", Style::default().fg(Color::White)),
            ])
        }
        InputMode::ConfirmDelete => {
            let (key, _) = app.current_entry().unwrap_or_default();
            Line::from(Span::styled(
                format!("Delete '{key}'? (y/n)"),
                Style::default().fg(Color::Red),
            ))
        }
        InputMode::ConfirmQuit => {
            Line::from(Span::styled(
                "Unsaved changes. Save before quitting? (y/n/Esc cancel)",
                Style::default().fg(Color::Red),
            ))
        }
        InputMode::Normal => {
            if let Some(ref msg) = app.status_message {
                Line::from(Span::styled(msg.clone(), Style::default().fg(Color::Green)))
            } else if app.dirty {
                Line::from(Span::styled(
                    " [unsaved changes]",
                    Style::default().fg(Color::Yellow),
                ))
            } else {
                Line::from("")
            }
        }
    };

    let paragraph = Paragraph::new(content);
    frame.render_widget(paragraph, area);
}

fn render_help(frame: &mut Frame, app: &App, area: Rect) {
    let help_text = match app.input_mode {
        InputMode::EditingValue | InputMode::AddingKey | InputMode::AddingValue => {
            "Enter Confirm  Esc Cancel"
        }
        InputMode::ConfirmDelete => "y Confirm  n Cancel",
        InputMode::ConfirmQuit => "y Save & quit  n Quit without saving  Esc Cancel",
        InputMode::Normal => "↑↓ Navigate  ←→ Tab  Enter Edit  a Add  d Delete  s Save  q Quit",
    };

    let help = Paragraph::new(Line::from(Span::styled(
        format!(" {help_text}"),
        Style::default().fg(Color::DarkGray),
    )));
    frame.render_widget(help, area);
}
```

**Step 2: Verify it compiles**

Run: `cargo check`
Expected: Compiles with no errors.

**Step 3: Commit**

```bash
git add src/tui/ui.rs
git commit -m "feat: implement TUI rendering for config command"
```

---

## Task 7: Build TUI event handling (event.rs)

**Files:**
- Modify: `src/tui/event.rs`

**Step 1: Implement the event handler**

Replace the stub in `src/tui/event.rs` with:

```rust
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::app::{App, InputMode, Tab};

/// Handle a key event and mutate app state. Returns Ok(()) on success.
pub fn handle_key(app: &mut App, key: KeyEvent) -> anyhow::Result<()> {
    // Ctrl+C always quits immediately
    if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
        app.should_quit = true;
        return Ok(());
    }

    match app.input_mode {
        InputMode::Normal => handle_normal(app, key),
        InputMode::EditingValue => handle_editing(app, key),
        InputMode::AddingKey => handle_adding_key(app, key),
        InputMode::AddingValue => handle_adding_value(app, key),
        InputMode::ConfirmDelete => handle_confirm_delete(app, key),
        InputMode::ConfirmQuit => handle_confirm_quit(app, key),
    }
}

fn handle_normal(app: &mut App, key: KeyEvent) -> anyhow::Result<()> {
    match key.code {
        KeyCode::Char('q') | KeyCode::Esc => {
            if app.dirty {
                app.input_mode = InputMode::ConfirmQuit;
            } else {
                app.should_quit = true;
            }
        }
        KeyCode::Up => {
            if app.total_entries() > 0 && app.cursor > 0 {
                app.cursor -= 1;
            }
            app.status_message = None;
        }
        KeyCode::Down => {
            let max = app.total_entries().saturating_sub(1);
            if app.cursor < max {
                app.cursor += 1;
            }
            app.status_message = None;
        }
        KeyCode::Left => {
            app.tab = Tab::Global;
            app.cursor = 0;
            app.status_message = None;
        }
        KeyCode::Right => {
            app.tab = Tab::Project;
            app.cursor = 0;
            app.status_message = None;
        }
        KeyCode::Enter => {
            if app.total_entries() > 0 {
                if let Some((_, value)) = app.current_entry() {
                    app.input_buffer = if value == "(unset)" {
                        String::new()
                    } else {
                        value
                    };
                    app.input_mode = InputMode::EditingValue;
                }
            }
        }
        KeyCode::Char('a') => {
            // Can't add to project tab if no project
            if app.tab == Tab::Project && !app.has_project {
                app.status_message = Some("No ion.toml in current directory.".to_string());
                return Ok(());
            }
            app.input_buffer.clear();
            app.input_mode = InputMode::AddingKey;
        }
        KeyCode::Char('d') => {
            if app.total_entries() > 0 {
                app.input_mode = InputMode::ConfirmDelete;
            }
        }
        KeyCode::Char('s') => {
            if app.dirty {
                app.save()?;
            } else {
                app.status_message = Some("No changes to save.".to_string());
            }
        }
        _ => {}
    }
    Ok(())
}

fn handle_editing(app: &mut App, key: KeyEvent) -> anyhow::Result<()> {
    match key.code {
        KeyCode::Enter => {
            // Apply the edit
            if let Some((si, ei)) = app.cursor_position() {
                let new_value = app.input_buffer.clone();
                app.current_sections_mut()[si].entries[ei].1 = new_value;
                app.dirty = true;
                app.status_message = Some("Value updated.".to_string());
            }
            app.input_buffer.clear();
            app.input_mode = InputMode::Normal;
        }
        KeyCode::Esc => {
            app.input_buffer.clear();
            app.input_mode = InputMode::Normal;
        }
        KeyCode::Backspace => {
            app.input_buffer.pop();
        }
        KeyCode::Char(c) => {
            app.input_buffer.push(c);
        }
        _ => {}
    }
    Ok(())
}

fn handle_adding_key(app: &mut App, key: KeyEvent) -> anyhow::Result<()> {
    match key.code {
        KeyCode::Enter => {
            if app.input_buffer.is_empty() {
                app.input_mode = InputMode::Normal;
                return Ok(());
            }

            // If the key contains a dot (e.g., "targets.newkey"), parse it
            // Otherwise, add to the current section
            let (section_name, field_name) = if let Some((s, f)) = app.input_buffer.split_once('.') {
                (s.to_string(), f.to_string())
            } else if let Some(name) = app.current_section_name() {
                (name, app.input_buffer.clone())
            } else {
                // No sections exist yet — default to "targets"
                ("targets".to_string(), app.input_buffer.clone())
            };

            app.adding_key_buffer = format!("{section_name}.{field_name}");
            app.input_buffer.clear();
            app.input_mode = InputMode::AddingValue;
        }
        KeyCode::Esc => {
            app.input_buffer.clear();
            app.input_mode = InputMode::Normal;
        }
        KeyCode::Backspace => {
            app.input_buffer.pop();
        }
        KeyCode::Char(c) => {
            app.input_buffer.push(c);
        }
        _ => {}
    }
    Ok(())
}

fn handle_adding_value(app: &mut App, key: KeyEvent) -> anyhow::Result<()> {
    match key.code {
        KeyCode::Enter => {
            let full_key = app.adding_key_buffer.clone();
            let value = app.input_buffer.clone();

            if let Some((section_name, field_name)) = full_key.split_once('.') {
                // Find or create the section
                let sections = app.current_sections_mut();
                let section = sections.iter_mut().find(|s| s.name == section_name);

                if let Some(section) = section {
                    section.entries.push((field_name.to_string(), value));
                } else {
                    sections.push(super::app::ConfigSection {
                        name: section_name.to_string(),
                        entries: vec![(field_name.to_string(), value)],
                    });
                }
                app.dirty = true;
                app.status_message = Some(format!("Added {full_key}."));
            }

            app.input_buffer.clear();
            app.adding_key_buffer.clear();
            app.input_mode = InputMode::Normal;
        }
        KeyCode::Esc => {
            app.input_buffer.clear();
            app.adding_key_buffer.clear();
            app.input_mode = InputMode::Normal;
        }
        KeyCode::Backspace => {
            app.input_buffer.pop();
        }
        KeyCode::Char(c) => {
            app.input_buffer.push(c);
        }
        _ => {}
    }
    Ok(())
}

fn handle_confirm_delete(app: &mut App, key: KeyEvent) -> anyhow::Result<()> {
    match key.code {
        KeyCode::Char('y') => {
            if let Some((si, ei)) = app.cursor_position() {
                let sections = app.current_sections_mut();
                let removed_key = sections[si].entries.remove(ei).0;

                // Remove empty sections (except cache/ui which always show)
                if sections[si].entries.is_empty()
                    && sections[si].name != "cache"
                    && sections[si].name != "ui"
                {
                    sections.remove(si);
                }

                // Adjust cursor
                let total = app.total_entries();
                if total > 0 && app.cursor >= total {
                    app.cursor = total - 1;
                }

                app.dirty = true;
                app.status_message = Some(format!("Deleted '{removed_key}'."));
            }
            app.input_mode = InputMode::Normal;
        }
        KeyCode::Char('n') | KeyCode::Esc => {
            app.input_mode = InputMode::Normal;
        }
        _ => {}
    }
    Ok(())
}

fn handle_confirm_quit(app: &mut App, key: KeyEvent) -> anyhow::Result<()> {
    match key.code {
        KeyCode::Char('y') => {
            app.save()?;
            app.should_quit = true;
        }
        KeyCode::Char('n') => {
            app.should_quit = true;
        }
        KeyCode::Esc => {
            app.input_mode = InputMode::Normal;
        }
        _ => {}
    }
    Ok(())
}
```

**Step 2: Verify it compiles**

Run: `cargo check`
Expected: Compiles with no errors.

**Step 3: Commit**

```bash
git add src/tui/event.rs
git commit -m "feat: implement TUI event handling for config command"
```

---

## Task 8: Wire up the TUI main loop and connect to config command

**Files:**
- Modify: `src/commands/config.rs` (replace `run_interactive` placeholder)

**Step 1: Implement `run_interactive` with the ratatui main loop**

Replace the `run_interactive()` function in `src/commands/config.rs`:

```rust
fn run_interactive() -> anyhow::Result<()> {
    use std::io;
    use crossterm::event::{self, Event};
    use crossterm::execute;
    use crossterm::terminal::{
        EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
    };
    use ratatui::Terminal;
    use ratatui::backend::CrosstermBackend;

    use crate::tui::app::App;
    use crate::tui::event::handle_key;
    use crate::tui::ui::render;

    let global_config_path = GlobalConfig::config_path()
        .ok_or_else(|| anyhow::anyhow!("Could not determine global config path"))?;

    let project_dir = std::env::current_dir()?;
    let manifest_path = project_dir.join("ion.toml");
    let manifest_opt = if manifest_path.exists() {
        Some(manifest_path)
    } else {
        None
    };

    let mut app = App::new(global_config_path, manifest_opt)?;

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Main loop
    let result = loop {
        terminal.draw(|frame| render(frame, &app))?;

        if let Event::Key(key) = event::read()? {
            if let Err(e) = handle_key(&mut app, key) {
                app.status_message = Some(format!("Error: {e}"));
            }
        }

        if app.should_quit {
            break Ok(());
        }
    };

    // Restore terminal
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    result
}
```

**Step 2: Verify it compiles**

Run: `cargo check`
Expected: Compiles with no errors.

**Step 3: Manually test**

Run: `cargo run -- config`
Expected: Full-screen TUI appears with tabs, keyboard navigation works, editing works, q quits.

Run: `cargo run -- config set targets.test-target .test/skills`
Run: `cargo run -- config get targets.test-target`
Expected: Prints `.test/skills`

Run: `cargo run -- config list`
Expected: Shows all config values.

**Step 4: Commit**

```bash
git add src/commands/config.rs
git commit -m "feat: wire up interactive TUI main loop for config command"
```

---

## Task 9: Integration tests

**Files:**
- Create: `tests/config_integration.rs`

**Step 1: Write integration tests for the non-interactive commands**

```rust
use std::process::Command;

fn ion_cmd() -> Command {
    Command::new(env!("CARGO_BIN_EXE_ion"))
}

#[test]
fn config_set_and_get_global() {
    let dir = tempfile::tempdir().unwrap();
    let config_path = dir.path().join("config.toml");

    // We can't easily override the global config path in integration tests,
    // so test via the library directly
    use ion_skill::config::GlobalConfig;

    GlobalConfig::set_value_in_file(&config_path, "targets.claude", ".claude/skills").unwrap();
    let config = GlobalConfig::load_from(&config_path).unwrap();
    assert_eq!(config.targets["claude"], ".claude/skills");
}

#[test]
fn config_set_and_get_project() {
    let dir = tempfile::tempdir().unwrap();
    let manifest_path = dir.path().join("ion.toml");
    std::fs::write(&manifest_path, "[skills]\n").unwrap();

    // Test config list via command in project dir
    let output = ion_cmd()
        .args(["config", "list", "--project"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    // Should succeed (empty project config is fine)
    assert!(output.status.success(), "stdout: {}, stderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr));
}

#[test]
fn config_list_no_project() {
    let dir = tempfile::tempdir().unwrap();
    // No ion.toml exists

    let output = ion_cmd()
        .args(["config", "list", "--project"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    // Should fail — no ion.toml
    assert!(!output.status.success());
}

#[test]
fn config_get_nonexistent_key() {
    let dir = tempfile::tempdir().unwrap();

    let output = ion_cmd()
        .args(["config", "get", "targets.nonexistent"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    // Should fail — key not found
    assert!(!output.status.success());
}
```

**Step 2: Run tests**

Run: `cargo test --test config_integration`
Expected: All PASS.

**Step 3: Commit**

```bash
git add tests/config_integration.rs
git commit -m "test: add integration tests for config command"
```

---

## Task 10: Final verification and cleanup

**Step 1: Run all tests**

Run: `cargo test`
Expected: All tests pass.

**Step 2: Run clippy**

Run: `cargo clippy -- -D warnings`
Expected: No warnings.

**Step 3: Manual smoke test of TUI**

Run: `cargo run -- config`
Verify:
- Tabs switch with ←/→
- Entries navigate with ↑/↓
- Enter edits a value
- 'a' adds a key
- 'd' deletes with confirmation
- 's' saves
- 'q' quits (prompts if unsaved)

**Step 4: Final commit if any cleanup was needed**

```bash
git add -A
git commit -m "chore: final cleanup for config command"
```
