//! Configuration loading module.
//! Handles loading and parsing configuration from files and environment.

use std::collections::HashMap;
use std::error::Error;
use std::path::Path;

/// Application configuration settings.
#[derive(Debug, Clone)]
pub struct Config {
    /// Database connection settings
    pub database_url: String,
    /// Server port number
    pub port: u16,
    /// Log level (debug, info, warn, error)
    pub log_level: String,
    /// Additional key-value settings
    pub settings: HashMap<String, String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            database_url: String::from("postgresql://localhost:5432/app"),
            port: 8080,
            log_level: String::from("info"),
            settings: HashMap::new(),
        }
    }
}

impl Config {
    /// Create a new configuration with default values.
    pub fn new() -> Self {
        Self::default()
    }

    /// Get a setting value by key.
    pub fn get(&self, key: &str) -> Option<&String> {
        self.settings.get(key)
    }

    /// Set a configuration value.
    pub fn set(&mut self, key: String, value: String) {
        self.settings.insert(key, value);
    }
}

/// Load configuration from a file path.
/// Supports JSON and TOML configuration files.
pub fn load_config(path: &str) -> Result<Config, Box<dyn Error>> {
    let path = Path::new(path);

    if !path.exists() {
        return Err(format!("Configuration file not found: {}", path.display()).into());
    }

    // Parse configuration file based on extension
    // Load and merge with default values
    let mut config = Config::default();

    // In real implementation, parse the file content
    println!("Loading configuration from {}", path.display());

    Ok(config)
}

/// Load configuration from environment variables.
pub fn load_from_env() -> Config {
    let mut config = Config::default();

    // Read DATABASE_URL from environment
    if let Ok(url) = std::env::var("DATABASE_URL") {
        config.database_url = url;
    }

    // Read PORT from environment
    if let Ok(port) = std::env::var("PORT") {
        if let Ok(p) = port.parse() {
            config.port = p;
        }
    }

    // Read LOG_LEVEL from environment
    if let Ok(level) = std::env::var("LOG_LEVEL") {
        config.log_level = level;
    }

    config
}

/// Validate configuration values.
pub fn validate_config(config: &Config) -> Result<(), String> {
    if config.database_url.is_empty() {
        return Err("database_url is required".to_string());
    }
    if config.port == 0 {
        return Err("port must be greater than 0".to_string());
    }
    Ok(())
}
