use anyhow::Result;
use colorful::Colorful;
use copypasta::{ClipboardContext, ClipboardProvider};
use either::Either;
use reqwest::header::ACCEPT;
use secrecy::{ExposeSecret, Secret};
use spinoff::{Color, Spinner, Spinners};
use std::time::Duration;
use tokio::runtime::Builder;

pub struct Auth {
    scope: Vec<String>,
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
            scope: scope.into_iter().map(|s| s.as_ref().to_string()).collect(),
        }
    }

    pub fn github(&self) -> GithubHandler {
        GithubHandler { auth: self }
    }

    pub fn get_token(&self) -> Result<String> {
        if let Ok(token) = std::env::var("GITHUB_TOKEN") {
            return Ok(token);
        }

        self.github().get_token()
    }

    // TODO: read from config
    pub fn get_username(&self) -> Result<String> {
        let token = self.get_token()?;
        let username = self.github().get_username(token)?;
        Ok(username)
    }
}

impl GithubHandler<'_> {
    pub fn get_token(&self) -> Result<String> {
        let token = Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(self.get_token_task())?;
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

        if let Ok(mut ctx) = ClipboardContext::new() {
            if ctx.set_contents(codes.user_code.to_owned()).is_err() {
                println!(
                    "Failed to copy your one-time code to \
                clipboard, please copy it manually: {}",
                    codes.user_code.to_owned().bold()
                );
            } else {
                println!(
                    "your one-time code has been copied to \
                clipboard: {}",
                    codes.user_code.to_owned().bold()
                );
            }
        } else {
            println!("your one-time code: {}", codes.user_code.to_owned().bold());
        }

        if dialoguer::Confirm::new()
            .with_prompt("open authentication page in browser?")
            .default(true)
            .interact()?
        {
            if open::that(&codes.verification_uri).is_err() {
                println!(
                    "Failed to open this page in your browser: {}",
                    codes.verification_uri.to_owned().underlined()
                );
            }
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
