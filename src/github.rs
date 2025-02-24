/*
 * Based on https://github.com/vadika/rust-bugreporter
 */

use dialog::DialogBox;
use http::header::ACCEPT;
use octocrab::Octocrab;
use secrecy::{ExposeSecret, SecretString};
use serde::Deserialize;
use std::io::Write;
use std::path::PathBuf;
use std::sync::Mutex;
use std::{env, time::Duration};
use toml_edit::{value, DocumentMut};

#[derive(Debug, Deserialize, Clone)]
pub struct GithubConfig {
    pub token: String,
    pub owner: String,
    pub repo: String,
}

pub static CONFIG: Mutex<GithubConfig> = Mutex::new(GithubConfig {
    token: String::new(),
    owner: String::new(),
    repo: String::new(),
});

pub async fn auth() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let client_id = std::env::var("GITHUB_CLIENT_ID")?.to_string();
    let secret_id = SecretString::from(client_id);
    let timeout = Duration::from_secs(60);

    let crab = octocrab::Octocrab::builder()
        .base_uri("https://github.com")?
        .add_header(ACCEPT, "application/json".to_string())
        .build()?;

    let codes = crab
        .authenticate_as_device(&secret_id, ["public_repo"])
        .await?;

    // Set message box text
    let message = format!(
        "{}\n{}",
        codes.verification_uri.clone(),
        codes.user_code.clone()
    );

    let mut backend = dialog::backends::Zenity::new();
    backend.set_width(250);
    dialog::Message::new(message)
        .title("Github Login")
        .show_with(backend)
        .expect("Could not display Github dialog box");

    // Atuhentication with timeout
    let auth = tokio::time::timeout(timeout, codes.poll_until_available(&crab, &secret_id))
        .await?
        .unwrap();
    // Write key to config file
    set_key(auth.access_token.expose_secret()).unwrap();
    Ok(())
}

pub fn get_config_path() -> String {
    let variable_name = "GITHUB_CONFIG";
    let variable = env::var(variable_name);
    let path = match variable {
        Ok(ref val) => val,
        Err(e) => {
            println!("Missing environment variable: {}, {}", variable_name, e);
            "/home/ghaf/.config/ctrl-panel/config.toml"
        }
    };
    path.to_string()
}

pub fn load_config() -> Result<GithubConfig, String> {
    let path = get_config_path();

    let config = match config::Config::builder()
        .add_source(config::File::from(PathBuf::from(path)))
        .build()
    {
        Ok(c) => c,
        Err(_e) => return Err("Failed to load config".to_string()),
    };

    let result = match config.try_deserialize::<GithubConfig>() {
        Ok(r) => r,
        Err(_e) => return Err("Failed to parse config".to_string()),
    };

    Ok(result)
}

pub fn set_config() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let _ = match load_config() {
        Ok(c) => *CONFIG.lock().unwrap() = c,
        Err(e) => return Err(e.into()),
    };
    Ok(())
}

pub fn get_config() -> GithubConfig {
    let config = CONFIG.lock().unwrap();
    config.clone()
}

pub async fn create_github_issue(
    title: &str,
    content: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let _ = set_config();

    let config = get_config().clone();

    let parts: Vec<&str> = content.split("\n\nAttachment:").collect();
    let (issue_body, _attachment_info) = (parts[0], parts.get(1));

    let issue_sent = send_issue(&config, title, issue_body);
    match issue_sent.await {
        Ok(issue) => issue,
        Err(_e) => {
            auth().await?;
            let _ = set_config();
            let config = get_config().clone();
            send_issue(&config, title, issue_body).await?
        }
    };

    Ok(())
}

#[inline]
async fn send_issue(
    config: &GithubConfig,
    title: &str,
    body: &str,
) -> octocrab::Result<octocrab::models::issues::Issue> {
    let octocrab = Octocrab::builder()
        .personal_token(config.token.clone())
        .build()?;
    octocrab
        .issues(&config.owner, &config.repo)
        .create(title)
        .body(body.to_string())
        .send()
        .await
}

#[inline]
fn set_key(token: &str) -> Result<(), std::io::Error> {
    let token_key = String::from("token");
    let path = get_config_path();
    let contents = std::fs::read_to_string(&path)?;
    let mut doc = contents.parse::<DocumentMut>().unwrap();
    doc[&token_key] = value(token);

    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .truncate(true)
        .open(&path)?;
    file.write(doc.to_string().as_bytes())?;

    Ok(())
}
