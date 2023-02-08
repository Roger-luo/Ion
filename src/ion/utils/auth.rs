use anyhow::Result;
use colorful::Colorful;
use copypasta::{ClipboardContext, ClipboardProvider};
use either::Either;
use keyring::Entry;
use reqwest::header::ACCEPT;
use secrecy::{ExposeSecret, Secret};
use spinoff::{Color, Spinner, Spinners};
use std::time::Duration;
use tokio::runtime::Builder;

pub struct Auth {
    github: Entry,
    scope: Vec<String>,
}

pub struct KeyringHandler<'a> {
    auth: &'a Auth,
}

pub struct GithubHandler<'a> {
    auth: &'a Auth,
}

impl Auth {
    pub fn new<I, S>(scope: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        Self {
            github: Entry::new("dev.rogerluo.ion-github-authentication", "github.auth"),
            scope: scope.into_iter().map(|s| s.as_ref().to_string()).collect(),
        }
    }

    pub fn keyring(&self) -> KeyringHandler {
        KeyringHandler { auth: self }
    }

    pub fn github(&self) -> GithubHandler {
        GithubHandler { auth: self }
    }

    pub fn get_token(&self) -> Result<String> {
        if let Ok(token) = std::env::var("GITHUB_TOKEN") {
            return Ok(token);
        }

        let token = match self.keyring().get_token() {
            Ok(token) => token,
            Err(_) => self.github().get_token()?,
        };
        Ok(token)
    }

    // TODO: read from config
    pub fn get_username(&self) -> Result<String> {
        let token = self.get_token()?;
        let username = self.github().get_username(token)?;
        Ok(username)
    }

    pub fn expire_token(&self) -> Result<()> {
        self.keyring().delete_token()?;
        Ok(())
    }
}

impl KeyringHandler<'_> {
    pub fn get_token(&self) -> Result<String, keyring::Error> {
        self.auth.github.get_password()
    }

    pub fn set_token(&self, access_token: &str) -> Result<(), keyring::Error> {
        self.auth.github.set_password(access_token)
    }

    pub fn delete_token(&self) -> Result<(), keyring::Error> {
        self.auth.github.delete_password()
    }
}

impl GithubHandler<'_> {
    pub fn get_token(&self) -> Result<String> {
        let token = Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(self.get_token_task())?;
        self.auth.keyring().set_token(token.as_str())?;
        Ok(token)
    }

    pub fn get_username(&self, token: String) -> Result<String> {
        let username = Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(self.get_username_task(token))?;
        Ok(username)
    }

    async fn get_username_task(&self, token: String) -> Result<String> {
        let octocrab = octocrab::Octocrab::builder()
            .personal_token(token)
            .build()?;
        let user = octocrab.current().user().await?;
        Ok(user.login)
    }

    async fn get_token_task(&self) -> Result<String> {
        let client_id = Secret::from("39fd6b5d93f0385cd1ff".to_string());
        let crab = octocrab::Octocrab::builder()
            .base_url("https://github.com")?
            .add_header(ACCEPT, "application/json".to_string())
            .build()?;
        let codes = crab
            .authenticate_as_device(&client_id, &self.auth.scope)
            .await?;

        let mut ctx = ClipboardContext::new().unwrap();
        let user_code = codes.user_code.to_owned();
        if ctx.set_contents(user_code.to_owned()).is_err() {
            println!(
                "Failed to copy your one-time code to \
            clipboard, please copy it manually: {}",
                user_code.to_owned().bold()
            );
        } else {
            println!(
                "your one-time code has been copied to \
            clipboard: {}",
                user_code.to_owned().bold()
            );
        }

        if dialoguer::Confirm::new()
            .with_prompt("open authentication page in browser?")
            .default(true)
            .interact()?
        {
            open::that(&codes.verification_uri)?;
        } else {
            println!(
                "Then open this page in your browser: {}",
                codes.verification_uri.to_owned().underlined()
            );
        }

        let spinner = Spinner::new(Spinners::Dots, "waiting github...", Color::Blue);
        let mut interval = Duration::from_secs(codes.interval);
        let mut clock = tokio::time::interval(interval);
        let auth = loop {
            clock.tick().await;
            match codes.poll_once(&crab, &client_id).await? {
                Either::Left(auth) => break auth,
                Either::Right(cont) => match cont {
                    octocrab::auth::Continue::SlowDown => {
                        // We were request to slow down. We add five seconds to the polling
                        // duration.
                        interval += Duration::from_secs(5);
                        clock = tokio::time::interval(interval);
                        // The first tick happens instantly, so we tick that off immediately.
                        clock.tick().await;
                    }
                    octocrab::auth::Continue::AuthorizationPending => {
                        // The user has not clicked authorize yet, but nothing has gone wrong.
                        // We keep polling.
                    }
                },
            }
        };
        spinner.success("Successfully authenticated!");
        Ok(auth.access_token.expose_secret().to_string())
    }
}
