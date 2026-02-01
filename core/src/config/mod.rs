//! Configuration Module
//!
//! Unified configuration system for Infinite Probability plugins.
//! Supports global, local, and project-level settings.

use anyhow::Result;
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Configuration hierarchy levels
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigLevel {
    /// Built-in defaults
    Default,
    /// User global (~/.config/infinite-probability/)
    Global,
    /// User local (~/.config/infinite-probability/*.local.toml)
    GlobalLocal,
    /// Project level (.infinite-probability/)
    Project,
    /// Project local (.infinite-probability/*.local.toml)
    ProjectLocal,
    /// Environment variables
    Environment,
}

/// Main configuration structure
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
#[derive(Default)]
pub struct Config {
    pub general: GeneralConfig,
    pub aisp: AispConfig,
    pub llm: LlmConfig,
}


/// General settings
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct GeneralConfig {
    pub log_level: String,
    pub telemetry: bool,
}

impl Default for GeneralConfig {
    fn default() -> Self {
        Self {
            log_level: "info".to_string(),
            telemetry: false,
        }
    }
}

/// AISP conversion settings
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AispConfig {
    /// Default conversion tier
    pub default_tier: String,
    /// Confidence threshold for LLM fallback
    pub confidence_threshold: f64,
    /// Enable LLM fallback
    pub enable_llm_fallback: bool,
    /// Maximum recursion for correction loop
    pub max_correction_attempts: usize,
}

impl Default for AispConfig {
    fn default() -> Self {
        Self {
            default_tier: "auto".to_string(),
            confidence_threshold: 0.8,
            enable_llm_fallback: true,
            max_correction_attempts: 3,
        }
    }
}

/// LLM settings
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct LlmConfig {
    /// Default model for fallback
    pub default_model: String,
    /// Model for simple tasks
    pub simple_model: String,
    /// Model for complex tasks
    pub complex_model: String,
}

impl Default for LlmConfig {
    fn default() -> Self {
        Self {
            default_model: "sonnet".to_string(),
            simple_model: "haiku".to_string(),
            complex_model: "opus".to_string(),
        }
    }
}

impl Config {
    /// Load configuration with full hierarchy
    pub fn load() -> Result<Self> {
        let mut config = Self::default();

        // Load global config
        if let Some(global_path) = Self::global_config_path()
            && global_path.exists() {
                let content = std::fs::read_to_string(&global_path)?;
                let global: Config = toml::from_str(&content)?;
                config.merge(global);
            }

        // Load project config
        let project_path = PathBuf::from(".infinite-probability/config.toml");
        if project_path.exists() {
            let content = std::fs::read_to_string(&project_path)?;
            let project: Config = toml::from_str(&content)?;
            config.merge(project);
        }

        // Apply environment overrides
        config.apply_env();

        Ok(config)
    }

    /// Get global config directory
    pub fn global_config_dir() -> Option<PathBuf> {
        ProjectDirs::from("dev", "epiphytic", "infinite-probability").map(|dirs| dirs.config_dir().to_path_buf())
    }

    /// Get global config file path
    pub fn global_config_path() -> Option<PathBuf> {
        Self::global_config_dir().map(|dir| dir.join("config.toml"))
    }

    /// Merge another config into this one
    fn merge(&mut self, other: Config) {
        // General
        if other.general.log_level != GeneralConfig::default().log_level {
            self.general.log_level = other.general.log_level;
        }
        if other.general.telemetry != GeneralConfig::default().telemetry {
            self.general.telemetry = other.general.telemetry;
        }

        // AISP
        if other.aisp.default_tier != AispConfig::default().default_tier {
            self.aisp.default_tier = other.aisp.default_tier;
        }
        if other.aisp.confidence_threshold != AispConfig::default().confidence_threshold {
            self.aisp.confidence_threshold = other.aisp.confidence_threshold;
        }
        if other.aisp.enable_llm_fallback != AispConfig::default().enable_llm_fallback {
            self.aisp.enable_llm_fallback = other.aisp.enable_llm_fallback;
        }

        // LLM
        if other.llm.default_model != LlmConfig::default().default_model {
            self.llm.default_model = other.llm.default_model;
        }
    }

    /// Apply environment variable overrides
    fn apply_env(&mut self) {
        if let Ok(level) = std::env::var("INFINITE_PROBABILITY_LOG_LEVEL") {
            self.general.log_level = level;
        }
        if let Ok(val) = std::env::var("INFINITE_PROBABILITY_AISP_TIER") {
            self.aisp.default_tier = val;
        }
        if let Ok(val) = std::env::var("INFINITE_PROBABILITY_AISP_CONFIDENCE")
            && let Ok(threshold) = val.parse() {
                self.aisp.confidence_threshold = threshold;
            }
        if let Ok(val) = std::env::var("INFINITE_PROBABILITY_LLM_MODEL") {
            self.llm.default_model = val;
        }
    }

    /// Save configuration to file
    pub fn save(&self, path: &PathBuf) -> Result<()> {
        let content = toml::to_string_pretty(self)?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, content)?;
        Ok(())
    }

    /// Get a configuration value by key path
    pub fn get(&self, key: &str) -> Option<String> {
        match key {
            "general.log_level" => Some(self.general.log_level.clone()),
            "general.telemetry" => Some(self.general.telemetry.to_string()),
            "aisp.default_tier" => Some(self.aisp.default_tier.clone()),
            "aisp.confidence_threshold" => Some(self.aisp.confidence_threshold.to_string()),
            "aisp.enable_llm_fallback" => Some(self.aisp.enable_llm_fallback.to_string()),
            "llm.default_model" => Some(self.llm.default_model.clone()),
            "llm.simple_model" => Some(self.llm.simple_model.clone()),
            "llm.complex_model" => Some(self.llm.complex_model.clone()),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert_eq!(config.general.log_level, "info");
        assert_eq!(config.aisp.confidence_threshold, 0.8);
    }

    #[test]
    fn test_get_config_value() {
        let config = Config::default();
        assert_eq!(config.get("general.log_level"), Some("info".to_string()));
        assert_eq!(config.get("unknown.key"), None);
    }
}
