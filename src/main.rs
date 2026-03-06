use clap::{Parser, Subcommand};

mod commands;
mod context;
pub mod style;
mod tui;

#[derive(Parser)]
#[command(name = "ion", about = "Agent skill manager")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
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
        /// Pick a result to install interactively
        #[arg(long, short)]
        interactive: bool,
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
    /// Project setup and migration
    Project {
        #[command(subcommand)]
        action: ProjectCommands,
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
    /// Eject a remote skill into an editable local copy
    Eject {
        /// Name of the skill to eject
        name: String,
    },
}

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

#[derive(Subcommand)]
enum SelfCommands {
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
    let cli = Cli::parse();

    let result = match cli.command {
        Commands::Add { source, rev, bin } => match source {
            Some(src) => commands::add::run(&src, rev.as_deref(), bin),
            None => commands::install::run(),
        },
        Commands::Remove { name, yes } => commands::remove::run(&name, yes),
        Commands::Search { query, agent, interactive, source, limit, verbose } => {
            if verbose {
                env_logger::Builder::new()
                    .filter_level(log::LevelFilter::Debug)
                    .init();
            }
            commands::search::run(&query, agent, interactive, source.as_deref(), limit)
        }
        Commands::Update { name } => commands::update::run(name.as_deref()),
        Commands::Run { name, args } => commands::run::run(&name, &args),
        Commands::Skill { action } => match action {
            SkillCommands::New { path, dir, bin, collection, force } => {
                commands::new::run(path.as_deref(), dir.as_deref(), bin, collection, force)
            }
            SkillCommands::Validate { path } => commands::validate::run(path.as_deref()),
            SkillCommands::Info { skill } => commands::info::run(&skill),
            SkillCommands::List => commands::list::run(),
            SkillCommands::Link { path } => commands::link::run(&path),
            SkillCommands::Eject { name } => commands::eject::run(&name),
        },
        Commands::Project { action } => match action {
            ProjectCommands::Init { target, force } => commands::init::run(&target, force),
            ProjectCommands::Migrate { from, dry_run } => {
                commands::migrate::run(from.as_deref(), dry_run)
            }
        },
        Commands::Cache { action } => match action {
            CacheCommands::Gc { dry_run } => commands::gc::run(dry_run),
        },
        Commands::Config { action } => commands::config::run(action),
        Commands::Self_ { action } => match action {
            SelfCommands::Info => commands::self_cmd::info(),
            SelfCommands::Check => commands::self_cmd::check(),
            SelfCommands::Update { version } => commands::self_cmd::update(version.as_deref()),
        },
    };

    if let Err(e) = result {
        eprintln!("Error: {e}");
        std::process::exit(1);
    }
}
