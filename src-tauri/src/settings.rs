use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

use crate::utils;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    pub save_path: String,
    pub audio_format: String,
    pub playlist_mode: bool,
}

impl Default for Settings {
    fn default() -> Self {
        let downloads = dirs::download_dir()
            .or_else(|| dirs::home_dir().map(|h| h.join("Downloads")))
            .unwrap_or_else(|| PathBuf::from("."));

        Self {
            save_path: downloads.to_string_lossy().to_string(),
            audio_format: "best".to_string(),
            playlist_mode: false,
        }
    }
}

impl Settings {
    fn path() -> PathBuf {
        utils::app_data_dir().join("settings.json")
    }

    pub fn load() -> Self {
        let path = Self::path();
        match fs::read_to_string(&path) {
            Ok(data) => serde_json::from_str(&data).unwrap_or_default(),
            Err(_) => Self::default(),
        }
    }

    pub fn save(&self) -> Result<(), String> {
        let path = Self::path();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let data = serde_json::to_string_pretty(self).map_err(|e| e.to_string())?;
        fs::write(&path, data).map_err(|e| e.to_string())
    }
}
