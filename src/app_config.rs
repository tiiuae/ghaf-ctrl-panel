use secrecy::{ExposeSecret, SecretString};
use serde::{Deserialize, Serialize};
use std::env;
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    TomlDe(#[from] toml::de::Error),
    #[error(transparent)]
    TomlSer(#[from] toml::ser::Error),
}

#[derive(Debug, Default, Serialize, Deserialize, Clone)]
pub struct AppConfig {
    #[serde(default)]
    pub github: GithubConfig,
    #[serde(default)]
    pub update: UpdateServerConfig,
}

#[derive(Debug, Default, Serialize, Deserialize, Clone)]
pub struct GithubConfig {
    #[serde(default)]
    #[serde(serialize_with = "serialize_secret_string")]
    pub token: SecretString,
    #[serde(default)]
    pub owner: String,
    #[serde(default)]
    pub repo: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct UpdateServerConfig {
    #[serde(default = "default_update_auth_mode")]
    pub auth_mode: String,
    #[serde(default)]
    pub reference: String,
    #[serde(default)]
    pub insecure: bool,
    #[serde(default)]
    pub username: String,
    #[serde(default)]
    #[serde(serialize_with = "serialize_secret_string")]
    pub password: SecretString,
    #[serde(default)]
    #[serde(serialize_with = "serialize_secret_string")]
    pub oauth_token: SecretString,
}

impl Default for UpdateServerConfig {
    fn default() -> Self {
        Self {
            auth_mode: default_update_auth_mode(),
            reference: String::new(),
            insecure: false,
            username: String::new(),
            password: SecretString::from(String::new()),
            oauth_token: SecretString::from(String::new()),
        }
    }
}

#[derive(Debug, Deserialize)]
struct LegacyGithubConfig {
    token: String,
    owner: String,
    repo: String,
}

fn default_update_auth_mode() -> String {
    "user-pass".to_string()
}

fn serialize_secret_string<S>(value: &SecretString, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    serializer.serialize_str(value.expose_secret())
}

pub fn get_config_path() -> PathBuf {
    if let Some(path) = env::var_os("CTRL_PANEL_CONFIG") {
        return PathBuf::from(path);
    }

    if let Some(path) = env::var_os("GITHUB_CONFIG") {
        return PathBuf::from(path);
    }

    Path::new(&env::var_os("HOME").unwrap_or_else(|| "/home/ghaf".into()))
        .join(".config/ctrl-panel/config.toml")
}

pub fn load_config() -> Result<AppConfig, Error> {
    let path = get_config_path();
    let raw = std::fs::read_to_string(&path)?;

    match toml::from_str::<AppConfig>(&raw) {
        Ok(config) => Ok(config),
        Err(new_format_err) => match toml::from_str::<LegacyGithubConfig>(&raw) {
            Ok(legacy) => Ok(AppConfig {
                github: GithubConfig {
                    token: SecretString::from(legacy.token),
                    owner: legacy.owner,
                    repo: legacy.repo,
                },
                ..Default::default()
            }),
            Err(_) => Err(new_format_err.into()),
        },
    }
}

pub fn save_config(config: &AppConfig) -> Result<(), Error> {
    let path = get_config_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&path, toml::to_string(config)?.as_bytes())?;
    Ok(())
}
