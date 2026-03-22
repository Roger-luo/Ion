use clap::{CommandFactory, Parser, Subcommand};

mod builtin_skill;
mod commands;
mod context;
mod json;
pub mod style;
mod tui;

#[derive(Parser)]
#[command(name = "ion", about = "Agent skill manager", version = env!("CARGO_PKG_VERSION"))]
struct Cli {
    /// Output results as JSON (for agents and scripts)
    #[arg(long, global = true)]
    json: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize a project or scaffold a binary skill
    Init {
        /// Path for binary skill project (default: current directory)
        path: Option<String>,
        /// Scaffold a binary skill CLI project with ionem
        #[arg(long)]
        bin: bool,
        /// Set up GitHub Actions CI/CD (requires --bin)
        #[arg(long)]
        ci: bool,
        /// Configure specific targets (e.g. claude, cursor, or name:path)
        #[arg(long, short = 't')]
        target: Vec<String>,
        /// Overwrite existing files
        #[arg(long)]
        force: bool,
    },
    /// Set up GitHub Actions CI/CD for a binary skill project
    Ci {
        /// Overwrite existing workflow files
        #[arg(long)]
        force: bool,
    },
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
        /// Dev mode: forward `ion run` to `cargo run` instead of building (local binary only)
        #[arg(long)]
        dev: bool,
        /// Override the skill name (default: inferred from source)
        #[arg(long)]
        name: Option<String>,
        /// Proceed despite validation warnings
        #[arg(long)]
        allow_warnings: bool,
        /// Comma-separated list of skills to install from a collection
        #[arg(long)]
        skills: Option<String>,
    },
    /// Remove a skill from the project
    Remove {
        /// Skill name or source (e.g. brainstorming, obra/superpowers)
        name: String,
        /// Skip confirmation prompt
        #[arg(long, short = 'y')]
        yes: bool,
    },
    /// Search for skills across registries and GitHub
    Search {
        /// Search query (word or phrase)
        query: String,
        /// Include configured CLI agent in search
        #[arg(long)]
        agent: bool,
        /// Search only a specific source
        #[arg(long)]
        source: Option<String>,
        /// Max results per source
        #[arg(long, default_value = "50")]
        limit: usize,
        /// Enable verbose debug logging
        #[arg(long, short)]
        verbose: bool,
    },
    /// Update skills to their latest versions
    Update {
        /// Update only a specific skill (default: update all)
        name: Option<String>,
    },
    /// Run a binary skill
    Run {
        /// Name of the binary skill to run
        name: String,
        /// Arguments to pass to the binary
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    /// Create, inspect, and validate skills
    Skill {
        #[command(subcommand)]
        action: SkillCommands,
    },
    /// Migrate skills from skills-lock.json or existing directories
    Migrate {
        /// Path to skills-lock.json (defaults to ./skills-lock.json)
        #[arg(long)]
        from: Option<String>,
        /// Show what would be migrated without writing files
        #[arg(long)]
        dry_run: bool,
        /// Skip confirmation prompts (auto-accept all)
        #[arg(long, short = 'y')]
        yes: bool,
    },
    /// Manage the skill cache
    Cache {
        #[command(subcommand)]
        action: CacheCommands,
    },
    /// Manage ion configuration
    Config {
        #[command(subcommand)]
        action: Option<commands::config::ConfigAction>,
    },
    /// Manage the ion installation
    #[command(name = "self")]
    Self_ {
        #[command(subcommand)]
        action: SelfCommands,
    },
    /// Generate shell completion scripts
    #[command(after_help = "\
Setup:
  bash   ion completion bash >> ~/.bashrc
  zsh    ion completion zsh > ~/.zfunc/_ion
  fish   ion completion fish > ~/.config/fish/completions/ion.fish
  elvish ion completion elvish >> ~/.config/elvish/rc.elv
  pwsh   ion completion powershell >> $PROFILE")]
    Completion {
        /// Shell to generate completions for
        shell: clap_complete::Shell,
    },
}

#[derive(Subcommand)]
enum SkillCommands {
    /// Create a new skill or skill collection
    New {
        /// Target directory (default: current directory)
        #[arg(long)]
        path: Option<String>,
        /// Set the project skills directory (persisted to Ion.toml)
        #[arg(long)]
        dir: Option<String>,
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
    /// Eject a remote skill into an editable local copy
    Eject {
        /// Name of the skill to eject
        name: String,
    },
}

#[derive(Subcommand)]
enum SelfCommands {
    /// Output the SKILL.md for this tool
    Skill,
    /// Show ion version and installation info
    Info,
    /// Check if a newer version of ion is available
    Check,
    /// Update ion to the latest (or a specific) version
    Update {
        /// Install a specific version (e.g. 0.2.0)
        #[arg(long)]
        version: Option<String>,
    },
    /// Uninstall ion and remove all data
    Uninstall {
        /// Skip confirmation prompt
        #[arg(long, short = 'y')]
        yes: bool,
    },
}

#[derive(Subcommand)]
enum CacheCommands {
    /// Garbage collect stale skill repos from global storage
    Gc {
        /// Show what would be cleaned without deleting
        #[arg(long)]
        dry_run: bool,
    },
}

fn main() {
    // Refresh the global ion-cli SKILL.md if the binary has been updated.
    // Cheap no-op if already current; ensures `ion self update` propagates
    // the new skill content to all symlinked projects on the next command.
    builtin_skill::refresh_global();

    let cli = Cli::parse();
    let json = cli.json;
    let skip_update_check = matches!(
        cli.command,
        Commands::Self_ { .. } | Commands::Completion { .. }
    );

    let result = match cli.command {
        Commands::Init {
            path,
            bin,
            ci,
            target,
            force,
        } => {
            if bin {
                commands::new::run_bin(path.as_deref(), ci, force, json)
            } else if ci {
                Err(anyhow::anyhow!(
                    "--ci requires --bin (CI/CD setup is for binary skill projects)"
                ))
            } else {
                commands::init::run(&target, force, json)
            }
        }
        Commands::Ci { force } => commands::ci::run(force, json),
        Commands::Add {
            source,
            rev,
            bin,
            dev,
            name,
            allow_warnings,
            skills,
        } => match source {
            Some(src) => commands::add::run(
                &src,
                rev.as_deref(),
                bin,
                dev,
                name.as_deref(),
                json,
                allow_warnings,
                skills.as_deref(),
            ),
            None => commands::install::run(json, allow_warnings),
        },
        Commands::Remove { name, yes } => commands::remove::run(&name, yes, json),
        Commands::Search {
            query,
            agent,
            source,
            limit,
            verbose,
        } => {
            if verbose {
                env_logger::Builder::new()
                    .filter_level(log::LevelFilter::Debug)
                    .init();
            }
            commands::search::run(&query, agent, json, source.as_deref(), limit)
        }
        Commands::Update { name } => commands::update::run(name.as_deref(), json),
        Commands::Run { name, args } => commands::run::run(&name, &args, json),
        Commands::Skill { action } => match action {
            SkillCommands::New {
                path,
                dir,
                collection,
                force,
            } => commands::new::run(path.as_deref(), dir.as_deref(), collection, force, json),
            SkillCommands::Validate { path } => commands::validate::run(path.as_deref(), json),
            SkillCommands::Info { skill } => commands::info::run(&skill, json),
            SkillCommands::List => commands::list::run(json),
            SkillCommands::Link { path } => commands::link::run(&path, json),
            SkillCommands::Eject { name } => commands::eject::run(&name, json),
        },
        Commands::Migrate { from, dry_run, yes } => {
            commands::migrate::run(from.as_deref(), dry_run, json, yes)
        }
        Commands::Cache { action } => match action {
            CacheCommands::Gc { dry_run } => commands::gc::run(dry_run, json),
        },
        Commands::Config { action } => commands::config::run(action, json),
        Commands::Self_ { action } => match action {
            SelfCommands::Skill => {
                commands::self_cmd::skill();
                Ok(())
            }
            SelfCommands::Info => commands::self_cmd::info(json),
            SelfCommands::Check => commands::self_cmd::check(json),
            SelfCommands::Update { version } => {
                commands::self_cmd::update(version.as_deref(), json)
            }
            SelfCommands::Uninstall { yes } => commands::self_cmd::uninstall(yes, json),
        },
        Commands::Completion { shell } => {
            commands::completion::run(shell, Cli::command());
            Ok(())
        }
    };

    if let Err(e) = result {
        if json {
            crate::json::print_error(&e.to_string());
        } else {
            eprintln!("Error: {e}");
            std::process::exit(1);
        }
    }

    // Hint about available updates (silent on failure, skipped for --json and `self` commands)
    if !json && !skip_update_check {
        commands::self_cmd::check_for_update_hint();
    }
}
