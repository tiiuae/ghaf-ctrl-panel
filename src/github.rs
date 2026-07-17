/*
 * Based on https://github.com/vadika/rust-bugreporter
 */

use adw::prelude::*;
use futures::FutureExt;
use http::header::ACCEPT;
use octocrab::{Octocrab, models::issues::Issue};
use secrecy::{ExposeSecret, SecretString};
use std::time::Duration;
use thiserror::Error as ThisError;

use crate::app_config::{self, GithubConfig};

#[derive(ThisError, Debug)]
pub enum Error {
    Cancelled,
    TimedOut,
    NotAuthenticated,
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Octocrab(#[from] octocrab::Error),
    #[error(transparent)]
    Var(#[from] std::env::VarError),
    #[error(transparent)]
    Channel(#[from] async_channel::RecvError),
    #[error(transparent)]
    AppConfig(#[from] app_config::Error),
    #[error(transparent)]
    TomlDe(#[from] toml::de::Error),
    #[error(transparent)]
    TomlSer(#[from] toml::ser::Error),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        match self {
            Error::Cancelled => write!(f, ": Authentication cancelled"),
            Error::TimedOut => write!(f, ": Authentication timed out"),
            _ => Ok(()),
        }
    }
}

pub async fn auth(config: &mut GithubConfig) -> Result<(), Error> {
    let client_id = std::env::var("GITHUB_CLIENT_ID")?.into();
    let timeout = Duration::from_mins(1);

    let crab = octocrab::Octocrab::builder()
        .base_uri("https://github.com")?
        .add_header(ACCEPT, "application/json".to_string())
        .build()?;

    let codes = crab
        .authenticate_as_device(&client_id, ["public_repo"])
        .await?;

    // Set message box text
    let message = format!(
        "<a href=\"{0}\">{0}</a>\n{1}",
        codes.verification_uri, codes.user_code
    );

    // rx.recv() will resolve when _tx is dropped
    let (_tx, rx) = async_channel::bounded::<()>(1);
    let (cancel_tx, cancel_rx) = async_channel::bounded::<()>(1);

    // GObjects are not Send + Sync, hence cannot be held across await. First create a future that
    // is run in main thread, and use the local variant from there.
    gtk::glib::spawn_future(async move {
        gtk::glib::spawn_future_local(async move {
            let dlg = adw::AlertDialog::new(Some("Github Login"), None);
            let (cncl_tx, cncl_rx) = async_channel::bounded::<()>(1);
            let _cancel_tx = cancel_tx;
            dlg.set_body(&message);
            dlg.add_response("cancel", "Cancel");
            dlg.set_body_use_markup(true);
            dlg.connect_response(None, move |_dlg, _ers| {
                let _ = cncl_tx.send_blocking(());
            });
            dlg.present(gtk::Window::NONE);
            futures::select! {
                _ = rx.recv().fuse() => (),
                _ = cncl_rx.recv().fuse() => (),
            };
            dlg.force_close();
        });
    });

    // Atuhentication with timeout

    let auth = tokio::select! {
        e = codes.poll_until_available(&crab, &client_id) => e?,
        () = tokio::time::sleep(timeout) => Err(Error::TimedOut)?,
        _ = cancel_rx.recv() => Err(Error::Cancelled)?,
    };
    set_key(config, auth.access_token)?;

    Ok(())
}

pub fn load_config() -> Result<GithubConfig, Error> {
    Ok(app_config::load_config()?.github)
}

pub async fn create_github_issue(title: String, content: String) -> Result<Issue, Error> {
    let mut config = load_config()?;

    let issue_body = content
        .split_once("\n\nAttachment:")
        .map_or(content.as_str(), |(a, _)| a);

    match send_issue(&config, &title, issue_body).await {
        Err(_e) => {
            auth(&mut config).await?;
            send_issue(&config, &title, issue_body).await
        }
        ok => ok,
    }
}

async fn send_issue(config: &GithubConfig, title: &str, body: &str) -> Result<Issue, Error> {
    if config.token.expose_secret().is_empty() {
        return Err(Error::NotAuthenticated);
    }

    let octocrab = Octocrab::builder()
        .personal_token(config.token.clone())
        .build()?;
    Ok(octocrab
        .issues(&config.owner, &config.repo)
        .create(title)
        .body(body.to_string())
        .send()
        .await?)
}

#[inline]
fn set_key(config: &mut GithubConfig, token: SecretString) -> Result<(), Error> {
    config.token = token;
    let mut app_config = app_config::load_config().unwrap_or_default();
    app_config.github = config.clone();
    app_config::save_config(&app_config)?;

    Ok(())
}
