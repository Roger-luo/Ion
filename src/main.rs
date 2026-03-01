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
}

fn main() {
    let cli = Cli::parse();

    let result = match cli.command {
        Commands::Add { source, rev } => commands::add::run(&source, rev.as_deref()),
        Commands::Remove { name } => commands::remove::run(&name),
        Commands::Install => commands::install::run(),
        Commands::List => commands::list::run(),
        Commands::Info { skill } => commands::info::run(&skill),
    };

    if let Err(e) = result {
        eprintln!("Error: {e}");
        std::process::exit(1);
    }
}
