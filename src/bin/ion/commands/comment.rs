use clap::parser::ArgMatches;
use clap::Command;
use ion::errors::CliResult;
use either::Either;
use reqwest::header::ACCEPT;
use std::time::Duration;
use tokio::runtime::Builder;
use octocrab::Octocrab;
use secrecy::ExposeSecret;

pub fn cli() -> Command {
    Command::new("comment")
        .about("release a new version of a package")
}

pub fn exec(_matches: &ArgMatches) -> CliResult {
    Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(send_comment())?;
    Ok(())
}

async fn send_comment() -> CliResult {
    let client_id = secrecy::Secret::from("39fd6b5d93f0385cd1ff".to_string());
    let crab = octocrab::Octocrab::builder()
        .base_url("https://github.com")?
        .add_header(ACCEPT, "application/json".to_string())
        .build()?;
    let codes = crab
        .authenticate_as_device(&client_id, ["public_repo", "read:org"])
        .await?;
    println!(
        "Go to {} and enter code {}",
        codes.verification_uri, codes.user_code
    );

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

    println!("Authorization succeeded with access to {:?}", auth.scope);

    let token = auth.access_token.expose_secret().to_owned();
    let octocrab = Octocrab::builder().personal_token(token).build()?;
    let commits = octocrab.commits("Roger-luo", "IonCLI.jl");
    let future = commits.create_comment(
        "3645a1db7690e4ddafb01ec40732914866ea3ade",
        "test comment"
    ).send().await;
    match future {
        Ok(_) => println!("comment sent"),
        Err(e) => return Err(e.into()),
    }
    Ok(())
}