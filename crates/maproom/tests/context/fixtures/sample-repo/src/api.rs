//! API handling for the sample application.

use crate::utils::{Config, format_output, validate_config};

/// Process an incoming request
pub fn process_request(config: &Config) -> String {
    if !validate_config(config) {
        return "Error: Invalid config".to_string();
    }

    let data = fetch_data();
    let formatted = format_output(&data);
    formatted
}

/// Fetch data from a source
fn fetch_data() -> String {
    "Sample data".to_string()
}

/// Parse request parameters
pub fn parse_params(query: &str) -> Vec<(String, String)> {
    query
        .split('&')
        .filter_map(|pair| {
            let mut parts = pair.split('=');
            match (parts.next(), parts.next()) {
                (Some(k), Some(v)) => Some((k.to_string(), v.to_string())),
                _ => None,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils::load_config;

    #[test]
    fn test_process_request() {
        let config = load_config();
        let result = process_request(&config);
        assert!(result.contains("Output"));
    }

    #[test]
    fn test_fetch_data() {
        let data = fetch_data();
        assert_eq!(data, "Sample data");
    }

    #[test]
    fn test_parse_params() {
        let params = parse_params("key1=value1&key2=value2");
        assert_eq!(params.len(), 2);
        assert_eq!(params[0].0, "key1");
        assert_eq!(params[0].1, "value1");
    }
}
