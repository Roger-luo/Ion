use clap::ArgMatches;
use dialoguer::Input;
use anyhow::{Error, format_err};
use serde;
use serde_derive::Serialize;
use std::collections::HashMap;
use super::{badge::Badge, Template, Blueprint};

#[derive(Debug, Serialize)]
pub struct Author {
    pub firstname: String,
    pub lastname: Option<String>,
    pub email: Option<String>,
    pub url: Option<String>,
    pub affiliation: Option<String>,
    pub orcid: Option<String>,
}

#[derive(Debug)]
pub enum Meta {
    String(String),
    Bool(bool),
    Integer(i64),
    Badges(Vec<Badge>),
    Items(Vec<String>),
    Authors(Vec<Author>),
    Object(HashMap<String, Meta>),
}

impl serde::Serialize for Meta {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            Meta::String(s) => serializer.serialize_str(s),
            Meta::Bool(b) => serializer.serialize_bool(*b),
            Meta::Integer(i) => serializer.serialize_i64(*i),
            Meta::Badges(b) => b.serialize(serializer),
            Meta::Items(i) => i.serialize(serializer),
            Meta::Authors(a) => a.serialize(serializer),
            Meta::Object(o) => o.serialize(serializer),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct Context {
    pub prompt: bool,
    pub name: String,
    pub meta: HashMap<String, Meta>,
}

impl Context {
    pub fn from(t: &Template, matches: &ArgMatches) -> Result<Self, anyhow::Error> {
        let prompt = !matches.get_flag("no-interactive");
        let name = match matches.get_one::<String>("name") {
            Some(name) => name.to_owned(),
            None => {
                if prompt {
                    Input::<String>::new()
                        .with_prompt("name of the project")
                        .allow_empty(false)
                        .interact_text().expect("error")
                } else {
                    return Err(anyhow::format_err!("No name provided."))
                }
            },
        };

        let mut ctx = Context {
            prompt,
            name: name.to_owned(),
            meta: HashMap::new(),
        };

        ctx.meta.insert("package".to_string(), Meta::String(name.to_owned()));
        t.collect(&mut ctx)?;
        if ctx.prompt {
            t.prompt(&mut ctx)?;
        }

        let path = std::env::current_dir()?.join(name);
        if path.is_dir() {
            if matches.get_flag("force") {
                std::fs::remove_dir_all(path)?;
            } else {
                return Err(anyhow::format_err!("Directory already exists. (Use --force to overwrite.)"))
            }
        }
        return Ok(ctx);
    }

    pub fn get_key_str(&self, key: &str) -> Result<&String, Error> {
        if let Some(value) = self.meta.get(key) {
            if let Meta::String(s) = value {
                return Ok(s);
            } else {
                return Err(format_err!("Key {} is not a string.", key));
            }
        } else {
            return Err(format_err!("Key {} does not exist.", key));
        }
    }

    pub fn ignore(&mut self, path: &str) -> &mut Self {
        let ignore = self.meta.entry("ignore".to_string()).or_insert_with(|| Meta::Items(Vec::new()));
        if let Meta::Items(items) = ignore {
            items.push(path.to_string());
        }
        self
    }
}
