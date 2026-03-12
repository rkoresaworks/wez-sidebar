use serde::Deserialize;
use std::{fs, path::PathBuf};

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct AppConfig {
    /// Terminal backend: "wezterm" (default) or "tmux"
    pub backend: String,
    /// Path to terminal CLI binary (auto-detected if empty)
    pub terminal_path: String,
    /// Legacy field: maps to terminal_path for backward compat
    #[serde(default)]
    pub wezterm_path: String,
    pub stale_threshold_mins: i64,
    pub data_dir: String,
    #[serde(default)]
    pub reaper: ReaperConfig,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct ReaperConfig {
    pub enabled: bool,
    pub threshold_hours: i64,
}

impl Default for ReaperConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            threshold_hours: 3,
        }
    }
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            backend: "wezterm".to_string(),
            terminal_path: String::new(),
            wezterm_path: String::new(),
            stale_threshold_mins: 30,
            data_dir: "~/.config/wez-sidebar".to_string(),
            reaper: ReaperConfig::default(),
        }
    }
}

impl AppConfig {
    /// Resolve the effective terminal_path (terminal_path > wezterm_path > auto-detect)
    pub fn effective_terminal_path(&self) -> &str {
        if !self.terminal_path.is_empty() {
            &self.terminal_path
        } else if !self.wezterm_path.is_empty() {
            &self.wezterm_path
        } else {
            ""
        }
    }
}

pub fn expand_tilde(path: &str) -> PathBuf {
    if let Some(rest) = path.strip_prefix("~/") {
        if let Some(home) = dirs::home_dir() {
            return home.join(rest);
        }
    }
    PathBuf::from(path)
}

pub fn load_config() -> AppConfig {
    let config_path = dirs::home_dir()
        .unwrap_or_default()
        .join(".config/wez-sidebar/config.toml");

    match fs::read_to_string(&config_path) {
        Ok(content) => toml::from_str(&content).unwrap_or_default(),
        Err(_) => AppConfig::default(),
    }
}
