use std::path::PathBuf;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub database_path: Option<String>,
    pub side_panel_default: bool,
    pub auto_save_seconds: u64,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            database_path: None,
            side_panel_default: true,
            auto_save_seconds: 30,
        }
    }
}

impl Config {
    pub fn data_dir() -> Result<PathBuf> {
        let base = dirs::data_dir().context("could not determine XDG data directory")?;
        let dir = base.join("rusty-notes");
        std::fs::create_dir_all(&dir)?;
        Ok(dir)
    }

    pub fn config_dir() -> Result<PathBuf> {
        let base = dirs::config_dir().context("could not determine XDG config directory")?;
        let dir = base.join("rusty-notes");
        std::fs::create_dir_all(&dir)?;
        Ok(dir)
    }

    pub fn db_path(&self) -> Result<PathBuf> {
        if let Some(ref path) = self.database_path {
            Ok(PathBuf::from(path))
        } else {
            Self::data_dir().map(|d| d.join("notes.db"))
        }
    }

    pub fn load() -> Result<Self> {
        let config_path = Self::config_dir()?.join("config.toml");
        if config_path.exists() {
            let content =
                std::fs::read_to_string(&config_path).context("failed to read config file")?;
            let config: Config = toml::from_str(&content).context("failed to parse config file")?;
            Ok(config)
        } else {
            let config = Self::default();
            config.save()?;
            Ok(config)
        }
    }

    pub fn save(&self) -> Result<()> {
        let config_dir = Self::config_dir()?;
        let content = toml::to_string_pretty(self)?;
        std::fs::write(config_dir.join("config.toml"), content)?;
        Ok(())
    }
}
