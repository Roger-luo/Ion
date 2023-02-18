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
        write!(
            format_buffer,
            "Name:\n{}\n\nDescription:\n{}\n\n",
            self.name, self.description
        )?;

        if let Some(repo) = &self.repo {
            write!(format_buffer, "Repo:\n{}\n", repo)?;
        } else {
            write!(format_buffer, "Repo:\nNone\n\n")?;
        }

        if let Some(project_file) = &self.project_file {
            write!(format_buffer, "Project File:\n{}\n", project_file)?;
        } else {
            write!(format_buffer, "Project File:\nNone\n\n")?;
        }

        if let Some(readme) = &self.readme {
            write!(format_buffer, "Readme:\n{}\n", readme)?;
        } else {
            write!(format_buffer, "Readme:\nNone\n\n")?;
        }

        if let Some(src_dir) = &self.src_dir {
            write!(format_buffer, "Source Directory:\n{}\n", src_dir)?;
        } else {
            write!(format_buffer, "Source Directory:\nNone\n\n")?;
        }

        if let Some(tests) = &self.tests {
            write!(format_buffer, "Tests:\n{}\n", tests)?;
        } else {
            write!(format_buffer, "Tests:\nNone\n\n")?;
        }

        if let Some(license_dir) = &self.license {
            write!(format_buffer, "License Template:\n{}\n", license_dir)?;
        } else {
            write!(format_buffer, "License Template:\nNone\n\n")?;
        }

        if let Some(citation) = &self.citation {
            write!(format_buffer, "Citation:\n{}\n", citation)?;
        } else {
            write!(format_buffer, "Citation:\nNone\n\n")?;
        }

        if let Some(documenter) = &self.documenter {
            write!(format_buffer, "Documenter:\n{}\n", documenter)?;
        } else {
            write!(format_buffer, "Documenter:\nNone\n\n")?;
        }

        if let Some(codecov) = &self.codecov {
            write!(format_buffer, "CodeCov:\n{}\n", codecov)?;
        } else {
            write!(format_buffer, "CodeCov:\nNone\n\n")?;
        }

        if let Some(coveralls) = &self.coveralls {
            write!(format_buffer, "Coveralls:\n{}\n", coveralls)?;
        } else {
            write!(format_buffer, "Coveralls:\nNone\n\n")?;
        }
        if let Some(github) = &self.github {
            write!(format_buffer, "Github:\n{}\n", github)?;
        } else {
            write!(format_buffer, "Github:\nNone\n")?;
        }

        Ok(())
    }
}
