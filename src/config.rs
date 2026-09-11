use anyhow::{Context, Result};
use serde::Deserialize;
use std::path::PathBuf;

#[derive(Debug, Default, Deserialize)]
struct FileConfig {
    api_key: Option<String>,
    organization: Option<String>,
    project: Option<String>,
    #[serde(default)]
    refresh_secs: Option<u64>,
}

#[derive(Debug, Clone)]
pub struct Config {
    pub api_key: String,
    pub organization: Option<String>,
    pub project: Option<String>,
    pub refresh_secs: u64,
}

fn config_path() -> Option<PathBuf> {
    directories::ProjectDirs::from("dev", "omarchy", "omarchy-batch-monitor")
        .map(|dirs| dirs.config_dir().join("config.toml"))
}

impl Config {
    pub fn load(cli_api_key: Option<String>, cli_refresh_secs: Option<u64>) -> Result<Self> {
        let file_cfg = config_path()
            .filter(|p| p.exists())
            .map(|p| -> Result<FileConfig> {
                let text = std::fs::read_to_string(&p)
                    .with_context(|| format!("failed to read config file at {}", p.display()))?;
                toml::from_str(&text)
                    .with_context(|| format!("failed to parse config file at {}", p.display()))
            })
            .transpose()?
            .unwrap_or_default();

        let api_key = cli_api_key
            .or_else(|| std::env::var("OPENAI_API_KEY").ok())
            .or(file_cfg.api_key)
            .context(
                "no OpenAI API key found. Set OPENAI_API_KEY, pass --api-key, or add \
                 api_key to the config file",
            )?;

        let organization = std::env::var("OPENAI_ORG_ID")
            .ok()
            .or(file_cfg.organization);
        let project = std::env::var("OPENAI_PROJECT_ID").ok().or(file_cfg.project);

        let refresh_secs = cli_refresh_secs
            .or(file_cfg.refresh_secs)
            .unwrap_or(15)
            .max(3);

        Ok(Self {
            api_key,
            organization,
            project,
            refresh_secs,
        })
    }

    pub fn example_path() -> String {
        config_path()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|| "~/.config/omarchy-batch-monitor/config.toml".to_string())
    }
}
