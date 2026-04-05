//! Decay profile configuration loading from TOML files.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use crate::error::{IcmError, IcmResult};

/// Decay profile configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecayProfileConfig {
    /// Profile descriptions
    pub profiles: HashMap<String, ProfileSettings>,
}

/// Individual profile settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileSettings {
    /// Human-readable description
    pub description: String,
    /// Algorithm identifier
    pub algorithm: String,
    /// Algorithm-specific parameters
    #[serde(flatten)]
    pub params: HashMap<String, serde_json::Value>,
}

/// Load decay profiles from a TOML configuration file.
///
/// # Arguments
///
/// * `path` - Path to the decay_profiles.toml file
///
/// # Returns
///
/// Parsed configuration with all profiles
///
/// # Examples
///
/// ```ignore
/// use alejandria_core::decay::load_decay_profiles;
///
/// let config = load_decay_profiles("config/decay_profiles.toml")?;
/// ```
pub fn load_decay_profiles<P: AsRef<Path>>(path: P) -> IcmResult<DecayProfileConfig> {
    let content = std::fs::read_to_string(path.as_ref())
        .map_err(|e| IcmError::InvalidInput(format!("Failed to read decay profiles config: {}", e)))?;
    
    let config: DecayProfileConfig = toml::from_str(&content)
        .map_err(|e| IcmError::InvalidInput(format!("Failed to parse decay profiles config: {}", e)))?;
    
    Ok(config)
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_parse_sample_config() {
        let toml_str = r#"
[profiles.exponential]
description = "Standard exponential decay (default)"
algorithm = "exponential"
half_life_days = 30

[profiles.spaced-repetition]
description = "SM-2 spaced repetition for learning"
algorithm = "sm2"
initial_interval_days = 1
initial_ease_factor = 2.5
"#;
        
        let config: DecayProfileConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(config.profiles.len(), 2);
        assert!(config.profiles.contains_key("exponential"));
        assert!(config.profiles.contains_key("spaced-repetition"));
    }
}
