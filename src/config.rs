use crate::model::mapping::CursorMapping;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Clone, Debug)]
pub struct Config {
    pub input_dir: PathBuf,
    pub output_dir: PathBuf,
    pub mapping: CursorMapping,
    pub thread_count: usize,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            input_dir: PathBuf::from("."),
            output_dir: PathBuf::from("./out"),
            mapping: CursorMapping::default(),
            thread_count: 0,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct AppConfig {
    pub theme: Option<String>,
    pub cursor_theme: Option<String>,
    pub cursor_size: Option<u32>,
}

impl AppConfig {
    pub fn config_path() -> Option<PathBuf> {
        dirs::config_dir().map(|p| p.join("ani2hyprtui").join("config.toml"))
    }

    pub fn load() -> Self {
        Self::config_path()
            .and_then(|path| fs::read_to_string(path).ok())
            .and_then(|content| toml::from_str(&content).ok())
            .unwrap_or_default()
    }

    pub fn save(&self) -> Result<(), std::io::Error> {
        if let Some(path) = Self::config_path() {
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)?;
            }
            let content = toml::to_string_pretty(self)
                .map_err(std::io::Error::other)?;
            fs::write(path, content)?;
        }
        Ok(())
    }

    pub fn save_theme(theme_name: &str) {
        let mut config = Self::load();
        config.theme = Some(theme_name.to_string());
        let _ = config.save();
    }

    pub fn save_cursor(theme_name: &str, size: u32) {
        let mut config = Self::load();
        config.cursor_theme = Some(theme_name.to_string());
        config.cursor_size = Some(size);
        let _ = config.save();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_app_config_serialize_deserialize() {
        let cfg = AppConfig {
            theme: Some("Tokyo Night".to_string()),
            cursor_theme: Some("Koosh-Hyprcursor2".to_string()),
            cursor_size: Some(64),
        };
        let toml_str = toml::to_string_pretty(&cfg).unwrap();
        let loaded: AppConfig = toml::from_str(&toml_str).unwrap();
        assert_eq!(cfg, loaded);
    }
}
