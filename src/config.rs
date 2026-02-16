use std::path::PathBuf;

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

pub struct ConfigPlugin;

impl Plugin for ConfigPlugin {
    fn build(&self, app: &mut App) {
        let settings = GameSettings::load();
        app.insert_resource(settings);
    }
}

#[derive(Resource, Debug, Clone, Serialize, Deserialize)]
pub struct GameSettings {
    pub master_volume: f32,
    pub sfx_volume: f32,
    pub preview_volume: f32,
    pub audio_offset_ms: i32,
    pub visual_offset_ms: i32,
    pub note_speed: f32,
    pub background_dim: f32,
    pub fullscreen: bool,
}

impl Default for GameSettings {
    fn default() -> Self {
        Self {
            master_volume: 80.0,
            sfx_volume: 80.0,
            preview_volume: 50.0,
            audio_offset_ms: 0,
            visual_offset_ms: 0,
            note_speed: 1.0,
            background_dim: 0.0,
            fullscreen: false,
        }
    }
}

impl GameSettings {
    fn config_path() -> Option<PathBuf> {
        let dirs = directories::ProjectDirs::from("", "", "FunkTrack")?;
        Some(dirs.config_dir().join("settings.json"))
    }

    pub fn load() -> Self {
        let Some(path) = Self::config_path() else {
            info!("No config directory available, using defaults");
            return Self::default();
        };

        match std::fs::read_to_string(&path) {
            Ok(contents) => match serde_json::from_str(&contents) {
                Ok(settings) => {
                    info!("Loaded settings from {:?}", path);
                    settings
                }
                Err(e) => {
                    warn!("Failed to parse settings {:?}: {}, using defaults", path, e);
                    Self::default()
                }
            },
            Err(_) => {
                info!("No settings file found, using defaults");
                Self::default()
            }
        }
    }

    pub fn save(&self) {
        let Some(path) = Self::config_path() else {
            warn!("No config directory available, cannot save settings");
            return;
        };

        if let Some(parent) = path.parent() {
            if let Err(e) = std::fs::create_dir_all(parent) {
                warn!("Failed to create config directory {:?}: {}", parent, e);
                return;
            }
        }

        match serde_json::to_string_pretty(self) {
            Ok(json) => {
                if let Err(e) = std::fs::write(&path, json) {
                    warn!("Failed to write settings to {:?}: {}", path, e);
                } else {
                    info!("Saved settings to {:?}", path);
                }
            }
            Err(e) => {
                warn!("Failed to serialize settings: {}", e);
            }
        }
    }

    /// Master volume as a 0.0–1.0 amplitude.
    pub fn master_amplitude(&self) -> f64 {
        (self.master_volume as f64 / 100.0).clamp(0.0, 1.0)
    }

    /// Preview volume as a 0.0–1.0 amplitude (scaled by master).
    pub fn preview_amplitude(&self) -> f64 {
        let preview = (self.preview_volume as f64 / 100.0).clamp(0.0, 1.0);
        preview * self.master_amplitude()
    }
}
