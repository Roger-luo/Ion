use clap::{Parser, Subcommand};

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

    match cli.command {
        Commands::Add { source, rev } => {
            println!("Adding skill: {source}");
            if let Some(rev) = &rev {
                println!("  rev: {rev}");
            }
        }
        Commands::Remove { name } => {
            println!("Removing skill: {name}");
        }
        Commands::Install => {
            println!("Installing skills from ion.toml...");
        }
        Commands::List => {
            println!("Listing skills...");
        }
        Commands::Info { skill } => {
            println!("Info for skill: {skill}");
        }
    }
}
