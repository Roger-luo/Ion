use std::io::Write;
use std::path::{Path, PathBuf};

use ion_skill::manifest::Manifest;
use ion_skill::manifest_writer;
use ion_skill::source::SkillSource;

const DEFAULT_SKILLS_DIR: &str = ".agents/skills";

fn slugify(name: &str) -> String {
    let slug: String = name
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect();

    let mut result = String::new();
    for ch in slug.chars() {
        if ch == '-' {
            if !result.ends_with('-') {
                result.push('-');
            }
        } else {
            result.push(ch);
        }
    }
    result.trim_matches('-').to_string()
}

const DEFAULT_TEMPLATE: &str = r#"---
name: {name}
description: Describe what this skill does and when to use it. Write in third person.
# allowed-tools: Bash, Read, Write, Edit
# argument-hint: [args]
# disable-model-invocation: false
# context: fork
# agent: general-purpose
---

# {title}

## Overview

Describe what this skill does.

## When to use

Describe the triggers — when should this skill activate? Be specific so the agent
knows when to apply it (e.g., "Use when reviewing pull requests" or "Use when the
user asks about deployment").

## Process

1. Step one
2. Step two

## Guidelines

- Guideline one
- Guideline two
"#;

const BIN_SKILL_TEMPLATE: &str = r#"---
name: {name}
description: A CLI tool that [does X]. Use when [trigger]. Invoke with `ion run {name}`.
allowed-tools: Bash(ion run {name} *)
metadata:
  binary: {name}
  version: 0.1.0
---

# {title}

## Overview

Describe what this tool does and when the agent should invoke it.

## Usage

```bash
ion run {name} [command] [options]
```

## Commands

Document the commands this tool supports.

## Standard Commands

All binary skills support:

```bash
ion run {name} self skill    # Output the SKILL.md
ion run {name} self info     # Show version and build info
ion run {name} self check    # Check for updates
ion run {name} self update   # Update to the latest version
```
"#;

const BIN_MAIN_TEMPLATE: &str = r#"use clap::{Parser, Subcommand};
use ionem::self_update::SelfManager;

const REPO: &str = "OWNER/{name}";

#[derive(Parser)]
#[command(name = "{name}", version, about = "{description}")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Manage this tool (skill output, updates, version info)
    #[command(name = "self")]
    Self_ {
        #[command(subcommand)]
        action: SelfCommands,
    },
}

#[derive(Subcommand)]
enum SelfCommands {
    /// Output the SKILL.md for this tool (used by Ion during install)
    Skill,
    /// Show version and build info
    Info,
    /// Check if a newer version is available
    Check,
    /// Update to the latest (or a specific) version
    Update {
        /// Install a specific version (e.g. 1.0.0)
        #[arg(long)]
        version: Option<String>,
    },
}

fn manager() -> SelfManager {
    SelfManager::new(REPO, "{name}", "v", env!("CARGO_PKG_VERSION"), env!("TARGET"))
}

fn main() {
    let cli = Cli::parse();

    let result = match cli.command {
        Some(Commands::Self_ { action }) => match action {
            SelfCommands::Skill => {
                print!(include_str!(concat!(env!("OUT_DIR"), "/SKILL.md")));
                Ok(())
            }
            SelfCommands::Info => {
                manager().print_info();
                Ok(())
            }
            SelfCommands::Check => manager().print_check().map_err(|e| e.to_string()),
            SelfCommands::Update { version } => {
                manager().run_update(version.as_deref()).map_err(|e| e.to_string())
            }
        },
        None => {
            println!("Hello from {name}! Use --help for usage info.");
            Ok(())
        }
    };

    if let Err(e) = result {
        eprintln!("Error: {e}");
        std::process::exit(1);
    }
}
"#;

const COLLECTION_README_TEMPLATE: &str = r#"# {title}

A collection of skills for AI agents.

## Skills

Add skills with:

```bash
ion skill new --path skills/<skill-name>
```
"#;

fn titleize(slug: &str) -> String {
    slug.split('-')
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                None => String::new(),
                Some(c) => c.to_uppercase().to_string() + chars.as_str(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// Prompt the user for a skill name via stdin and return the slugified result.
fn prompt_skill_name() -> anyhow::Result<String> {
    print!("Skill name: ");
    std::io::stdout().flush()?;

    let mut input = String::new();
    std::io::stdin().read_line(&mut input)?;

    let trimmed = input.trim();
    if trimmed.is_empty() {
        anyhow::bail!("Skill name cannot be empty");
    }

    let name = slugify(trimmed);
    if name.is_empty() {
        anyhow::bail!("Skill name '{trimmed}' produces an empty slug");
    }

    Ok(name)
}

/// Write a SKILL.md file into `target_dir`, using the binary template when `bin` is true.
fn write_skill_md(target_dir: &Path, name: &str, bin: bool, force: bool) -> anyhow::Result<()> {
    let skill_md_path = target_dir.join("SKILL.md");

    if skill_md_path.exists() && !force {
        anyhow::bail!(
            "SKILL.md already exists in {}. Use --force to overwrite.",
            target_dir.display()
        );
    }

    let title = titleize(name);

    if bin {
        let content = BIN_SKILL_TEMPLATE
            .replace("{name}", name)
            .replace("{title}", &title);
        std::fs::write(&skill_md_path, content)?;
    } else {
        let content = DEFAULT_TEMPLATE
            .replace("{name}", name)
            .replace("{title}", &title);
        std::fs::write(&skill_md_path, content)?;
    }

    Ok(())
}

const BIN_BUILD_RS_TEMPLATE: &str = r#"fn main() {
    ionem::build::emit_target();
    ionem::build::copy_skill_md();
}
"#;

/// Scaffold a Rust binary project in `target_dir` with clap, ion-skill, and a self subcommand.
fn scaffold_bin_project(target_dir: &Path, name: &str) -> anyhow::Result<()> {
    let status = std::process::Command::new("cargo")
        .args(["init", "--bin"])
        .current_dir(target_dir)
        .status()
        .map_err(|e| {
            anyhow::anyhow!("Failed to run cargo: {e}. Is the Rust toolchain installed?")
        })?;

    if !status.success() {
        anyhow::bail!("cargo init --bin failed");
    }

    let cargo_toml_path = target_dir.join("Cargo.toml");
    let cargo_content = std::fs::read_to_string(&cargo_toml_path)?;
    if !cargo_content.contains("clap") {
        let ionem_ver = env!("IONEM_VERSION");
        let updated = cargo_content.replace(
            "[dependencies]",
            &format!(
                "[dependencies]\nclap = {{ version = \"4\", features = [\"derive\"] }}\nionem = {{ version = \"{ionem_ver}\" }}\n\n[build-dependencies]\nionem = {{ version = \"{ionem_ver}\", default-features = false }}",
            ),
        );
        std::fs::write(&cargo_toml_path, updated)?;
    }

    let main_content = BIN_MAIN_TEMPLATE
        .replace("{name}", name)
        .replace("{description}", &format!("A CLI tool: {}", titleize(name)));
    std::fs::write(target_dir.join("src/main.rs"), main_content)?;

    // Write build.rs for TARGET env var
    std::fs::write(target_dir.join("build.rs"), BIN_BUILD_RS_TEMPLATE)?;

    Ok(())
}

/// Resolve the skills-dir for a project. Reads from Ion.toml if present, otherwise
/// uses the `--dir` argument, falling back to the default `.agents`.
fn resolve_skills_dir(project_dir: &Path, dir_flag: Option<&str>) -> String {
    let manifest_path = project_dir.join("Ion.toml");
    if let Ok(manifest) = Manifest::from_file(&manifest_path)
        && let Some(existing) = manifest.options.skills_dir
    {
        return existing;
    }
    dir_flag.unwrap_or(DEFAULT_SKILLS_DIR).to_string()
}

/// Scaffold a binary skill project via `ion init --bin [path]`.
pub fn run_bin(path: Option<&str>, force: bool, json: bool) -> anyhow::Result<()> {
    let target_dir = match path {
        Some(p) => {
            let dir = resolve_path(p)?;
            if !dir.exists() {
                std::fs::create_dir_all(&dir)?;
            }
            dir
        }
        None => std::env::current_dir()?,
    };

    let dir_name = target_dir
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("my-skill");
    let name = {
        let s = slugify(dir_name);
        if s.is_empty() {
            "my-skill".to_string()
        } else {
            s
        }
    };

    scaffold_bin_project(&target_dir, &name)?;
    write_skill_md(&target_dir, &name, true, force)?;

    if json {
        crate::json::print_success(serde_json::json!({
            "name": name,
            "path": target_dir.display().to_string(),
            "binary": true,
        }));
        return Ok(());
    }

    println!("Created binary skill project in {}", target_dir.display());
    println!("  cargo build              -- compile the binary");
    println!("  cargo run -- self skill  -- test the skill subcommand");
    Ok(())
}

pub fn run(
    path: Option<&str>,
    dir: Option<&str>,
    collection: bool,
    force: bool,
    json: bool,
) -> anyhow::Result<()> {
    // When --path is provided, use the original explicit-directory flow (no Ion.toml tracking).
    if let Some(p) = path {
        let target_dir = resolve_path(p)?;
        if !target_dir.exists() {
            std::fs::create_dir_all(&target_dir)?;
        }

        if collection {
            return run_collection(&target_dir, force, json);
        }

        return run_explicit_path(&target_dir, force, json);
    }

    // Local skill flow: no --path provided.
    // Activate when --dir is given or Ion.toml already exists in the current directory.
    let cwd = std::env::current_dir()?;
    let manifest_path = cwd.join("Ion.toml");
    let in_project = dir.is_some() || manifest_path.exists();

    if !in_project {
        // No project context and no --path: create in current directory (legacy behavior).
        if collection {
            return run_collection(&cwd, force, json);
        }
        return run_explicit_path(&cwd, force, json);
    }

    if collection {
        anyhow::bail!(
            "Cannot combine --collection with local skill creation (--dir or Ion.toml project). Use --path instead."
        );
    }

    // Determine the skills directory.
    let skills_dir = resolve_skills_dir(&cwd, dir);

    // Persist skills-dir to Ion.toml if --dir was explicitly provided.
    if let Some(d) = dir {
        manifest_writer::write_skills_dir(&manifest_path, d)?;
    }

    // In JSON mode, we cannot prompt for a skill name interactively.
    if json {
        anyhow::bail!("In --json mode, provide a skill name via --path");
    }

    // Prompt for the skill name.
    let name = prompt_skill_name()?;

    // Create the skill directory under {skills_dir}/skills/{name}/.
    let skill_dir = cwd.join(&skills_dir).join(&name);
    if skill_dir.exists() && !force {
        anyhow::bail!(
            "Skill directory already exists: {}. Use --force to overwrite.",
            skill_dir.display()
        );
    }
    std::fs::create_dir_all(&skill_dir)?;

    // Write SKILL.md.
    write_skill_md(&skill_dir, &name, false, force)?;

    // Register the skill in Ion.toml as a local skill.
    let source = SkillSource::local();
    manifest_writer::add_skill(&manifest_path, &name, &source)?;

    println!("Created local skill in {}", skill_dir.display());
    println!("Registered '{}' in Ion.toml as local skill", name);

    Ok(())
}

fn resolve_path(p: &str) -> anyhow::Result<PathBuf> {
    let p = PathBuf::from(p);
    if p.is_absolute() {
        Ok(p)
    } else {
        Ok(std::env::current_dir()?.join(p))
    }
}

/// Create a text skill in an explicit target directory (the --path flow). No Ion.toml tracking.
fn run_explicit_path(target_dir: &Path, force: bool, json: bool) -> anyhow::Result<()> {
    let dir_name = target_dir
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("my-skill");
    let name = {
        let s = slugify(dir_name);
        if s.is_empty() {
            "my-skill".to_string()
        } else {
            s
        }
    };

    write_skill_md(target_dir, &name, false, force)?;

    if json {
        crate::json::print_success(serde_json::json!({
            "name": name,
            "path": target_dir.display().to_string(),
        }));
        return Ok(());
    }

    println!("Created SKILL.md in {}", target_dir.display());
    Ok(())
}

fn run_collection(target_dir: &Path, force: bool, json: bool) -> anyhow::Result<()> {
    let readme_path = target_dir.join("README.md");

    if readme_path.exists() && !force {
        anyhow::bail!(
            "README.md already exists in {}. Use --force to overwrite.",
            target_dir.display()
        );
    }

    std::fs::create_dir_all(target_dir.join("skills"))?;

    let dir_name = target_dir
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("my-skills");
    let name = {
        let s = slugify(dir_name);
        if s.is_empty() {
            "my-skills".to_string()
        } else {
            s
        }
    };
    let title = titleize(&name);

    let content = COLLECTION_README_TEMPLATE.replace("{title}", &title);
    std::fs::write(&readme_path, content)?;

    if json {
        crate::json::print_success(serde_json::json!({
            "name": name,
            "path": target_dir.display().to_string(),
            "collection": true,
        }));
        return Ok(());
    }

    println!("Created skill collection in {}", target_dir.display());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slugify_lowercase() {
        assert_eq!(slugify("my-skill"), "my-skill");
    }

    #[test]
    fn slugify_spaces_and_caps() {
        assert_eq!(slugify("My Cool Skill"), "my-cool-skill");
    }

    #[test]
    fn slugify_underscores() {
        assert_eq!(slugify("my_cool_skill"), "my-cool-skill");
    }

    #[test]
    fn slugify_special_chars() {
        assert_eq!(slugify("skill@v2.0!"), "skill-v2-0");
    }

    #[test]
    fn slugify_leading_trailing_hyphens() {
        assert_eq!(slugify("--my-skill--"), "my-skill");
    }

    #[test]
    fn slugify_consecutive_hyphens() {
        assert_eq!(slugify("my---skill"), "my-skill");
    }

    #[test]
    fn slugify_all_special_chars_returns_empty() {
        assert_eq!(slugify("---"), "");
        assert_eq!(slugify("..."), "");
    }
}
