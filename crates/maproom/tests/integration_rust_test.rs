use maproom::indexer::parser;

#[test]
fn test_real_rust_file_parsing() {
    let source = r#"
//! Sample Rust file for testing parser

use std::collections::HashMap;

/// Maximum number of retries
pub const MAX_RETRIES: u32 = 3;

/// Configuration struct for the application
pub struct Config {
    name: String,
    values: HashMap<String, String>,
}

impl Config {
    /// Creates a new Config instance
    pub fn new(name: String) -> Self {
        Config {
            name,
            values: HashMap::new(),
        }
    }

    /// Gets a value from the config
    pub fn get(&self, key: &str) -> Option<&String> {
        self.values.get(key)
    }
}

/// A trait for items that can be configured
pub trait Configurable {
    fn configure(&mut self, config: &Config);
}

impl Configurable for Config {
    fn configure(&mut self, config: &Config) {
        self.name = config.name.clone();
    }
}

/// An enum representing different states
pub enum State {
    Active,
    Inactive,
    Pending,
}

/// Asynchronously fetches data
pub async fn fetch_data(url: &str) -> Result<String, Box<dyn std::error::Error>> {
    Ok(format!("Data from {}", url))
}

/// Simple macro for logging
macro_rules! log {
    ($msg:expr) => {
        println!("[LOG] {}", $msg);
    };
}

mod private_module {
    fn internal_helper() {
        println!("internal");
    }
}
"#;

    let chunks = parser::extract_chunks(source, "rs");

    println!("\n=== Extracted {} chunks ===", chunks.len());
    for chunk in &chunks {
        println!(
            "  - {} ({}) lines {}-{}: {}",
            chunk
                .symbol_name
                .as_ref()
                .unwrap_or(&"<anonymous>".to_string()),
            chunk.kind,
            chunk.start_line,
            chunk.end_line,
            chunk
                .docstring
                .as_ref()
                .map(|d| &d[..d.len().min(50)])
                .unwrap_or("")
        );
    }

    // Verify we extracted the major items
    assert!(chunks
        .iter()
        .any(|c| c.kind == "constant" && c.symbol_name == Some("MAX_RETRIES".to_string())));
    assert!(chunks
        .iter()
        .any(|c| c.kind == "struct" && c.symbol_name == Some("Config".to_string())));
    assert!(chunks
        .iter()
        .any(|c| c.kind == "trait" && c.symbol_name == Some("Configurable".to_string())));
    assert!(chunks
        .iter()
        .any(|c| c.kind == "enum" && c.symbol_name == Some("State".to_string())));
    assert!(chunks
        .iter()
        .any(|c| c.kind == "macro" && c.symbol_name == Some("log".to_string())));
    assert!(chunks
        .iter()
        .any(|c| c.kind == "module" && c.symbol_name == Some("private_module".to_string())));

    // Verify impl blocks
    let impl_blocks: Vec<_> = chunks.iter().filter(|c| c.kind == "impl").collect();
    assert_eq!(impl_blocks.len(), 2, "Should have 2 impl blocks");

    // Verify async function
    let fetch_data = chunks
        .iter()
        .find(|c| c.symbol_name == Some("fetch_data".to_string()))
        .unwrap();
    assert_eq!(fetch_data.kind, "func");
    let metadata = fetch_data.metadata.as_ref().unwrap();
    assert_eq!(metadata["is_async"].as_bool(), Some(true));

    // Verify functions inside impl block
    let impl_functions: Vec<_> = chunks
        .iter()
        .filter(|c| {
            c.kind == "func"
                && (c.symbol_name == Some("new".to_string())
                    || c.symbol_name == Some("get".to_string()))
        })
        .collect();
    assert!(
        impl_functions.len() >= 2,
        "Should have extracted impl functions"
    );

    println!(
        "\n✅ Successfully parsed real Rust file with {} chunks",
        chunks.len()
    );
}
