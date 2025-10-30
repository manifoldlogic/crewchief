//! Utility functions for the sample application.

use std::collections::HashMap;

/// Configuration structure
pub struct Config {
    pub settings: HashMap<String, String>,
}

/// Load configuration from environment
pub fn load_config() -> Config {
    let mut settings = HashMap::new();
    settings.insert("mode".to_string(), "production".to_string());
    Config { settings }
}

/// Validate configuration
pub fn validate_config(config: &Config) -> bool {
    !config.settings.is_empty()
}

/// Format output string
pub fn format_output(data: &str) -> String {
    format!("Output: {}", data)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_config() {
        let config = load_config();
        assert!(!config.settings.is_empty());
    }

    #[test]
    fn test_validate_config() {
        let config = load_config();
        assert!(validate_config(&config));
    }

    #[test]
    fn test_format_output() {
        let result = format_output("test");
        assert_eq!(result, "Output: test");
    }
}
