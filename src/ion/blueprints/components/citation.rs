use crate::blueprints::*;
use crate::spec::Author;
use chrono::Datelike;
use dialoguer::{Confirm, Input};
use serde_derive::{Deserialize, Serialize};

#[derive(Debug, Serialize, Clone)]
pub struct Info {
    pub readme: bool,
    pub title: String,
    pub authors: Vec<Author>,
    pub year: i32,
    pub journal: Option<String>,
    pub volume: Option<String>,
    pub number: Option<String>,
    pub pages: Option<String>,
    pub doi: Option<String>,
    pub url: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct Citation {
    #[serde(default = "Citation::default_template")]
    template: String,
    #[serde(default = "Citation::default_readme")]
    readme: bool,
}

impl Citation {
    pub fn default_template() -> String {
        "./CITATION.cff.hbs".to_string()
    }

    pub fn default_readme() -> bool {
        true
    }
}

impl Blueprint for Citation {
    fn collect(&self, _t: &Template, ctx: &mut Context) -> RenderResult {
        let current_date = chrono::Utc::now();
        let year = current_date.year();
        ctx.citation = Some(Info {
            readme: self.readme,
            title: ctx.project.name.to_owned(),
            authors: ctx.project.authors.to_owned(), // use authors for packages without prompt
            year,
            journal: None,
            volume: None,
            number: None,
            pages: None,
            doi: None,
            url: None,
        });
        Ok(())
    }

    fn prompt(&self, _t: &Template, ctx: &mut Context) -> RenderResult {
        if !Confirm::new()
            .with_prompt("Do you want to setup custom citation info?")
            .interact()?
        {
            return Ok(());
        }
        let authors = prompt_for_authors()?;
        let year = Input::<i32>::new()
            .with_prompt("year")
            .default(chrono::Utc::now().year())
            .interact()?;
        let journal = Input::<String>::new()
            .with_prompt("journal")
            .allow_empty(true)
            .interact()?;
        let volume = Input::<String>::new()
            .with_prompt("volume")
            .allow_empty(true)
            .interact()?;
        let number = Input::<String>::new()
            .with_prompt("number")
            .allow_empty(true)
            .interact()?;
        let pages = Input::<String>::new()
            .with_prompt("pages")
            .allow_empty(true)
            .interact()?;
        let doi = Input::<String>::new()
            .with_prompt("doi")
            .allow_empty(true)
            .interact()?;
        let url = Input::<String>::new()
            .with_prompt("url")
            .allow_empty(true)
            .interact()?;
        let readme = Confirm::new()
            .with_prompt("Do you want to add a citation section to the README?")
            .default(true)
            .interact()?;
        ctx.citation = Some(Info {
            title: ctx.project.name.to_owned(),
            readme,
            authors,
            year,
            journal: Some(journal),
            volume: Some(volume),
            number: Some(number),
            pages: Some(pages),
            doi: Some(doi),
            url: Some(url),
        });
        Ok(())
    }

    fn render(&self, _t: &Template, ctx: &Context) -> RenderResult {
        self.template.as_template()?.render(ctx, "CITATION.cff")
    }
}
