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
