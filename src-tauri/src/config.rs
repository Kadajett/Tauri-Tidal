use crate::error::{AppError, AppResult};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub client_id: String,
    pub client_secret: String,
    #[serde(default)]
    pub access_token: Option<String>,
    #[serde(default)]
    pub refresh_token: Option<String>,
    #[serde(default)]
    pub expires_at: Option<DateTime<Utc>>,
    #[serde(default)]
    pub user_id: Option<String>,
    #[serde(default)]
    pub display_name: Option<String>,
    #[serde(default = "default_country_code")]
    pub country_code: String,
    #[serde(default = "default_audio_quality")]
    pub audio_quality: String,
    #[serde(default = "default_volume")]
    pub volume: f32,
    #[serde(default)]
    pub muted: bool,
}

fn default_country_code() -> String {
    "US".to_string()
}

fn default_audio_quality() -> String {
    "LOSSLESS".to_string()
}

fn default_volume() -> f32 {
    1.0
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            client_id: "fX2JxdmntZWK0ixT".to_string(),
            client_secret: "1Nn9AfDAjxrgJFJbKNWLeAyKGVGmINuXPPLHVXAvxAg=".to_string(),
            access_token: None,
            refresh_token: None,
            expires_at: None,
            user_id: None,
            display_name: None,
            country_code: default_country_code(),
            audio_quality: default_audio_quality(),
            volume: default_volume(),
            muted: false,
        }
    }
}

impl AppConfig {
    pub fn config_dir() -> AppResult<PathBuf> {
        let home = dirs::home_dir()
            .ok_or_else(|| AppError::Config("Cannot find home directory".into()))?;
        Ok(home.join(".tauritidal"))
    }

    pub fn config_path() -> AppResult<PathBuf> {
        Ok(Self::config_dir()?.join("config.json"))
    }

    pub fn load() -> AppResult<Self> {
        let path = Self::config_path()?;
        if !path.exists() {
            return Err(AppError::Config(
                "Config file not found. Please run setup.".into(),
            ));
        }
        let content = std::fs::read_to_string(&path)?;
        let config: Self = serde_json::from_str(&content)?;
        Ok(config)
    }

    pub fn save(&self) -> AppResult<()> {
        let dir = Self::config_dir()?;
        std::fs::create_dir_all(&dir)?;
        let path = Self::config_path()?;
        let content = serde_json::to_string_pretty(self)?;
        std::fs::write(&path, content)?;
        Ok(())
    }

    pub fn queue_path() -> AppResult<PathBuf> {
        Ok(Self::config_dir()?.join("queue.json"))
    }

    pub fn is_authenticated(&self) -> bool {
        self.access_token.is_some()
    }

    pub fn is_token_expired(&self) -> bool {
        match self.expires_at {
            Some(expires) => Utc::now() >= expires,
            None => true,
        }
    }
}
