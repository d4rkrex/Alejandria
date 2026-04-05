use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default = "default_db_path")]
    pub db_path: String,

    #[serde(default)]
    pub search: SearchConfig,

    #[serde(default)]
    pub decay: DecayConfig,

    #[serde(default)]
    pub mcp: McpConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchConfig {
    #[serde(default = "default_search_limit")]
    pub limit: usize,

    #[serde(default = "default_min_score")]
    pub min_score: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecayConfig {
    #[serde(default = "default_auto_decay")]
    pub auto_decay: bool,

    #[serde(default = "default_prune_threshold")]
    pub prune_threshold: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpConfig {
    #[serde(default = "default_stdio")]
    pub stdio: bool,

    #[serde(default)]
    pub log_requests: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            db_path: default_db_path(),
            search: SearchConfig::default(),
            decay: DecayConfig::default(),
            mcp: McpConfig::default(),
        }
    }
}

impl Default for SearchConfig {
    fn default() -> Self {
        Self {
            limit: default_search_limit(),
            min_score: default_min_score(),
        }
    }
}

impl Default for DecayConfig {
    fn default() -> Self {
        Self {
            auto_decay: default_auto_decay(),
            prune_threshold: default_prune_threshold(),
        }
    }
}

impl Default for McpConfig {
    fn default() -> Self {
        Self {
            stdio: default_stdio(),
            log_requests: false,
        }
    }
}

fn default_db_path() -> String {
    "~/.local/share/alejandria/alejandria.db".to_string()
}

fn default_search_limit() -> usize {
    10
}

fn default_min_score() -> f32 {
    0.3
}

fn default_auto_decay() -> bool {
    true
}

fn default_prune_threshold() -> f32 {
    0.1
}

fn default_stdio() -> bool {
    true
}

impl Config {
    /// Load configuration with priority: defaults < config file < environment variables
    pub fn load() -> Result<Self> {
        let mut config = Self::default();

        // Try to load from config file
        if let Some(config_path) = Self::config_path() {
            if config_path.exists() {
                let content =
                    std::fs::read_to_string(&config_path).context("Failed to read config file")?;
                config = toml::from_str(&content).context("Failed to parse config file")?;
            }
        }

        // Apply environment variable overrides
        config.apply_env_overrides();

        Ok(config)
    }

    /// Get the config file path (~/.config/alejandria/config.toml)
    pub fn config_path() -> Option<PathBuf> {
        dirs::config_dir().map(|p| p.join("alejandria").join("config.toml"))
    }

    /// Apply environment variable overrides
    fn apply_env_overrides(&mut self) {
        if let Ok(path) = std::env::var("ALEJANDRIA_DB_PATH") {
            self.db_path = path;
        }

        if let Ok(limit) = std::env::var("ALEJANDRIA_SEARCH_LIMIT") {
            if let Ok(limit) = limit.parse() {
                self.search.limit = limit;
            }
        }

        if let Ok(score) = std::env::var("ALEJANDRIA_SEARCH_MIN_SCORE") {
            if let Ok(score) = score.parse() {
                self.search.min_score = score;
            }
        }

        if let Ok(auto) = std::env::var("ALEJANDRIA_DECAY_AUTO_DECAY") {
            if let Ok(auto) = auto.parse() {
                self.decay.auto_decay = auto;
            }
        }

        if let Ok(threshold) = std::env::var("ALEJANDRIA_DECAY_PRUNE_THRESHOLD") {
            if let Ok(threshold) = threshold.parse() {
                self.decay.prune_threshold = threshold;
            }
        }
    }

    /// Expand ~ in paths to home directory
    pub fn expand_db_path(&self) -> Result<PathBuf> {
        let path = if self.db_path.starts_with("~/") {
            if let Some(home) = dirs::home_dir() {
                home.join(&self.db_path[2..])
            } else {
                PathBuf::from(&self.db_path)
            }
        } else {
            PathBuf::from(&self.db_path)
        };

        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).context("Failed to create database directory")?;
        }

        Ok(path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert_eq!(config.search.limit, 10);
        assert_eq!(config.search.min_score, 0.3);
        assert!(config.decay.auto_decay);
        assert_eq!(config.decay.prune_threshold, 0.1);
    }

    #[test]
    fn test_expand_db_path() {
        let config = Config::default();
        let path = config.expand_db_path().unwrap();
        assert!(!path.to_string_lossy().contains("~"));
    }
}
