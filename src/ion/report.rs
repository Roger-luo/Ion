use node_semver::Version;
use colorful::Colorful;
use core::fmt::{Display, Formatter};
use colorful::core::color_string::CString;

#[derive(Debug)]
pub struct ReleaseReport {
    pub name: String,
    pub current_version: Version,
    pub latest_version: Option<Version>,
    pub release_version: Version,
    pub registry: Option<String>,
    pub branch: Option<String>,
    pub commit: Option<String>,
    pub subdir: Option<String>,
}

impl Display for ReleaseReport {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let mut fields: Vec<CString> = vec!["project".blue()];
        let mut values: Vec<String> = vec![self.name.to_owned()];

        if let Some(registry) = &self.registry {
            fields.push("registry".blue());
            values.push(registry.to_string());
        }

        if let Some(latest) = &self.latest_version {
            if *latest == self.current_version {
                fields.push("latest/current version".blue());
                values.push(latest.to_string());
            } else {
                fields.push("latest version".blue());
                values.push(latest.to_string());
            }
        } else {
            fields.push("current version".blue());
            values.push(self.current_version.to_string());
        }

        fields.push("release version".blue());
        values.push(self.release_version.to_string());

        if let Some(branch) = &self.branch {
            fields.push("branch".blue());
            values.push(branch.to_string());
        }

        if let Some(commit) = &self.commit {
            fields.push("commit".blue());
            values.push(commit[0..8].to_string());
        }

        if let Some(subdir) = &self.subdir {
            fields.push("subdir".blue());
            values.push(subdir.to_string());
        }

        let mut max_field_len = 0;
        for field in &fields {
            let len = field.to_string().len();
            if len > max_field_len {
                max_field_len = len;
            }
        }

        for (field, value) in fields.iter().zip(values.iter()) {
            writeln!(f, "   {:>width$} : {}", field.to_string(), value, width = max_field_len)?;
        }
        Ok(())
    }
}
