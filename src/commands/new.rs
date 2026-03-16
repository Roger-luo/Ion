use std::io::Write;
use std::path::{Path, PathBuf};

use ion_skill::manifest::Manifest;
use ion_skill::manifest_writer;
use ion_skill::source::{SkillSource, SourceType};

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
description: A brief description of what this skill does
# license: MIT
# compatibility: claude-code
# allowed-tools: Bash, Read, Write
# metadata:
#   author: your-name
#   version: 0.1.0
---

# {title}

## Overview

Describe what this skill does and when to use it.

## Process

1. Step one
2. Step two

## Examples

```bash
# Example usage
```
"#;

const BIN_SKILL_TEMPLATE: &str = r#"---
name: {name}
description: A CLI tool that provides agent capabilities. Invoke with `ion run {name}`.
metadata:
  binary: {name}
  version: 0.1.0
---

# {title}

## Overview

Describe what this tool does. The agent invokes this via `ion run {name} [args]`.

## Usage

```bash
ion run {name} [command] [options]
```
"#;

const BIN_MAIN_TEMPLATE: &str = r#"use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "{name}", version, about = "{description}")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Output the SKILL.md for this tool (used by Ion during install)
    Skill,
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Skill) => print_skill(),
        None => println!("Hello from {name}! Use --help for usage info."),
    }
}

fn print_skill() {
    print!(include_str!("../SKILL.md"));
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

/// Scaffold a Rust binary project in `target_dir` with clap and a skill subcommand.
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
        let updated = cargo_content.replace(
            "[dependencies]",
            "[dependencies]\nclap = { version = \"4\", features = [\"derive\"] }",
        );
        std::fs::write(&cargo_toml_path, updated)?;
    }

    let main_content = BIN_MAIN_TEMPLATE
        .replace("{name}", name)
        .replace("{description}", &format!("A CLI tool: {}", titleize(name)));
    std::fs::write(target_dir.join("src/main.rs"), main_content)?;

    Ok(())
}

/// Resolve the skills-dir for a project. Reads from Ion.toml if present, otherwise
/// uses the `--dir` argument, falling back to the default `.agents`.
fn resolve_skills_dir(project_dir: &Path, dir_flag: Option<&str>) -> String {
    let manifest_path = project_dir.join("Ion.toml");
    if let Ok(manifest) = Manifest::from_file(&manifest_path) {
        if let Some(existing) = manifest.options.skills_dir {
            return existing;
        }
    }
    dir_flag.unwrap_or(DEFAULT_SKILLS_DIR).to_string()
}

pub fn run(
    path: Option<&str>,
    dir: Option<&str>,
    bin: bool,
    collection: bool,
    force: bool,
    json: bool,
) -> anyhow::Result<()> {
    if collection && bin {
        anyhow::bail!("Cannot combine --collection with --bin");
    }

    // When --path is provided, use the original explicit-directory flow (no Ion.toml tracking).
    if let Some(p) = path {
        let target_dir = resolve_path(p)?;
        if !target_dir.exists() {
            std::fs::create_dir_all(&target_dir)?;
        }

        if collection {
            return run_collection(&target_dir, force, json);
        }

        return run_explicit_path(&target_dir, bin, force, json);
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
        return run_explicit_path(&cwd, bin, force, json);
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

    // Scaffold binary project if --bin is set.
    if bin {
        scaffold_bin_project(&skill_dir, &name)?;
    }

    // Write SKILL.md.
    write_skill_md(&skill_dir, &name, bin, force)?;

    // Register the skill in Ion.toml as a local skill.
    let source = SkillSource {
        source_type: SourceType::Local,
        source: String::new(),
        path: None,
        rev: None,
        version: None,
        binary: None,
        asset_pattern: None,
        forked_from: None,
    };
    manifest_writer::add_skill(&manifest_path, &name, &source)?;

    if bin {
        println!("Created local binary skill in {}", skill_dir.display());
        println!("  cargo build    -- compile the binary");
        println!("  cargo run -- skill  -- test the skill subcommand");
    } else {
        println!("Created local skill in {}", skill_dir.display());
    }
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

/// Create a skill in an explicit target directory (the --path flow). No Ion.toml tracking.
fn run_explicit_path(target_dir: &Path, bin: bool, force: bool, json: bool) -> anyhow::Result<()> {
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

    if bin {
        scaffold_bin_project(target_dir, &name)?;
        write_skill_md(target_dir, &name, true, force)?;

        if json {
            crate::json::print_success(serde_json::json!({
                "name": name,
                "path": target_dir.display().to_string(),
                "binary": true,
            }));
            return Ok(());
        }

        println!("Created binary skill project in {}", target_dir.display());
        println!("  cargo build    -- compile the binary");
        println!("  cargo run -- skill  -- test the skill subcommand");
        return Ok(());
    }

    write_skill_md(target_dir, &name, false, force)?;

    if json {
        crate::json::print_success(serde_json::json!({
            "name": name,
            "path": target_dir.display().to_string(),
            "binary": false,
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
