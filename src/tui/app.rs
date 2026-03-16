use std::path::{Path, PathBuf};

use ion_skill::config::GlobalConfig;
use ion_skill::manifest::Manifest;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    Global,
    Project,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputMode {
    Normal,
    EditingValue,
    AddingKey,
    AddingValue,
    ConfirmDelete,
    ConfirmQuit,
}

#[derive(Debug, Clone)]
pub struct ConfigSection {
    pub name: String,
    pub entries: Vec<(String, String)>,
}

pub struct App {
    pub tab: Tab,
    pub input_mode: InputMode,
    pub global_sections: Vec<ConfigSection>,
    pub project_sections: Vec<ConfigSection>,
    pub cursor: usize,
    pub input_buffer: String,
    pub adding_key_buffer: String,
    pub dirty: bool,
    pub has_project: bool,
    pub global_config_path: PathBuf,
    pub manifest_path: Option<PathBuf>,
    pub status_message: Option<String>,
    pub should_quit: bool,
}

impl App {
    pub fn new(
        global_config_path: PathBuf,
        manifest_path: Option<PathBuf>,
    ) -> anyhow::Result<Self> {
        let global_config = GlobalConfig::load_from(&global_config_path).unwrap_or_default();
        let global_sections = Self::build_global_sections(&global_config);

        let (project_sections, has_project) = match &manifest_path {
            Some(mp) if mp.exists() => {
                let manifest = Manifest::from_file(mp)?;
                (Self::build_project_sections(&manifest), true)
            }
            _ => (Vec::new(), false),
        };

        Ok(Self {
            tab: Tab::Global,
            input_mode: InputMode::Normal,
            global_sections,
            project_sections,
            cursor: 0,
            input_buffer: String::new(),
            adding_key_buffer: String::new(),
            dirty: false,
            has_project,
            global_config_path,
            manifest_path,
            status_message: None,
            should_quit: false,
        })
    }

    fn build_global_sections(config: &GlobalConfig) -> Vec<ConfigSection> {
        vec![
            ConfigSection {
                name: "targets".to_string(),
                entries: config
                    .targets
                    .iter()
                    .map(|(k, v)| (k.clone(), v.clone()))
                    .collect(),
            },
            ConfigSection {
                name: "sources".to_string(),
                entries: config
                    .sources
                    .iter()
                    .map(|(k, v)| (k.clone(), v.clone()))
                    .collect(),
            },
            ConfigSection {
                name: "cache".to_string(),
                entries: vec![(
                    "max-age-days".to_string(),
                    config
                        .cache
                        .max_age_days
                        .map_or("1".to_string(), |v| v.to_string()),
                )],
            },
            ConfigSection {
                name: "ui".to_string(),
                entries: vec![(
                    "color".to_string(),
                    config
                        .ui
                        .color
                        .map_or("true".to_string(), |v| v.to_string()),
                )],
            },
        ]
    }

    fn build_project_sections(manifest: &Manifest) -> Vec<ConfigSection> {
        vec![
            ConfigSection {
                name: "targets".to_string(),
                entries: manifest
                    .options
                    .targets
                    .iter()
                    .map(|(k, v)| (k.clone(), v.clone()))
                    .collect(),
            },
            ConfigSection {
                name: "options".to_string(),
                entries: vec![(
                    "skills-dir".to_string(),
                    manifest
                        .options
                        .skills_dir
                        .clone()
                        .unwrap_or_else(|| ".agents/skills".to_string()),
                )],
            },
        ]
    }

    pub fn current_sections(&self) -> &[ConfigSection] {
        match self.tab {
            Tab::Global => &self.global_sections,
            Tab::Project => &self.project_sections,
        }
    }

    pub fn current_sections_mut(&mut self) -> &mut Vec<ConfigSection> {
        match self.tab {
            Tab::Global => &mut self.global_sections,
            Tab::Project => &mut self.project_sections,
        }
    }

    pub fn total_entries(&self) -> usize {
        self.current_sections()
            .iter()
            .map(|s| s.entries.len())
            .sum()
    }

    pub fn cursor_position(&self) -> Option<(usize, usize)> {
        let mut remaining = self.cursor;
        for (si, section) in self.current_sections().iter().enumerate() {
            if remaining < section.entries.len() {
                return Some((si, remaining));
            }
            remaining -= section.entries.len();
        }
        None
    }

    pub fn current_entry(&self) -> Option<(String, String)> {
        let (si, ei) = self.cursor_position()?;
        let section = &self.current_sections()[si];
        let (key, value) = &section.entries[ei];
        Some((format!("{}.{}", section.name, key), value.clone()))
    }

    /// Compute the number of content lines needed for a set of sections.
    fn content_height(sections: &[ConfigSection]) -> u16 {
        if sections.is_empty() {
            return 1; // "No config values" message
        }
        let mut h: u16 = 0;
        for section in sections {
            h += 1; // section header
            h += section.entries.len() as u16;
            h += 1; // blank line after section
        }
        h
    }

    /// Maximum inline viewport height across both tabs.
    pub fn viewport_height(&self) -> u16 {
        let global_h = Self::content_height(&self.global_sections);
        let project_h = if self.has_project {
            Self::content_height(&self.project_sections)
        } else {
            1 // "No Ion.toml" message
        };
        let content_h = global_h.max(project_h);
        // tabs(3) + content + hint(2) + status(1) + help(2)
        3 + content_h + 2 + 1 + 2
    }

    pub fn current_section_name(&self) -> Option<String> {
        let (si, _) = self.cursor_position()?;
        Some(self.current_sections()[si].name.clone())
    }

    pub fn save(&mut self) -> anyhow::Result<()> {
        let config = self.sections_to_global_config();
        config.save_to(&self.global_config_path)?;

        if let Some(ref mp) = self.manifest_path
            && mp.exists()
        {
            self.save_project_config(mp)?;
        }

        self.dirty = false;
        self.status_message = Some("Saved!".to_string());
        Ok(())
    }

    fn sections_to_global_config(&self) -> GlobalConfig {
        let mut config = GlobalConfig::default();
        for section in &self.global_sections {
            match section.name.as_str() {
                "targets" => {
                    for (k, v) in &section.entries {
                        config.targets.insert(k.clone(), v.clone());
                    }
                }
                "sources" => {
                    for (k, v) in &section.entries {
                        config.sources.insert(k.clone(), v.clone());
                    }
                }
                "cache" => {
                    for (k, v) in &section.entries {
                        if k == "max-age-days" {
                            config.cache.max_age_days = v.parse().ok();
                        }
                    }
                }
                "ui" => {
                    for (k, v) in &section.entries {
                        if k == "color" {
                            config.ui.color = v.parse().ok();
                        }
                    }
                }
                _ => {}
            }
        }
        config
    }

    fn save_project_config(&self, manifest_path: &Path) -> anyhow::Result<()> {
        use toml_edit::{DocumentMut, Item, Table};

        let content = std::fs::read_to_string(manifest_path)?;
        let mut doc: DocumentMut = content.parse()?;

        if !doc.contains_key("options") {
            doc["options"] = Item::Table(Table::new());
        }
        let options = doc["options"]
            .as_table_mut()
            .ok_or_else(|| anyhow::anyhow!("[options] is not a table"))?;

        // Save targets
        options["targets"] = Item::Table(Table::new());
        if let Some(section) = self.project_sections.iter().find(|s| s.name == "targets") {
            let targets_table = options["targets"].as_table_mut().unwrap();
            for (k, v) in &section.entries {
                targets_table[k.as_str()] = toml_edit::value(v.as_str());
            }
        }

        // Save top-level options (skills-dir, etc.)
        // Only write skills-dir if it differs from the default
        if let Some(section) = self.project_sections.iter().find(|s| s.name == "options") {
            for (k, v) in &section.entries {
                if k == "skills-dir" && v == ".agents/skills" {
                    options.remove(k.as_str());
                } else {
                    options[k.as_str()] = toml_edit::value(v.as_str());
                }
            }
        }

        std::fs::write(manifest_path, doc.to_string())?;
        Ok(())
    }
}
