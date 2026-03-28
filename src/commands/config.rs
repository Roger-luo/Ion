use clap::Subcommand;
use ion_skill::config::GlobalConfig;
use ion_skill::manifest::Manifest;
use ion_skill::manifest_writer;

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
        None if json => run_list(false, json),
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
        manifest_writer::set_option(&ctx.manifest_path, key, value)?;
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

    // Show hint when configuring a codex target
    if !json
        && let Some(target_name) = key.strip_prefix("targets.")
        && target_name.eq_ignore_ascii_case("codex")
    {
        let config = GlobalConfig::load().unwrap_or_default();
        let p = crate::style::Paint::new(&config);
        println!(
            "  {}: Codex uses the default .agents/ directory — no extra target configuration needed.",
            p.warn("hint")
        );
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

fn run_interactive() -> anyhow::Result<()> {
    use std::io;

    use crossterm::event::{self, Event};
    use crossterm::terminal::{disable_raw_mode, enable_raw_mode};
    use ratatui::backend::CrosstermBackend;
    use ratatui::{Terminal, TerminalOptions, Viewport};

    use crate::tui::app::App;
    use crate::tui::event::handle_key;
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
    let height = app.viewport_height();

    enable_raw_mode()?;
    let backend = CrosstermBackend::new(io::stdout());
    let options = TerminalOptions {
        viewport: Viewport::Inline(height),
    };
    let mut terminal = Terminal::with_options(backend, options)?;
    // Capture viewport start row (after any scrolling done by ratatui)
    let viewport_start_y = crossterm::cursor::position()?.1;

    let result = (|| {
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
    })();

    disable_raw_mode()?;
    // Clear the inline viewport so no button hints or UI artifacts remain
    crossterm::execute!(
        io::stdout(),
        crossterm::cursor::MoveTo(0, viewport_start_y),
        crossterm::terminal::Clear(crossterm::terminal::ClearType::FromCursorDown)
    )?;

    result
}
