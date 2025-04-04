/*
 * Based on https://github.com/vadika/rust-bugreporter
 */

use adw::prelude::*;
use futures::FutureExt;
use http::header::ACCEPT;
use octocrab::{models::issues::Issue, Octocrab};
use secrecy::{ExposeSecret, SecretBox, SecretString};
use serde::{Deserialize, Serialize, Serializer};
use std::path::PathBuf;
use std::sync::Mutex;
use std::{env, time::Duration};
use thiserror::Error as ThisError;

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

fn expose<S>(t: &Option<SecretString>, s: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    s.serialize_str(t.as_ref().unwrap().expose_secret())
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GithubConfig {
    #[serde(skip_serializing_if = "Option::is_none", serialize_with = "expose")]
    pub token: Option<SecretString>,
    pub owner: String,
    pub repo: String,
}

pub static CONFIG: Mutex<Option<GithubConfig>> = Mutex::new(None);

pub async fn auth() -> Result<(), Error> {
    let client_id = std::env::var("GITHUB_CLIENT_ID")?.into();
    let timeout = Duration::from_secs(60);

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
            let dlg = adw::MessageDialog::new(gtk::Window::NONE, Some("Github Login"), None);
            let (cncl_tx, cncl_rx) = async_channel::bounded::<()>(1);
            let _cancel_tx = cancel_tx;
            dlg.set_body(&message);
            dlg.add_response("cancel", "Cancel");
            dlg.set_body_use_markup(true);
            dlg.connect_response(None, move |_dlg, _ers| {
                let _ = cncl_tx.send_blocking(());
            });
            dlg.show();
            futures::select! {
                _ = rx.recv().fuse() => (),
                _ = cncl_rx.recv().fuse() => (),
            };
            dlg.destroy();
        });
    });

    // Atuhentication with timeout

    let auth = tokio::select! {
        e = codes.poll_until_available(&crab, &client_id) => e?,
        _ = tokio::time::sleep(timeout) => Err(Error::TimedOut)?,
        _ = cancel_rx.recv() => Err(Error::Cancelled)?,
    };
    set_key(auth.access_token)?;

    Ok(())
}

pub fn get_config_path() -> PathBuf {
    let variable_name = "GITHUB_CONFIG";
    let variable = env::var_os(variable_name);
    match variable {
        Some(ref val) => PathBuf::from(val),
        None => {
            println!("Missing environment variable: {variable_name}");
            PathBuf::from("/home/ghaf/.config/ctrl-panel/config.toml")
        }
    }
}

pub fn load_config() -> Result<GithubConfig, Error> {
    let path = get_config_path();

    Ok(toml::from_str(&std::fs::read_to_string(path)?)?)
}

pub fn set_config() -> Result<(), Error> {
    let _ = match load_config() {
        Ok(c) => *CONFIG.lock().unwrap() = Some(c),
        Err(e) => return Err(e.into()),
    };
    Ok(())
}

pub fn get_config() -> GithubConfig {
    let config = CONFIG.lock().unwrap();
    config.as_ref().cloned().unwrap()
}

pub async fn create_github_issue(title: String, content: String) -> Result<Issue, Error> {
    let _ = set_config();
    let config = get_config().clone();

    let issue_body = content
        .split_once("\n\nAttachment:")
        .map(|(a, _)| a)
        .unwrap_or(&content);

    match send_issue(&config, &title, &issue_body).await {
        Err(_e) => {
            auth().await?;
            let _ = set_config();
            let config = get_config();
            send_issue(&config, &title, &issue_body).await
        }
        ok => ok,
    }
}

async fn send_issue(config: &GithubConfig, title: &str, body: &str) -> Result<Issue, Error> {
    let octocrab = Octocrab::builder()
        .personal_token(
            config
                .token
                .as_ref()
                .ok_or(Error::NotAuthenticated)?
                .clone(),
        )
        .build()?;
    Ok(octocrab
        .issues(&config.owner, &config.repo)
        .create(title)
        .body(body.to_string())
        .send()
        .await?)
}

#[inline]
fn set_key(token: SecretString) -> Result<(), Error> {
    let mut guard = CONFIG.lock().unwrap();
    guard.as_mut().unwrap().token = Some(token);
    let path = get_config_path();

    std::fs::write(&path, toml::to_string(guard.as_ref().unwrap())?.as_bytes())?;

    Ok(())
}
