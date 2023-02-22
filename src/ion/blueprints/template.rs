use super::badge::Badge;
use super::components::*;
use super::Blueprint;
use super::{Julia, Project};

use ion_derive::Template;
use serde_derive::Deserialize;
use std::fmt;

#[derive(Debug, Deserialize, Template)]
pub struct Template {
    pub name: String,        // name of the template
    pub description: String, // description of the template
    // the following has order of appearance in prompts
    pub repo: Option<GitRepo>,
    pub project_file: Option<ProjectFile>,
    pub readme: Option<Readme>,
    pub src_dir: Option<SrcDir>,
    pub tests: Option<ProjectTest>,
    pub license: Option<License>,
    pub citation: Option<Citation>,
    pub documenter: Option<Documenter>,
    pub codecov: Option<Codecov>,
    pub coveralls: Option<Coveralls>,
    pub github: Option<GitHub>,
}

impl fmt::Display for Template {
    fn fmt(&self, format_buffer: &mut fmt::Formatter) -> fmt::Result {
        // Name & Description are req'd fields
        writeln!(format_buffer, "{:#?}", self.name)?;

        writeln!(format_buffer, "{:#?}", self.description)?;

        if let Some(repo) = &self.repo {
            writeln!(format_buffer, "{:#?}", repo)?;
        }

        if let Some(project_file) = &self.project_file {
            writeln!(format_buffer, "{:#?}", project_file)?;
        }

        if let Some(readme) = &self.readme {
            writeln!(format_buffer, "{:#?}", readme)?;
        }

        if let Some(src_dir) = &self.src_dir {
            writeln!(format_buffer, "{:#?}", src_dir)?;
        }

        if let Some(tests) = &self.tests {
            writeln!(format_buffer, "{:#?}", tests)?;
        }

        if let Some(license_dir) = &self.license {
            writeln!(format_buffer, "{:#?}", license_dir)?;
        }

        if let Some(citation) = &self.citation {
            writeln!(format_buffer, "{:#?}", citation)?;
        }

        if let Some(documenter) = &self.documenter {
            writeln!(format_buffer, "{:#?}", documenter)?;
        }

        if let Some(codecov) = &self.codecov {
            writeln!(format_buffer, "{:#?}", codecov)?;
        }

        if let Some(coveralls) = &self.coveralls {
            writeln!(format_buffer, "{:#?}", coveralls)?;
        }

        if let Some(github) = &self.github {
            writeln!(format_buffer, "{:#?}", github)?;
        }

        Ok(())
    }
}
