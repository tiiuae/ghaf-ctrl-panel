/*
 * Based on https://github.com/vadika/rust-bugreporter
 */

use base64::Engine;
use chrono::Utc;
use octocrab::Octocrab;
use serde::Deserialize;
use std::env;
use std::path::PathBuf;
use std::sync::OnceLock;

#[derive(Debug, Deserialize, Clone)]
pub struct GithubConfig {
    pub token: String,
    pub owner: String,
    pub repo: String,
}

pub static CONFIG: OnceLock<GithubConfig> = OnceLock::new();

pub fn load_config() -> Result<GithubConfig, String> {
    let variable_name = "GITHUB_CONFIG";
    let variable = env::var(variable_name);
    let path = match variable {
        Ok(ref val) => val,
        Err(e) => {
            println!("Missing environment variable: {}, {}", variable_name, e);
            "/home/ghaf/.config/ctrl-panel/config.toml"
        }
    };

    let config = match config::Config::builder()
        .add_source(config::File::from(PathBuf::from(path)))
        .build()
    {
        Ok(c) => c,
        Err(e) => return Err("Failed to load config".to_string()),
    };

    let result = match config.try_deserialize::<GithubConfig>() {
        Ok(r) => r,
        Err(e) => return Err("Failed to parse config".to_string()),
    };

    Ok(result)
}

pub async fn create_github_issue(
    content: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let conf = match load_config() {
        Ok(c) => CONFIG.set(c),
        Err(e) => return Err(e.into()),
    };

    let settings = CONFIG.get().unwrap();

    let octocrab = Octocrab::builder()
        .personal_token(settings.token.clone())
        .build()?;

    let parts: Vec<&str> = content.split("\n\nAttachment:").collect();
    let (issue_body, attachment_info) = (parts[0], parts.get(1));

    let mut final_body = issue_body.to_string();

    if let Some(attachment_text) = attachment_info {
        if let Some(base64_start) = attachment_text.find("Base64 Data:\n") {
            let base64_data = &attachment_text[base64_start + 12..];
            let image_data =
                base64::engine::general_purpose::STANDARD.decode(base64_data.trim())?;

            let timestamp = Utc::now().timestamp();
            let filename = format!("screenshot_{}.png", timestamp);

            let route = format!(
                "/repos/{}/{}/contents/{}",
                settings.owner, settings.repo, filename
            );

            let encoded_content = base64::engine::general_purpose::STANDARD.encode(&image_data);

            let body = serde_json::json!({
                "message": "Add screenshot for bug report",
                "content": encoded_content
            });

            let response = octocrab._put(route, Some(&body)).await?;

            if response.status().is_success() {
                let bytes = hyper::body::to_bytes(response.into_body()).await?;
                let file_info: serde_json::Value = serde_json::from_slice(&bytes)?;
                if let Some(content) = file_info.get("content") {
                    if let Some(download_url) = content.get("download_url").and_then(|u| u.as_str())
                    {
                        final_body.push_str(&format!("\n\n![Screenshot]({})", download_url));
                    }
                }
            }
        }
    }

    octocrab
        .issues(&settings.owner, &settings.repo)
        .create("New Bug Report")
        .body(&final_body)
        .send()
        .await?;

    Ok(())
}
