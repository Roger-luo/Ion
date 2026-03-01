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

        if let Event::Key(key) = event::read()?
            && let Err(e) = handle_key(&mut app, key)
        {
            app.status_message = Some(format!("Error: {e}"));
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
