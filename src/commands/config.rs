use clap::Subcommand;
use ion_skill::config::GlobalConfig;
use ion_skill::manifest::Manifest;

#[derive(Subcommand)]
pub enum ConfigAction {
    /// Get a config value
    Get {
        /// Key in dot notation (e.g., targets.claude)
        key: String,
        /// Read from project config (Ion.toml) instead of global
        #[arg(long)]
        project: bool,
    },
    /// Set a config value
    Set {
        /// Key in dot notation (e.g., targets.claude)
        key: String,
        /// Value to set
        value: String,
        /// Write to project config (Ion.toml) instead of global
        #[arg(long)]
        project: bool,
    },
    /// List all config values
    List {
        /// Show project config (Ion.toml) instead of global
        #[arg(long)]
        project: bool,
    },
}

pub fn run(action: Option<ConfigAction>, json: bool) -> anyhow::Result<()> {
    match action {
        None if json => anyhow::bail!(
            "Interactive config editor not available in --json mode. Use 'ion config get/set/list'."
        ),
        None => run_interactive(),
        Some(ConfigAction::Get { key, project }) => run_get(&key, project, json),
        Some(ConfigAction::Set {
            key,
            value,
            project,
        }) => run_set(&key, &value, project, json),
        Some(ConfigAction::List { project }) => run_list(project, json),
    }
}

fn run_get(key: &str, project: bool, json: bool) -> anyhow::Result<()> {
    let (value, scope) = if project {
        let ctx = crate::context::ProjectContext::load()?;
        let manifest = Manifest::from_file(&ctx.manifest_path)?;
        (manifest.options.get_value(key), "project")
    } else {
        let config = GlobalConfig::load()?;
        (config.get_value(key), "global")
    };

    let value = match value {
        Some(v) => v,
        None if json => anyhow::bail!("Key '{key}' not found in {scope} config"),
        None => {
            eprintln!("Key '{key}' not found in {scope} config");
            std::process::exit(1);
        }
    };

    if json {
        crate::json::print_success(serde_json::json!({"key": key, "value": value}));
        return Ok(());
    }

    println!("{value}");
    Ok(())
}

fn run_set(key: &str, value: &str, project: bool, json: bool) -> anyhow::Result<()> {
    if project {
        let ctx = crate::context::ProjectContext::load()?;
        set_project_value(&ctx.manifest_path, key, value)?;
    } else {
        let config_path = GlobalConfig::config_path()
            .ok_or_else(|| anyhow::anyhow!("Could not determine global config path"))?;
        GlobalConfig::set_value_in_file(&config_path, key, value)?;
    }

    if json {
        crate::json::print_success(serde_json::json!({"key": key, "value": value}));
    } else {
        let scope = if project { "project" } else { "global" };
        println!("Set {key} = \"{value}\" in {scope} config");
    }
    Ok(())
}

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

fn run_list(project: bool, json: bool) -> anyhow::Result<()> {
    let (values, scope) = if project {
        let ctx = crate::context::ProjectContext::load()?;
        let manifest = Manifest::from_file(&ctx.manifest_path)?;
        (manifest.options.list_values(), "project")
    } else {
        let config = GlobalConfig::load()?;
        (config.list_values(), "global")
    };

    if json {
        let map: serde_json::Map<String, serde_json::Value> = values
            .iter()
            .map(|(k, v)| (k.clone(), serde_json::Value::String(v.clone())))
            .collect();
        crate::json::print_success(map);
        return Ok(());
    }

    if values.is_empty() {
        println!("No {scope} config values set.");
    } else {
        print_config_sections(&values);
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

    let (section, field) = key
        .split_once('.')
        .ok_or_else(|| anyhow::anyhow!("Invalid key format '{key}': expected 'section.key'"))?;

    let content = std::fs::read_to_string(manifest_path)?;
    let mut doc: DocumentMut = content.parse()?;

    if !doc.contains_key("options") {
        doc["options"] = Item::Table(Table::new());
    }
    let options = doc["options"]
        .as_table_mut()
        .ok_or_else(|| anyhow::anyhow!("[options] is not a table"))?;

    match section {
        "targets" => {
            if !options.contains_key(section) {
                options[section] = Item::Table(Table::new());
            }
            options[section][field] = toml_edit::value(value);
        }
        "options" => {
            options[field] = toml_edit::value(value);
        }
        _ => {
            anyhow::bail!(
                "Project config only supports 'targets' and 'options' sections, got '{section}'"
            );
        }
    }

    std::fs::write(manifest_path, doc.to_string())?;
    Ok(())
}

fn run_interactive() -> anyhow::Result<()> {
    use crossterm::event::{self, Event};

    use crate::tui::app::App;
    use crate::tui::event::handle_key;
    use crate::tui::terminal::run_tui;
    use crate::tui::ui::render;

    let global_config_path = GlobalConfig::config_path()
        .ok_or_else(|| anyhow::anyhow!("Could not determine global config path"))?;

    let ctx = crate::context::ProjectContext::load()?;
    let manifest_opt = if ctx.manifest_path.exists() {
        Some(ctx.manifest_path)
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
