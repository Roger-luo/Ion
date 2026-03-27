use std::path::Path;

use toml_edit::{DocumentMut, Item, Table, value};

use crate::source::{SkillSource, SkillSourceKind};
use crate::{Error, Result};

/// Add a skill entry to an Ion.toml string. Returns the updated TOML string.
pub fn add_skill(manifest_path: &Path, name: &str, source: &SkillSource) -> Result<String> {
    let content =
        std::fs::read_to_string(manifest_path).unwrap_or_else(|_| "[skills]\n".to_string());
    let mut doc: DocumentMut = content.parse().map_err(Error::TomlEdit)?;

    if !doc.contains_key("skills") {
        doc["skills"] = Item::Table(Table::new());
    }

    let skills = doc["skills"]
        .as_table_mut()
        .ok_or_else(|| Error::Manifest("[skills] is not a table".to_string()))?;

    skills[name] = skill_to_toml(source);

    let result = doc.to_string();
    std::fs::write(manifest_path, &result).map_err(Error::Io)?;
    Ok(result)
}

/// Remove a skill entry from an Ion.toml file. Returns the updated TOML string.
pub fn remove_skill(manifest_path: &Path, name: &str) -> Result<String> {
    let content = std::fs::read_to_string(manifest_path).map_err(Error::Io)?;
    let mut doc: DocumentMut = content.parse().map_err(Error::TomlEdit)?;

    let skills = doc["skills"]
        .as_table_mut()
        .ok_or_else(|| Error::Manifest("[skills] is not a table".to_string()))?;

    if !skills.contains_key(name) {
        return Err(Error::Manifest(format!(
            "Skill '{name}' not found in manifest"
        )));
    }

    skills.remove(name);

    let result = doc.to_string();
    std::fs::write(manifest_path, &result).map_err(Error::Io)?;
    Ok(result)
}

/// Write target entries to an Ion.toml file's [options.targets] section.
/// Creates the file with a [skills] section if it doesn't exist.
/// Preserves all existing content.
pub fn write_targets(
    manifest_path: &Path,
    targets: &std::collections::BTreeMap<String, String>,
) -> Result<String> {
    let content =
        std::fs::read_to_string(manifest_path).unwrap_or_else(|_| "[skills]\n".to_string());
    let mut doc: DocumentMut = content.parse().map_err(Error::TomlEdit)?;

    if !doc.contains_key("skills") {
        doc["skills"] = Item::Table(Table::new());
    }

    if !doc.contains_key("options") {
        doc["options"] = Item::Table(Table::new());
    }
    let options = doc["options"]
        .as_table_mut()
        .ok_or_else(|| Error::Manifest("[options] is not a table".to_string()))?;

    options["targets"] = Item::Table(Table::new());
    let targets_table = options["targets"].as_table_mut().unwrap();
    for (k, v) in targets {
        targets_table[k.as_str()] = value(v.as_str());
    }

    let result = doc.to_string();
    std::fs::write(manifest_path, &result).map_err(Error::Io)?;
    Ok(result)
}

/// Write a skills-dir value to an Ion.toml file's [options] section.
/// Creates the file with a [skills] section if it doesn't exist.
/// Preserves all existing content.
pub fn write_skills_dir(manifest_path: &Path, skills_dir: &str) -> Result<String> {
    let content =
        std::fs::read_to_string(manifest_path).unwrap_or_else(|_| "[skills]\n".to_string());
    let mut doc: DocumentMut = content.parse().map_err(Error::TomlEdit)?;

    if !doc.contains_key("skills") {
        doc["skills"] = Item::Table(Table::new());
    }

    if !doc.contains_key("options") {
        doc["options"] = Item::Table(Table::new());
    }
    let options = doc["options"]
        .as_table_mut()
        .ok_or_else(|| Error::Manifest("[options] is not a table".to_string()))?;

    options["skills-dir"] = value(skills_dir);

    let result = doc.to_string();
    std::fs::write(manifest_path, &result).map_err(Error::Io)?;
    Ok(result)
}

/// Write an [agents] section to an Ion.toml file.
/// Creates the file with a [skills] section if it doesn't exist.
/// Preserves all existing content.
pub fn write_agents_config(
    manifest_path: &Path,
    template: &str,
    rev: Option<&str>,
    path: Option<&str>,
) -> Result<String> {
    let content =
        std::fs::read_to_string(manifest_path).unwrap_or_else(|_| "[skills]\n".to_string());
    let mut doc: DocumentMut = content.parse().map_err(Error::TomlEdit)?;

    if !doc.contains_key("skills") {
        doc["skills"] = Item::Table(Table::new());
    }

    let mut agents_table = Table::new();
    agents_table["template"] = value(template);
    if let Some(r) = rev {
        agents_table["rev"] = value(r);
    }
    if let Some(p) = path {
        agents_table["path"] = value(p);
    }
    doc["agents"] = Item::Table(agents_table);

    let result = doc.to_string();
    std::fs::write(manifest_path, &result).map_err(Error::Io)?;
    Ok(result)
}

/// Build a TOML representation of a skill source.
fn skill_to_toml(source: &SkillSource) -> Item {
    // Extract kind-specific optional fields
    let (binary_name, asset_pattern, dev) = match &source.kind {
        SkillSourceKind::Binary {
            binary_name,
            asset_pattern,
            dev,
            ..
        } => (Some(binary_name.as_str()), asset_pattern.as_deref(), *dev),
        _ => (None, None, false),
    };
    let forked_from = match &source.kind {
        SkillSourceKind::Local { forked_from } => forked_from.as_deref(),
        _ => None,
    };

    let needs_table = source.rev.is_some()
        || source.version.is_some()
        || source.path.is_some()
        || binary_name.is_some()
        || asset_pattern.is_some()
        || forked_from.is_some()
        || dev
        || source.is_local();

    if !needs_table {
        let display = if source.is_github() {
            match &source.path {
                Some(path) => format!("{}/{}", source.source, path),
                None => source.source.clone(),
            }
        } else {
            source.source.clone()
        };
        return value(display);
    }

    let mut table = toml_edit::InlineTable::new();

    match &source.kind {
        SkillSourceKind::Github => {}
        SkillSourceKind::Git => {
            table.insert("type", "git".into());
        }
        SkillSourceKind::Http => {
            table.insert("type", "http".into());
        }
        SkillSourceKind::Path => {
            table.insert("type", "path".into());
        }
        SkillSourceKind::Binary { .. } => {
            table.insert("type", "binary".into());
        }
        SkillSourceKind::Local { .. } => {
            table.insert("type", "local".into());
        }
    }

    // Local skills have no source field
    if !source.is_local() {
        let source_str = if source.is_github() {
            match &source.path {
                Some(path) => format!("{}/{}", source.source, path),
                None => source.source.clone(),
            }
        } else {
            source.source.clone()
        };
        table.insert("source", source_str.into());
    }

    if let Some(ref v) = source.version {
        table.insert("version", v.as_str().into());
    }
    if let Some(ref r) = source.rev {
        table.insert("rev", r.as_str().into());
    }
    if let Some(ref p) = source.path
        && !source.is_github()
    {
        table.insert("path", p.as_str().into());
    }
    if let Some(b) = binary_name {
        table.insert("binary", b.into());
    }
    if let Some(ap) = asset_pattern {
        table.insert("asset-pattern", ap.into());
    }
    if let Some(ff) = forked_from {
        table.insert("forked-from", ff.into());
    }
    if dev {
        table.insert("dev", true.into());
    }

    value(table)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_skill_to_empty_manifest() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("Ion.toml");
        std::fs::write(&path, "[skills]\n").unwrap();

        let result = add_skill(
            &path,
            "brainstorming",
            &SkillSource::infer("obra/superpowers/brainstorming").unwrap(),
        )
        .unwrap();

        assert!(result.contains("brainstorming"));
        assert!(result.contains("obra/superpowers/brainstorming"));
    }

    #[test]
    fn add_skill_with_rev() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("Ion.toml");
        std::fs::write(&path, "[skills]\n").unwrap();

        let mut source = SkillSource::infer("org/my-skill").unwrap();
        source.rev = Some("v1.0".to_string());

        let result = add_skill(&path, "my-skill", &source).unwrap();
        assert!(result.contains("rev"));
        assert!(result.contains("v1.0"));
    }

    #[test]
    fn remove_skill_from_manifest() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("Ion.toml");
        std::fs::write(
            &path,
            "[skills]\nbrainstorming = \"obra/superpowers/brainstorming\"\n",
        )
        .unwrap();

        let result = remove_skill(&path, "brainstorming").unwrap();
        assert!(!result.contains("brainstorming"));
    }

    #[test]
    fn remove_nonexistent_skill_is_error() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("Ion.toml");
        std::fs::write(&path, "[skills]\n").unwrap();

        assert!(remove_skill(&path, "nonexistent").is_err());
    }

    #[test]
    fn write_targets_to_empty_manifest() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("Ion.toml");
        std::fs::write(&path, "[skills]\n").unwrap();

        let targets = std::collections::BTreeMap::from([(
            "claude".to_string(),
            ".claude/skills".to_string(),
        )]);
        write_targets(&path, &targets).unwrap();

        let content = std::fs::read_to_string(&path).unwrap();
        assert!(content.contains("[skills]"), "existing content preserved");
        assert!(content.contains("[options]"));
        assert!(content.contains("claude"));
        assert!(content.contains(".claude/skills"));
    }

    #[test]
    fn write_targets_preserves_existing_skills() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("Ion.toml");
        std::fs::write(
            &path,
            "[skills]\nbrainstorming = \"obra/superpowers/brainstorming\"\n",
        )
        .unwrap();

        let targets = std::collections::BTreeMap::from([(
            "claude".to_string(),
            ".claude/skills".to_string(),
        )]);
        write_targets(&path, &targets).unwrap();

        let content = std::fs::read_to_string(&path).unwrap();
        assert!(content.contains("brainstorming"));
        assert!(content.contains("claude"));
    }

    #[test]
    fn write_targets_to_new_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("Ion.toml");

        let targets = std::collections::BTreeMap::from([(
            "claude".to_string(),
            ".claude/skills".to_string(),
        )]);
        write_targets(&path, &targets).unwrap();

        let content = std::fs::read_to_string(&path).unwrap();
        assert!(content.contains("[skills]"));
        assert!(content.contains("claude"));
    }

    #[test]
    fn add_local_skill_to_manifest() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("Ion.toml");
        std::fs::write(&path, "[skills]\n").unwrap();

        let source = SkillSource::local();

        let result = add_skill(&path, "my-local-skill", &source).unwrap();
        assert!(result.contains("my-local-skill"));
        assert!(result.contains("type = \"local\""));
        assert!(
            !result.contains("source"),
            "local skills should not have a source field"
        );
    }

    #[test]
    fn add_local_skill_with_forked_from() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("Ion.toml");
        std::fs::write(&path, "[skills]\n").unwrap();

        let source = SkillSource::local().with_forked_from("org/original-skill");

        let result = add_skill(&path, "my-forked-skill", &source).unwrap();
        assert!(result.contains("type = \"local\""));
        assert!(result.contains("forked-from = \"org/original-skill\""));
        assert!(
            !result.contains("source ="),
            "local skills should not have a source field"
        );
    }

    #[test]
    fn write_agents_config_to_manifest() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("Ion.toml");
        std::fs::write(
            &path,
            "[skills]\nbrainstorming = \"obra/superpowers/brainstorming\"\n",
        )
        .unwrap();

        write_agents_config(&path, "org/agents-templates", None, None).unwrap();

        let content = std::fs::read_to_string(&path).unwrap();
        assert!(content.contains("[agents]"));
        assert!(content.contains("template = \"org/agents-templates\""));
        assert!(
            content.contains("brainstorming"),
            "existing skills preserved"
        );
    }

    #[test]
    fn write_agents_config_with_rev_and_path() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("Ion.toml");
        std::fs::write(&path, "[skills]\n").unwrap();

        write_agents_config(
            &path,
            "org/agents-templates",
            Some("v2.0"),
            Some("templates/AGENTS.md"),
        )
        .unwrap();

        let content = std::fs::read_to_string(&path).unwrap();
        assert!(content.contains("template = \"org/agents-templates\""));
        assert!(content.contains("rev = \"v2.0\""));
        assert!(content.contains("path = \"templates/AGENTS.md\""));
    }

    #[test]
    fn write_skills_dir_to_manifest() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("Ion.toml");
        std::fs::write(
            &path,
            "[skills]\nbrainstorming = \"obra/superpowers/brainstorming\"\n",
        )
        .unwrap();

        let result = write_skills_dir(&path, "my-skills").unwrap();
        assert!(result.contains("[options]"));
        assert!(result.contains("skills-dir = \"my-skills\""));
        assert!(
            result.contains("brainstorming"),
            "existing skills should be preserved"
        );
    }
}
