use std::path::Path;

use toml_edit::{DocumentMut, Item, Table, value};

use crate::source::{SkillSource, SourceType};
use crate::{Error, Result};

/// Add a skill entry to an ion.toml string. Returns the updated TOML string.
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

/// Remove a skill entry from an ion.toml file. Returns the updated TOML string.
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

/// Build a TOML representation of a skill source.
fn skill_to_toml(source: &SkillSource) -> Item {
    let needs_table = source.rev.is_some() || source.version.is_some() || source.path.is_some();

    if !needs_table {
        let display = match (&source.source_type, &source.path) {
            (SourceType::Github, Some(path)) => format!("{}/{}", source.source, path),
            _ => source.source.clone(),
        };
        return value(display);
    }

    let mut table = toml_edit::InlineTable::new();

    match source.source_type {
        SourceType::Github => {}
        SourceType::Git => {
            table.insert("type", "git".into());
        }
        SourceType::Http => {
            table.insert("type", "http".into());
        }
        SourceType::Path => {
            table.insert("type", "path".into());
        }
    }

    let source_str = match (&source.source_type, &source.path) {
        (SourceType::Github, Some(path)) => format!("{}/{}", source.source, path),
        _ => source.source.clone(),
    };
    table.insert("source", source_str.into());

    if let Some(ref v) = source.version {
        table.insert("version", v.as_str().into());
    }
    if let Some(ref r) = source.rev {
        table.insert("rev", r.as_str().into());
    }
    if let Some(ref p) = source.path
        && source.source_type != SourceType::Github
    {
        table.insert("path", p.as_str().into());
    }

    value(table)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_skill_to_empty_manifest() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("ion.toml");
        std::fs::write(&path, "[skills]\n").unwrap();

        let result = add_skill(
            &path,
            "brainstorming",
            &SkillSource::infer("anthropics/skills/brainstorming").unwrap(),
        )
        .unwrap();

        assert!(result.contains("brainstorming"));
        assert!(result.contains("anthropics/skills/brainstorming"));
    }

    #[test]
    fn add_skill_with_rev() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("ion.toml");
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
        let path = dir.path().join("ion.toml");
        std::fs::write(
            &path,
            "[skills]\nbrainstorming = \"anthropics/skills/brainstorming\"\n",
        )
        .unwrap();

        let result = remove_skill(&path, "brainstorming").unwrap();
        assert!(!result.contains("brainstorming"));
    }

    #[test]
    fn remove_nonexistent_skill_is_error() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("ion.toml");
        std::fs::write(&path, "[skills]\n").unwrap();

        assert!(remove_skill(&path, "nonexistent").is_err());
    }
}
