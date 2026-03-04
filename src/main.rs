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
    /// Remove a skill from the project
    Remove {
        /// Skill name or source (e.g. brainstorming, obra/superpowers)
        name: String,
        /// Skip confirmation prompt
        #[arg(long, short = 'y')]
        yes: bool,
    },
    /// Install all skills from Ion.toml
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
    /// Garbage collect stale skill repos from global storage
    Gc {
        /// Show what would be cleaned without deleting
        #[arg(long)]
        dry_run: bool,
    },
    /// Link a local skill directory into the project
    Link {
        /// Path to the local skill directory containing SKILL.md
        path: String,
    },
    /// Validate local skill definitions
    Validate {
        /// Optional path to a SKILL.md file or skill/workspace directory
        path: Option<String>,
    },
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
    /// Initialize Ion.toml with agent tool targets
    Init {
        /// Configure specific targets (e.g. claude, cursor, or name:path)
        #[arg(long, short = 't')]
        target: Vec<String>,
        /// Overwrite existing [options.targets] without prompting
        #[arg(long)]
        force: bool,
    },
    /// Run a binary skill
    Run {
        /// Name of the binary skill to run
        name: String,
        /// Arguments to pass to the binary
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    /// Update binary skills to their latest versions
    Update {
        /// Update only a specific skill (default: update all binary skills)
        name: Option<String>,
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
        Commands::Add { source, rev, bin } => commands::add::run(&source, rev.as_deref(), bin),
        Commands::Remove { name, yes } => commands::remove::run(&name, yes),
        Commands::Install => commands::install::run(),
        Commands::List => commands::list::run(),
        Commands::Info { skill } => commands::info::run(&skill),
        Commands::Migrate { from, dry_run } => commands::migrate::run(from.as_deref(), dry_run),
        Commands::Search { query, agent, interactive, source, limit, verbose } => {
            if verbose {
                env_logger::Builder::new()
                    .filter_level(log::LevelFilter::Debug)
                    .init();
            }
            commands::search::run(&query, agent, interactive, source.as_deref(), limit)
        }
        Commands::Gc { dry_run } => commands::gc::run(dry_run),
        Commands::Link { path } => commands::link::run(&path),
        Commands::New { path, bin, collection, force } => commands::new::run(path.as_deref(), bin, collection, force),
        Commands::Validate { path } => commands::validate::run(path.as_deref()),
        Commands::Init { target, force } => commands::init::run(&target, force),
        Commands::Run { name, args } => commands::run::run(&name, &args),
        Commands::Update { name } => commands::update::run(name.as_deref()),
        Commands::Config { action } => commands::config::run(action),
    };

    if let Err(e) = result {
        eprintln!("Error: {e}");
        std::process::exit(1);
    }
}
