use std::path::PathBuf;

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

const COLLECTION_README_TEMPLATE: &str = r#"# {title}

A collection of skills for AI agents.

## Skills

Add skills with:

```bash
ion new --path skills/<skill-name>
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

pub fn run(path: Option<&str>, bin: bool, collection: bool, force: bool) -> anyhow::Result<()> {
    if collection && bin {
        anyhow::bail!("Cannot combine --collection with --bin");
    }

    let target_dir = match path {
        Some(p) => {
            let p = PathBuf::from(p);
            if p.is_absolute() {
                p
            } else {
                std::env::current_dir()?.join(p)
            }
        }
        None => std::env::current_dir()?,
    };

    if !target_dir.exists() {
        std::fs::create_dir_all(&target_dir)?;
    }

    if collection {
        return run_collection(&target_dir, force);
    }

    let skill_md_path = target_dir.join("SKILL.md");

    if skill_md_path.exists() && !force {
        anyhow::bail!(
            "SKILL.md already exists in {}. Use --force to overwrite.",
            target_dir.display()
        );
    }

    if bin {
        let status = std::process::Command::new("cargo")
            .args(["init", "--bin"])
            .current_dir(&target_dir)
            .status()
            .map_err(|e| anyhow::anyhow!("Failed to run cargo: {e}. Is the Rust toolchain installed?"))?;

        if !status.success() {
            anyhow::bail!("cargo init --bin failed");
        }
    }

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
    let title = titleize(&name);

    let content = DEFAULT_TEMPLATE
        .replace("{name}", &name)
        .replace("{title}", &title);
    std::fs::write(&skill_md_path, content)?;

    println!("Created SKILL.md in {}", target_dir.display());
    Ok(())
}

fn run_collection(target_dir: &std::path::Path, force: bool) -> anyhow::Result<()> {
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
