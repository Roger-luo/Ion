# Codebase Simplification Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Remove duplication, extract shared abstractions, and decompose long functions across the ion codebase without changing any features or UI.

**Architecture:** Bottom-up refactoring — start with leaf utilities (text wrapping, TOML loading), then shared infrastructure (TUI terminal helper, config printing), then major abstractions (ProjectContext, SkillInstaller). Each task is independently testable; all existing tests must pass after every commit.

**Tech Stack:** Rust 2024 edition, ratatui, crossterm, toml/toml_edit, serde, clap, thiserror

---

### Task 1: Shared text wrapping utility

**Files:**
- Create: `src/tui/util.rs`
- Modify: `src/tui/mod.rs:1-6`
- Modify: `src/tui/search_ui.rs:180-201` (remove `wrap_text` function)
- Modify: `src/commands/search.rs:200-242` (rewrite `print_wrapped` to use shared `wrap_text`)

**Step 1: Create `src/tui/util.rs` with the `wrap_text` function**

```rust
/// Word-wrap text to fit within a given width.
/// Returns a vector of lines, each fitting within `width` characters.
pub fn wrap_text(text: &str, width: usize) -> Vec<String> {
    if width == 0 {
        return vec![text.to_string()];
    }
    let mut lines = Vec::new();
    let mut current = String::new();
    for word in text.split_whitespace() {
        if current.is_empty() {
            current = word.to_string();
        } else if current.len() + 1 + word.len() <= width {
            current.push(' ');
            current.push_str(word);
        } else {
            lines.push(current);
            current = word.to_string();
        }
    }
    if !current.is_empty() {
        lines.push(current);
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wrap_text_basic() {
        let result = wrap_text("hello world foo bar", 11);
        assert_eq!(result, vec!["hello world", "foo bar"]);
    }

    #[test]
    fn wrap_text_zero_width() {
        let result = wrap_text("hello world", 0);
        assert_eq!(result, vec!["hello world"]);
    }

    #[test]
    fn wrap_text_single_word() {
        let result = wrap_text("hello", 80);
        assert_eq!(result, vec!["hello"]);
    }

    #[test]
    fn wrap_text_empty() {
        let result = wrap_text("", 80);
        assert!(result.is_empty());
    }

    #[test]
    fn wrap_text_exact_width() {
        let result = wrap_text("ab cd", 5);
        assert_eq!(result, vec!["ab cd"]);
    }
}
```

**Step 2: Register module in `src/tui/mod.rs`**

Add `pub mod util;` to `src/tui/mod.rs`.

**Step 3: Update `src/tui/search_ui.rs` to use shared `wrap_text`**

Replace the local `wrap_text` function (lines 180-201) by importing the shared one. Change:
- Remove the entire `fn wrap_text(...)` at the bottom of the file
- Add `use super::util::wrap_text;` to the imports

**Step 4: Rewrite `print_wrapped` in `src/commands/search.rs` to use shared `wrap_text`**

Replace `print_wrapped` (lines 200-242) with:

```rust
fn print_wrapped(text: &str, indent: usize, width: usize, max_lines: usize, color: bool) {
    let prefix: String = " ".repeat(indent);
    let lines = crate::tui::util::wrap_text(text, width);

    for (i, line) in lines.iter().take(max_lines).enumerate() {
        let is_last_allowed = i + 1 == max_lines;
        let has_more = i + 1 < lines.len();

        let display = if is_last_allowed && has_more {
            // Truncate and add ellipsis
            let limit = width.saturating_sub(3);
            let truncated = if line.len() > limit {
                &line[..limit]
            } else {
                line.as_str()
            };
            format!("{truncated}...")
        } else {
            line.clone()
        };

        if color {
            use crossterm::style::Stylize;
            println!("{prefix}{}", display.cyan());
        } else {
            println!("{prefix}{display}");
        }
    }
}
```

**Step 5: Run tests to verify**

Run: `cargo test`
Expected: All tests pass.

Run: `cargo clippy`
Expected: No warnings.

**Step 6: Commit**

```bash
git add src/tui/util.rs src/tui/mod.rs src/tui/search_ui.rs src/commands/search.rs
git commit -m "refactor: extract shared wrap_text utility into tui::util"
```

---

### Task 2: TOML file loading helper

**Files:**
- Modify: `crates/ion-skill/src/lib.rs:1-16`
- Modify: `crates/ion-skill/src/lockfile.rs:28-33`
- Modify: `crates/ion-skill/src/config.rs:68-74`

**Step 1: Add `load_toml_or_default` helper to `crates/ion-skill/src/lib.rs`**

Add after the existing `pub type Result<T>` line:

```rust
use std::path::Path;
use serde::de::DeserializeOwned;

/// Load a TOML file and deserialize it. Returns `T::default()` if file doesn't exist.
pub fn load_toml_or_default<T: DeserializeOwned + Default>(path: &Path) -> Result<T> {
    match std::fs::read_to_string(path) {
        Ok(content) => toml::from_str(&content).map_err(Error::TomlParse),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(T::default()),
        Err(e) => Err(Error::Io(e)),
    }
}
```

**Step 2: Update `Lockfile::from_file` in `crates/ion-skill/src/lockfile.rs`**

Replace lines 28-34 with:

```rust
    pub fn from_file(path: &Path) -> Result<Self> {
        crate::load_toml_or_default(path)
    }
```

**Step 3: Update `GlobalConfig::load_from` in `crates/ion-skill/src/config.rs`**

Replace lines 68-74 with:

```rust
    pub fn load_from(path: &Path) -> Result<Self> {
        crate::load_toml_or_default(path)
    }
```

**Step 4: Run tests to verify**

Run: `cargo test`
Expected: All existing tests pass (both lockfile and config tests already cover missing-file and parsing).

Run: `cargo clippy`
Expected: No warnings.

**Step 5: Commit**

```bash
git add crates/ion-skill/src/lib.rs crates/ion-skill/src/lockfile.rs crates/ion-skill/src/config.rs
git commit -m "refactor: extract load_toml_or_default helper to deduplicate TOML loading"
```

---

### Task 3: Config list section printing dedup

**Files:**
- Modify: `src/commands/config.rs:80-122`

**Step 1: Extract `print_config_sections` helper in `src/commands/config.rs`**

Add this function before `run_list`:

```rust
fn print_config_sections(values: &[(String, String)]) {
    let mut current_section = "";
    for (key, value) in values {
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
```

**Step 2: Replace `run_list` body**

Replace lines 80-122 with:

```rust
fn run_list(project: bool) -> anyhow::Result<()> {
    if project {
        let manifest_path = std::env::current_dir()?.join("ion.toml");
        let manifest = Manifest::from_file(&manifest_path)?;
        let values = manifest.options.list_values();
        if values.is_empty() {
            println!("No project config values set.");
        } else {
            print_config_sections(&values);
        }
    } else {
        let config = GlobalConfig::load()?;
        let values = config.list_values();
        if values.is_empty() {
            println!("No global config values set.");
        } else {
            print_config_sections(&values);
        }
    }
    Ok(())
}
```

**Step 3: Run tests and verify**

Run: `cargo test`
Expected: All tests pass.

Run: `cargo clippy`
Expected: No warnings.

**Step 4: Commit**

```bash
git add src/commands/config.rs
git commit -m "refactor: deduplicate config list section printing"
```

---

### Task 4: TUI terminal lifecycle helper

**Files:**
- Create: `src/tui/terminal.rs`
- Modify: `src/tui/mod.rs`
- Modify: `src/commands/config.rs:159-215`
- Modify: `src/commands/search.rs:244-312`

**Step 1: Create `src/tui/terminal.rs`**

```rust
use std::io;

use crossterm::event::{self, Event};
use crossterm::execute;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;

pub type Term = Terminal<CrosstermBackend<io::Stdout>>;

/// Run a TUI application with proper terminal setup and cleanup.
/// The `body` closure receives a mutable reference to the terminal and runs the
/// main event loop. Terminal is always restored, even on error.
pub fn run_tui<F, T>(body: F) -> anyhow::Result<T>
where
    F: FnOnce(&mut Term) -> anyhow::Result<T>,
{
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let result = body(&mut terminal);

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    result
}
```

**Step 2: Register module in `src/tui/mod.rs`**

Add `pub mod terminal;` to `src/tui/mod.rs`.

**Step 3: Rewrite `run_interactive` in `src/commands/config.rs`**

Replace lines 159-215 with:

```rust
fn run_interactive() -> anyhow::Result<()> {
    use crossterm::event::{self, Event};

    use crate::tui::app::App;
    use crate::tui::event::handle_key;
    use crate::tui::terminal::run_tui;
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

    run_tui(|terminal| {
        loop {
            terminal.draw(|frame| render(frame, &app))?;

            if let Event::Key(key) = event::read()?
                && let Err(e) = handle_key(&mut app, key)
            {
                app.status_message = Some(format!("Error: {e}"));
            }

            if app.should_quit {
                return Ok(());
            }
        }
    })
}
```

**Step 4: Rewrite `pick_and_install` in `src/commands/search.rs`**

Replace lines 244-312 with:

```rust
fn pick_and_install(results: &[SearchResult]) -> anyhow::Result<()> {
    use crossterm::event::{self, Event};

    use crate::tui::search_app::SearchApp;
    use crate::tui::search_event::handle_search_key;
    use crate::tui::search_ui::render_search;
    use crate::tui::terminal::run_tui;

    let installable: Vec<SearchResult> = results
        .iter()
        .filter(|r| !r.source.is_empty())
        .cloned()
        .collect();
    if installable.is_empty() {
        println!("No installable results to select from.");
        return Ok(());
    }

    let mut app = SearchApp::new(installable);

    run_tui(|terminal| {
        loop {
            terminal.draw(|frame| render_search(frame, &mut app))?;

            if let Event::Key(key) = event::read()? {
                handle_search_key(&mut app, key)?;
            }

            if app.should_quit || app.should_install {
                break;
            }
        }
        Ok(())
    })?;

    // Handle install (after terminal is restored)
    if app.should_install {
        if let Some(chosen) = app.selected_result() {
            log::debug!("user selected: {} (source={})", chosen.name, chosen.source);
            println!("\nInstalling '{}'...", chosen.name);
            let status = std::process::Command::new("ion")
                .arg("add")
                .arg(&chosen.source)
                .status()?;
            if !status.success() {
                anyhow::bail!("ion add failed");
            }
        }
    }

    Ok(())
}
```

**Step 5: Run tests and verify**

Run: `cargo test`
Expected: All tests pass.

Run: `cargo clippy`
Expected: No warnings.

**Step 6: Commit**

```bash
git add src/tui/terminal.rs src/tui/mod.rs src/commands/config.rs src/commands/search.rs
git commit -m "refactor: extract TUI terminal lifecycle helper to deduplicate setup/teardown"
```

---

### Task 5: ProjectContext abstraction

**Files:**
- Create: `src/context.rs`
- Modify: `src/main.rs:3` (add `mod context;`)
- Modify: `src/commands/add.rs` (full rewrite)
- Modify: `src/commands/remove.rs` (full rewrite)
- Modify: `src/commands/install.rs` (full rewrite)
- Modify: `src/commands/list.rs` (full rewrite)
- Modify: `src/commands/info.rs:5-7` (use ProjectContext)
- Modify: `src/commands/config.rs:42-44,66-68,80-83` (use ProjectContext for project paths)
- Modify: `src/commands/migrate.rs:11` (use ProjectContext)

**Step 1: Create `src/context.rs`**

```rust
use std::path::PathBuf;

use ion_skill::config::GlobalConfig;
use ion_skill::lockfile::Lockfile;
use ion_skill::manifest::{Manifest, ManifestOptions};

/// Shared project context used across commands.
/// Loads global config eagerly; manifest and lockfile are loaded on demand.
pub struct ProjectContext {
    pub project_dir: PathBuf,
    pub manifest_path: PathBuf,
    pub lockfile_path: PathBuf,
    pub global_config: GlobalConfig,
}

impl ProjectContext {
    /// Load project context from the current directory.
    pub fn load() -> anyhow::Result<Self> {
        let project_dir = std::env::current_dir()?;
        let manifest_path = project_dir.join("ion.toml");
        let lockfile_path = project_dir.join("ion.lock");
        let global_config = GlobalConfig::load()?;

        Ok(Self {
            project_dir,
            manifest_path,
            lockfile_path,
            global_config,
        })
    }

    /// Load manifest from ion.toml. Fails if file doesn't exist.
    pub fn manifest(&self) -> anyhow::Result<Manifest> {
        Manifest::from_file(&self.manifest_path).map_err(Into::into)
    }

    /// Load manifest, or return an empty one if ion.toml doesn't exist.
    pub fn manifest_or_empty(&self) -> anyhow::Result<Manifest> {
        if self.manifest_path.exists() {
            self.manifest()
        } else {
            Ok(Manifest::empty())
        }
    }

    /// Load lockfile from ion.lock. Returns empty if file doesn't exist.
    pub fn lockfile(&self) -> anyhow::Result<Lockfile> {
        Lockfile::from_file(&self.lockfile_path).map_err(Into::into)
    }

    /// Merge global and project targets into a single ManifestOptions.
    pub fn merged_options(&self, manifest: &Manifest) -> ManifestOptions {
        let merged_targets = self.global_config.resolve_targets(&manifest.options);
        ManifestOptions { targets: merged_targets }
    }

    /// Check that ion.toml exists, returning an error if not.
    pub fn require_manifest(&self) -> anyhow::Result<()> {
        if !self.manifest_path.exists() {
            anyhow::bail!("No ion.toml found in current directory");
        }
        Ok(())
    }
}
```

**Step 2: Register module in `src/main.rs`**

Add `mod context;` after `mod commands;` (line 3).

**Step 3: Rewrite `src/commands/add.rs`**

```rust
use ion_skill::installer::install_skill;
use ion_skill::manifest_writer;
use ion_skill::source::SkillSource;

use crate::context::ProjectContext;

pub fn run(source_str: &str, rev: Option<&str>) -> anyhow::Result<()> {
    let ctx = ProjectContext::load()?;

    let expanded = ctx.global_config.resolve_source(source_str);
    let mut source = SkillSource::infer(&expanded)?;
    if let Some(r) = rev {
        source.rev = Some(r.to_string());
    }

    let name = skill_name_from_source(&source);
    println!("Adding skill '{name}' from {source_str}...");

    let manifest = ctx.manifest_or_empty()?;
    let merged_options = ctx.merged_options(&manifest);

    let locked = install_skill(&ctx.project_dir, &name, &source, &merged_options)?;
    println!("  Installed to .agents/skills/{name}/");
    for target_name in merged_options.targets.keys() {
        println!("  Linked to {target_name}");
    }

    manifest_writer::add_skill(&ctx.manifest_path, &name, &source)?;
    println!("  Updated ion.toml");

    let mut lockfile = ctx.lockfile()?;
    lockfile.upsert(locked);
    lockfile.write_to(&ctx.lockfile_path)?;
    println!("  Updated ion.lock");

    println!("Done!");
    Ok(())
}

fn skill_name_from_source(source: &SkillSource) -> String {
    if let Some(ref path) = source.path {
        path.rsplit('/').next().unwrap_or(path).to_string()
    } else {
        source
            .source
            .trim_end_matches(".git")
            .rsplit('/')
            .next()
            .unwrap_or(&source.source)
            .to_string()
    }
}
```

**Step 4: Rewrite `src/commands/remove.rs`**

```rust
use ion_skill::installer::uninstall_skill;
use ion_skill::manifest_writer;

use crate::context::ProjectContext;

pub fn run(name: &str) -> anyhow::Result<()> {
    let ctx = ProjectContext::load()?;
    let manifest = ctx.manifest()?;

    if !manifest.skills.contains_key(name) {
        anyhow::bail!("Skill '{name}' not found in ion.toml");
    }

    let merged_options = ctx.merged_options(&manifest);

    println!("Removing skill '{name}'...");

    uninstall_skill(&ctx.project_dir, name, &merged_options)?;
    println!("  Removed from .agents/skills/{name}/");

    manifest_writer::remove_skill(&ctx.manifest_path, name)?;
    println!("  Updated ion.toml");

    let mut lockfile = ctx.lockfile()?;
    lockfile.remove(name);
    lockfile.write_to(&ctx.lockfile_path)?;
    println!("  Updated ion.lock");

    println!("Done!");
    Ok(())
}
```

**Step 5: Rewrite `src/commands/install.rs`**

```rust
use ion_skill::installer::install_skill;
use ion_skill::manifest::Manifest;

use crate::context::ProjectContext;

pub fn run() -> anyhow::Result<()> {
    let ctx = ProjectContext::load()?;
    ctx.require_manifest()?;

    let manifest = ctx.manifest()?;
    let mut lockfile = ctx.lockfile()?;

    if manifest.skills.is_empty() {
        println!("No skills declared in ion.toml.");
        return Ok(());
    }

    let merged_options = ctx.merged_options(&manifest);

    println!("Installing {} skill(s)...", manifest.skills.len());

    for (name, entry) in &manifest.skills {
        let source = Manifest::resolve_entry(entry)?;
        println!("  Installing '{name}'...");
        let locked = install_skill(&ctx.project_dir, name, &source, &merged_options)?;
        lockfile.upsert(locked);
    }

    lockfile.write_to(&ctx.lockfile_path)?;
    println!("Updated ion.lock");
    println!("Done!");

    // Check gitignore for managed directories
    let mut managed_dirs = vec![".agents/".to_string()];
    for path in merged_options.targets.values() {
        let top_level = path.split('/').next().unwrap_or(path);
        let entry = format!("{top_level}/");
        if !managed_dirs.contains(&entry) {
            managed_dirs.push(entry);
        }
    }

    let dir_refs: Vec<&str> = managed_dirs.iter().map(|s| s.as_str()).collect();
    let missing = ion_skill::gitignore::find_missing_gitignore_entries(&ctx.project_dir, &dir_refs)?;

    if !missing.is_empty() {
        println!("\nThese directories are not in .gitignore:");
        for dir in &missing {
            println!("  {dir}");
        }
        print!("\nAdd them? [Y/n] (press Enter for yes) ");
        std::io::Write::flush(&mut std::io::stdout())?;

        let mut answer = String::new();
        std::io::stdin().read_line(&mut answer)?;

        if answer.trim().is_empty() || answer.trim().eq_ignore_ascii_case("y") {
            let refs: Vec<&str> = missing.iter().map(|s| s.as_str()).collect();
            ion_skill::gitignore::append_to_gitignore(&ctx.project_dir, &refs)?;
            println!("Updated .gitignore");
        }
    }

    Ok(())
}
```

**Step 6: Rewrite `src/commands/list.rs`**

```rust
use ion_skill::manifest::Manifest;

use crate::context::ProjectContext;

pub fn run() -> anyhow::Result<()> {
    let ctx = ProjectContext::load()?;
    ctx.require_manifest()?;

    let manifest = ctx.manifest()?;
    let lockfile = ctx.lockfile()?;

    if manifest.skills.is_empty() {
        println!("No skills declared in ion.toml.");
        return Ok(());
    }

    println!("Skills ({}):", manifest.skills.len());
    for (name, entry) in &manifest.skills {
        let source = Manifest::resolve_entry(entry)?;
        let locked = lockfile.find(name);

        let version_str = locked
            .and_then(|l| l.version.as_deref())
            .unwrap_or("unknown");
        let commit_str = locked
            .and_then(|l| l.commit.as_deref())
            .map(|c| &c[..c.len().min(8)])
            .unwrap_or("none");
        let installed = ctx.project_dir
            .join(".agents")
            .join("skills")
            .join(name)
            .exists();
        let status = if installed {
            "installed"
        } else {
            "not installed"
        };

        println!("  {name} v{version_str} ({commit_str}) [{status}]");
        println!("    source: {}", source.source);
    }
    Ok(())
}
```

**Step 7: Update `src/commands/info.rs`**

```rust
use ion_skill::skill::SkillMetadata;
use ion_skill::source::SkillSource;

use crate::context::ProjectContext;

pub fn run(skill_str: &str) -> anyhow::Result<()> {
    let ctx = ProjectContext::load()?;

    // First check if it's a name in the manifest
    if ctx.manifest_path.exists() {
        let manifest = ctx.manifest()?;
        if manifest.skills.contains_key(skill_str) {
            return show_info_from_installed(&ctx.project_dir, skill_str);
        }
    }

    // Otherwise try to resolve as a source
    let source = SkillSource::infer(skill_str)?;
    println!("Fetching info for '{skill_str}'...");
    println!("  Source type: {:?}", source.source_type);
    println!("  Source: {}", source.source);
    if let Some(ref path) = source.path {
        println!("  Path: {path}");
    }
    if let Ok(url) = source.git_url() {
        println!("  Git URL: {url}");
    }
    Ok(())
}

fn show_info_from_installed(project_dir: &std::path::Path, name: &str) -> anyhow::Result<()> {
    let skill_md = project_dir
        .join(".agents")
        .join("skills")
        .join(name)
        .join("SKILL.md");

    if !skill_md.exists() {
        anyhow::bail!("Skill '{name}' is in ion.toml but not installed. Run `ion install`.");
    }

    let (meta, _body) = SkillMetadata::from_file(&skill_md)?;

    println!("Skill: {}", meta.name);
    println!("Description: {}", meta.description);
    if let Some(ref license) = meta.license {
        println!("License: {license}");
    }
    if let Some(ref compat) = meta.compatibility {
        println!("Compatibility: {compat}");
    }
    if let Some(version) = meta.version() {
        println!("Version: {version}");
    }
    if let Some(ref metadata) = meta.metadata {
        for (k, v) in metadata {
            if k != "version" {
                println!("  {k}: {v}");
            }
        }
    }
    Ok(())
}
```

**Step 8: Update project config paths in `src/commands/config.rs`**

In `run_get`, `run_set`, and `run_list`, replace `std::env::current_dir()?.join("ion.toml")` with `ProjectContext::load()` where it makes the code cleaner. However, since `config.rs` uses `GlobalConfig::load()` independently and has its own TUI flow, only update the project-path lines:

In the `if project` branches of `run_get` (line 44), `run_set` (line 68), and `run_list` (line 82), change:
```rust
let manifest_path = std::env::current_dir()?.join("ion.toml");
```
to:
```rust
let manifest_path = crate::context::ProjectContext::load()?.manifest_path;
```

Also update `run_interactive` (line 177-183) to:
```rust
    let ctx = crate::context::ProjectContext::load()?;
    let manifest_opt = if ctx.manifest_path.exists() {
        Some(ctx.manifest_path)
    } else {
        None
    };
```

(This removes the separate `project_dir` and `manifest_path` computations.)

**Step 9: Update `src/commands/migrate.rs` to use ProjectContext**

Replace line 11 (`let project_dir = std::env::current_dir()?;`) with:
```rust
    let ctx = crate::context::ProjectContext::load()?;
    let project_dir = &ctx.project_dir;
```

And update the `lockfile_path` default to:
```rust
    let lockfile_path = from
        .map(PathBuf::from)
        .unwrap_or_else(|| ctx.project_dir.join("skills-lock.json"));
```

**Step 10: Run tests and verify**

Run: `cargo test`
Expected: All tests pass.

Run: `cargo clippy`
Expected: No warnings.

**Step 11: Commit**

```bash
git add src/context.rs src/main.rs src/commands/add.rs src/commands/remove.rs src/commands/install.rs src/commands/list.rs src/commands/info.rs src/commands/config.rs src/commands/migrate.rs
git commit -m "refactor: introduce ProjectContext to eliminate path boilerplate across commands"
```

---

### Task 6: SkillInstaller abstraction

**Files:**
- Modify: `crates/ion-skill/src/installer.rs` (major rewrite — restructure into `SkillInstaller` struct)
- Modify: `crates/ion-skill/src/lib.rs` (update re-exports if needed)
- Modify: `src/commands/add.rs` (use `SkillInstaller`)
- Modify: `src/commands/remove.rs` (use `SkillInstaller`)
- Modify: `src/commands/install.rs` (use `SkillInstaller`)
- Modify: `crates/ion-skill/src/migrate.rs` (use `SkillInstaller`)

**Step 1: Rewrite `crates/ion-skill/src/installer.rs` with `SkillInstaller` struct**

Keep all private helper functions (`cache_dir`, `fetch_skill`, `copy_skill_dir`, `copy_dir_recursive`, `create_skill_symlink`, `find_repo_root`, `hash_simple`) as module-level private functions. Wrap the public API in `SkillInstaller`:

```rust
use std::path::{Path, PathBuf};

use crate::lockfile::LockedSkill;
use crate::manifest::ManifestOptions;
use crate::skill::SkillMetadata;
use crate::source::{SkillSource, SourceType};
use crate::{Error, Result, git};

/// Where ion caches cloned repositories.
fn cache_dir() -> PathBuf {
    dirs::cache_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("ion")
        .join("repos")
}

/// Manages skill installation and uninstallation for a project.
pub struct SkillInstaller<'a> {
    project_dir: &'a Path,
    options: &'a ManifestOptions,
}

impl<'a> SkillInstaller<'a> {
    pub fn new(project_dir: &'a Path, options: &'a ManifestOptions) -> Self {
        Self { project_dir, options }
    }

    /// Install a single skill from a resolved source. Returns the locked entry.
    pub fn install(&self, name: &str, source: &SkillSource) -> Result<LockedSkill> {
        let skill_dir = self.fetch(source)?;
        let meta = self.validate(&skill_dir, source)?;
        self.deploy(name, &skill_dir)?;
        self.build_locked_entry(name, source, &meta, &skill_dir)
    }

    /// Remove an installed skill from the project directory.
    pub fn uninstall(&self, name: &str) -> Result<()> {
        let agents_dir = self.project_dir.join(".agents").join("skills").join(name);
        if agents_dir.exists() {
            std::fs::remove_dir_all(&agents_dir).map_err(Error::Io)?;
        }

        for target_path in self.options.targets.values() {
            let target_dir = self.project_dir.join(target_path).join(name);
            if target_dir.is_symlink() {
                std::fs::remove_file(&target_dir).map_err(Error::Io)?;
            } else if target_dir.exists() {
                std::fs::remove_dir_all(&target_dir).map_err(Error::Io)?;
            }
        }

        Ok(())
    }

    fn fetch(&self, source: &SkillSource) -> Result<PathBuf> {
        fetch_skill(source)
    }

    fn validate(&self, skill_dir: &Path, source: &SkillSource) -> Result<SkillMetadata> {
        let skill_md = skill_dir.join("SKILL.md");
        if !skill_md.exists() {
            return Err(Error::InvalidSkill(format!(
                "No SKILL.md found at {}",
                skill_md.display()
            )));
        }

        let (meta, _body) = SkillMetadata::from_file(&skill_md)?;

        if let Some(ref required_version) = source.version {
            let actual_version = meta.version().unwrap_or("(none)");
            if actual_version != required_version {
                return Err(Error::InvalidSkill(format!(
                    "Version mismatch: expected {required_version}, found {actual_version}"
                )));
            }
        }

        Ok(meta)
    }

    fn deploy(&self, name: &str, skill_dir: &Path) -> Result<()> {
        let agents_target = self.project_dir.join(".agents").join("skills").join(name);
        copy_skill_dir(skill_dir, &agents_target)?;

        let canonical = self.project_dir.join(".agents").join("skills").join(name);
        for target_path in self.options.targets.values() {
            let target_skill_dir = self.project_dir.join(target_path).join(name);
            create_skill_symlink(&canonical, &target_skill_dir)?;
        }

        Ok(())
    }

    fn build_locked_entry(
        &self,
        name: &str,
        source: &SkillSource,
        meta: &SkillMetadata,
        skill_dir: &Path,
    ) -> Result<LockedSkill> {
        let (commit, checksum) = match source.source_type {
            SourceType::Github | SourceType::Git => {
                let repo_dir = find_repo_root(skill_dir);
                let commit = git::head_commit(&repo_dir).ok();
                let checksum = git::checksum_dir(skill_dir).ok();
                (commit, checksum)
            }
            SourceType::Path | SourceType::Http => {
                let checksum = git::checksum_dir(skill_dir).ok();
                (None, checksum)
            }
        };

        let git_url = source.git_url().ok().unwrap_or_else(|| source.source.clone());

        Ok(LockedSkill {
            name: name.to_string(),
            source: git_url,
            path: source.path.clone(),
            version: meta.version().map(|s| s.to_string()),
            commit,
            checksum,
        })
    }
}

// Keep standalone functions as thin wrappers for backward compatibility during transition,
// delegating to SkillInstaller. These can be removed once all callers are migrated.

/// Install a single skill from a resolved source into a project directory.
pub fn install_skill(
    project_dir: &Path,
    name: &str,
    source: &SkillSource,
    options: &ManifestOptions,
) -> Result<LockedSkill> {
    SkillInstaller::new(project_dir, options).install(name, source)
}

/// Remove an installed skill from the project directory.
pub fn uninstall_skill(project_dir: &Path, name: &str, options: &ManifestOptions) -> Result<()> {
    SkillInstaller::new(project_dir, options).uninstall(name)
}

/// Fetch a skill source to a local directory. Returns the path to the skill directory.
fn fetch_skill(source: &SkillSource) -> Result<PathBuf> {
    match source.source_type {
        SourceType::Github | SourceType::Git => {
            let url = source.git_url()?;
            let repo_hash = format!("{:x}", hash_simple(&url));
            let repo_dir = cache_dir().join(&repo_hash);

            git::clone_or_fetch(&url, &repo_dir)?;

            if let Some(ref rev) = source.rev {
                git::checkout(&repo_dir, rev)?;
            }

            match &source.path {
                Some(path) => {
                    let skill_dir = repo_dir.join(path);
                    if skill_dir.exists() {
                        return Ok(skill_dir);
                    }
                    let fallback_dir = repo_dir.join("skills").join(path);
                    if fallback_dir.exists() {
                        return Ok(fallback_dir);
                    }
                    Err(Error::Source(format!(
                        "Skill path '{path}' not found in repository (also tried 'skills/{path}')"
                    )))
                }
                None => Ok(repo_dir),
            }
        }
        SourceType::Path => {
            let path = PathBuf::from(&source.source);
            if !path.exists() {
                return Err(Error::Source(format!(
                    "Local path does not exist: {}", source.source
                )));
            }
            Ok(path)
        }
        SourceType::Http => {
            Err(Error::Source("HTTP source not yet implemented".to_string()))
        }
    }
}

fn copy_skill_dir(src: &Path, dst: &Path) -> Result<()> {
    if dst.exists() {
        std::fs::remove_dir_all(dst).map_err(Error::Io)?;
    }
    if let Some(parent) = dst.parent() {
        std::fs::create_dir_all(parent).map_err(Error::Io)?;
    }
    copy_dir_recursive(src, dst)
}

fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<()> {
    std::fs::create_dir_all(dst).map_err(Error::Io)?;
    for entry in std::fs::read_dir(src).map_err(Error::Io)? {
        let entry = entry.map_err(Error::Io)?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());
        if src_path.file_name().is_some_and(|n| n == ".git") {
            continue;
        }
        if src_path.is_dir() {
            copy_dir_recursive(&src_path, &dst_path)?;
        } else {
            std::fs::copy(&src_path, &dst_path).map_err(Error::Io)?;
        }
    }
    Ok(())
}

fn create_skill_symlink(original: &Path, link: &Path) -> Result<()> {
    if link.is_symlink() {
        std::fs::remove_file(link).map_err(Error::Io)?;
    } else if link.exists() {
        std::fs::remove_dir_all(link).map_err(Error::Io)?;
    }

    if let Some(parent) = link.parent() {
        std::fs::create_dir_all(parent).map_err(Error::Io)?;
    }

    let link_parent = link.parent().unwrap();
    let relative = pathdiff::diff_paths(original, link_parent)
        .ok_or_else(|| Error::Io(std::io::Error::other(
            format!("Cannot compute relative path from {} to {}", link_parent.display(), original.display()),
        )))?;

    #[cfg(unix)]
    std::os::unix::fs::symlink(&relative, link).map_err(Error::Io)?;

    #[cfg(windows)]
    std::os::windows::fs::symlink_dir(&relative, link).map_err(Error::Io)?;

    Ok(())
}

fn find_repo_root(path: &Path) -> PathBuf {
    let mut current = path.to_path_buf();
    loop {
        if current.join(".git").exists() {
            return current;
        }
        if !current.pop() {
            return path.to_path_buf();
        }
    }
}

fn hash_simple(s: &str) -> u64 {
    use std::hash::{DefaultHasher, Hash, Hasher};
    let mut hasher = DefaultHasher::new();
    s.hash(&mut hasher);
    hasher.finish()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn copy_skill_dir_works() {
        let src = tempfile::tempdir().unwrap();
        std::fs::write(src.path().join("SKILL.md"), "---\nname: test\ndescription: Test.\n---\nBody").unwrap();
        std::fs::create_dir(src.path().join("scripts")).unwrap();
        std::fs::write(src.path().join("scripts").join("run.sh"), "#!/bin/bash").unwrap();

        let dst_dir = tempfile::tempdir().unwrap();
        let dst = dst_dir.path().join("test-skill");
        copy_skill_dir(src.path(), &dst).unwrap();

        assert!(dst.join("SKILL.md").exists());
        assert!(dst.join("scripts").join("run.sh").exists());
    }

    #[test]
    fn copy_skill_dir_skips_git() {
        let src = tempfile::tempdir().unwrap();
        std::fs::write(src.path().join("SKILL.md"), "content").unwrap();
        std::fs::create_dir(src.path().join(".git")).unwrap();
        std::fs::write(src.path().join(".git").join("HEAD"), "ref").unwrap();

        let dst_dir = tempfile::tempdir().unwrap();
        let dst = dst_dir.path().join("out");
        copy_skill_dir(src.path(), &dst).unwrap();

        assert!(dst.join("SKILL.md").exists());
        assert!(!dst.join(".git").exists());
    }

    #[test]
    fn uninstall_removes_dirs() {
        let project = tempfile::tempdir().unwrap();
        let agents = project.path().join(".agents").join("skills").join("test");
        std::fs::create_dir_all(&agents).unwrap();
        std::fs::write(agents.join("SKILL.md"), "x").unwrap();

        let claude = project.path().join(".claude").join("skills");
        std::fs::create_dir_all(&claude).unwrap();
        #[cfg(unix)]
        std::os::unix::fs::symlink(
            std::path::Path::new("../../../.agents/skills/test"),
            claude.join("test"),
        ).unwrap();

        let mut targets = std::collections::BTreeMap::new();
        targets.insert("claude".to_string(), ".claude/skills".to_string());
        let options = ManifestOptions { targets };

        SkillInstaller::new(project.path(), &options).uninstall("test").unwrap();

        assert!(!agents.exists());
        assert!(!claude.join("test").exists());
    }

    #[test]
    fn install_creates_symlinks_for_targets() {
        let skill_src = tempfile::tempdir().unwrap();
        std::fs::write(
            skill_src.path().join("SKILL.md"),
            "---\nname: sym-test\ndescription: Symlink test.\n---\n\nBody.\n",
        ).unwrap();

        let project = tempfile::tempdir().unwrap();
        let source = SkillSource {
            source_type: SourceType::Path,
            source: skill_src.path().display().to_string(),
            path: None,
            rev: None,
            version: None,
        };

        let mut targets = std::collections::BTreeMap::new();
        targets.insert("claude".to_string(), ".claude/skills".to_string());
        let options = ManifestOptions { targets };

        let installer = SkillInstaller::new(project.path(), &options);
        let _locked = installer.install("sym-test", &source).unwrap();

        let canonical = project.path().join(".agents/skills/sym-test");
        assert!(canonical.exists());
        assert!(canonical.is_dir());
        assert!(!canonical.is_symlink());

        let target = project.path().join(".claude/skills/sym-test");
        assert!(target.exists());
        assert!(target.is_symlink());
        assert!(target.join("SKILL.md").exists());
    }

    #[test]
    fn install_local_skill() {
        let skill_src = tempfile::tempdir().unwrap();
        std::fs::write(
            skill_src.path().join("SKILL.md"),
            "---\nname: local-test\ndescription: A local test skill.\n---\n\nInstructions here.\n",
        ).unwrap();

        let project = tempfile::tempdir().unwrap();
        let source = SkillSource {
            source_type: SourceType::Path,
            source: skill_src.path().display().to_string(),
            path: None,
            rev: None,
            version: None,
        };
        let options = ManifestOptions { targets: std::collections::BTreeMap::new() };

        let installer = SkillInstaller::new(project.path(), &options);
        let locked = installer.install("local-test", &source).unwrap();
        assert_eq!(locked.name, "local-test");
        assert!(project.path().join(".agents/skills/local-test/SKILL.md").exists());
    }

    // Backward-compat wrapper tests
    #[test]
    fn install_skill_wrapper_works() {
        let skill_src = tempfile::tempdir().unwrap();
        std::fs::write(
            skill_src.path().join("SKILL.md"),
            "---\nname: wrapper-test\ndescription: Test.\n---\n\nBody.\n",
        ).unwrap();

        let project = tempfile::tempdir().unwrap();
        let source = SkillSource {
            source_type: SourceType::Path,
            source: skill_src.path().display().to_string(),
            path: None,
            rev: None,
            version: None,
        };
        let options = ManifestOptions { targets: std::collections::BTreeMap::new() };

        let locked = install_skill(project.path(), "wrapper-test", &source, &options).unwrap();
        assert_eq!(locked.name, "wrapper-test");
    }
}
```

**Step 2: Update `src/commands/add.rs` to use `SkillInstaller`**

Replace the `install_skill` import and call:

```rust
use ion_skill::installer::SkillInstaller;
```

And change:
```rust
    let locked = install_skill(&ctx.project_dir, &name, &source, &merged_options)?;
```
to:
```rust
    let installer = SkillInstaller::new(&ctx.project_dir, &merged_options);
    let locked = installer.install(&name, &source)?;
```

**Step 3: Update `src/commands/remove.rs` to use `SkillInstaller`**

Replace:
```rust
use ion_skill::installer::uninstall_skill;
```
with:
```rust
use ion_skill::installer::SkillInstaller;
```

And change:
```rust
    uninstall_skill(&ctx.project_dir, name, &merged_options)?;
```
to:
```rust
    SkillInstaller::new(&ctx.project_dir, &merged_options).uninstall(name)?;
```

**Step 4: Update `src/commands/install.rs` to use `SkillInstaller`**

Replace:
```rust
use ion_skill::installer::install_skill;
```
with:
```rust
use ion_skill::installer::SkillInstaller;
```

And change:
```rust
        let locked = install_skill(&ctx.project_dir, name, &source, &merged_options)?;
```
to:
```rust
        let installer = SkillInstaller::new(&ctx.project_dir, &merged_options);
```
(created before the loop), then inside the loop:
```rust
        let locked = installer.install(name, &source)?;
```

**Step 5: Update `crates/ion-skill/src/migrate.rs` to use `SkillInstaller`**

Replace:
```rust
use crate::installer::install_skill;
```
with:
```rust
use crate::installer::SkillInstaller;
```

And change:
```rust
        let locked = install_skill(project_dir, &skill.name, &source, &options.manifest_options)?;
```
to:
```rust
    let installer = SkillInstaller::new(project_dir, &options.manifest_options);
```
(created before the loop), then inside the loop:
```rust
        let locked = installer.install(&skill.name, &source)?;
```

**Step 6: Run tests and verify**

Run: `cargo test`
Expected: All tests pass (including the updated installer tests using `SkillInstaller`).

Run: `cargo clippy`
Expected: No warnings.

**Step 7: Commit**

```bash
git add crates/ion-skill/src/installer.rs crates/ion-skill/src/migrate.rs src/commands/add.rs src/commands/remove.rs src/commands/install.rs
git commit -m "refactor: introduce SkillInstaller struct to encapsulate install/uninstall logic"
```

---

### Task 7: Remove backward-compat wrappers (cleanup)

After Task 6, the standalone `install_skill()` and `uninstall_skill()` wrapper functions still exist for safety. Once all callers have been migrated:

**Step 1: Check for remaining callers**

Run: `grep -r "install_skill\|uninstall_skill" --include="*.rs" | grep -v "SkillInstaller" | grep -v "tests" | grep -v "mod tests"`

If no callers remain outside tests, remove the wrapper functions from `crates/ion-skill/src/installer.rs`.

**Step 2: Remove the wrappers**

Delete the `pub fn install_skill(...)` and `pub fn uninstall_skill(...)` wrapper functions from `installer.rs`.

**Step 3: Run tests and verify**

Run: `cargo test`
Expected: All tests pass.

Run: `cargo clippy`
Expected: No warnings.

**Step 4: Commit**

```bash
git add crates/ion-skill/src/installer.rs
git commit -m "refactor: remove backward-compat install_skill/uninstall_skill wrappers"
```

---

### Task 8: Final verification

**Step 1: Full test suite**

Run: `cargo test`
Expected: All tests pass.

**Step 2: Clippy**

Run: `cargo clippy -- -D warnings`
Expected: No warnings.

**Step 3: Build release**

Run: `cargo build --release`
Expected: Builds successfully.

**Step 4: Smoke test**

Run: `cargo run -- list` (in a project with ion.toml)
Run: `cargo run -- config list`
Expected: Same output as before the refactoring.
