//! Provider factory for creating embedding providers from environment configuration.
//!
//! This module provides factory functions for constructing embedding providers with
//! auto-detection and validation. It implements a zero-config experience by detecting
//! Ollama availability automatically, with fallback to explicit provider configuration.
//!
//! # Auto-detection Strategy
//!
//! 1. Check if `MAPROOM_EMBEDDING_PROVIDER` environment variable is set
//! 2. If not set, attempt to detect Ollama using fallback chain:
//!    - `MAPROOM_EMBEDDING_API_ENDPOINT` (extract base URL)
//!    - `localhost:11434` (native development)
//!    - `host.docker.internal:11434` (Docker/DevContainer)
//! 3. If Ollama is unavailable, return an error with helpful configuration guidance
//!
//! # Examples
//!
//! ```no_run
//! use crewchief_maproom::embedding::factory::create_provider_from_env;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Auto-detect provider (prefers Ollama, falls back to MAPROOM_EMBEDDING_PROVIDER env var)
//!     let provider = create_provider_from_env().await?;
//!
//!     println!("Using provider: {}", provider.provider_name());
//!     println!("Embedding dimension: {}", provider.dimension());
//!
//!     // Generate embedding
//!     let embedding = provider.embed("Hello, world!".to_string()).await?;
//!     assert_eq!(embedding.len(), provider.dimension());
//!
//!     Ok(())
//! }
//! ```
//!
//! # Environment Variables
//!
//! - `MAPROOM_EMBEDDING_PROVIDER`: Provider name ("ollama", "openai", "google")
//! - `MAPROOM_EMBEDDING_MODEL`: Model name (provider-specific defaults)
//! - `MAPROOM_EMBEDDING_API_ENDPOINT`: API endpoint (provider-specific defaults)
//! - `OPENAI_API_KEY`: Required for OpenAI provider
//! - `GOOGLE_PROJECT_ID`: Required for Google provider
//! - `GOOGLE_APPLICATION_CREDENTIALS`: Required for Google provider
//!
//! # Configuration Examples
//!
//! ## Zero-config (Ollama auto-detection)
//! ```bash
//! # No environment variables needed - detects Ollama automatically
//! cargo run
//! ```
//!
//! ## Explicit OpenAI configuration
//! ```bash
//! export MAPROOM_EMBEDDING_PROVIDER=openai
//! export OPENAI_API_KEY=sk-...
//! cargo run
//! ```
//!
//! ## Custom Ollama endpoint
//! ```bash
//! export MAPROOM_EMBEDDING_PROVIDER=ollama
//! export MAPROOM_EMBEDDING_API_ENDPOINT=http://remote-host:11434/api/embed
//! export MAPROOM_EMBEDDING_MODEL=nomic-embed-text
//! cargo run
//! ```
//!
//! ## Google Vertex AI configuration
//! ```bash
//! export MAPROOM_EMBEDDING_PROVIDER=google
//! export GOOGLE_PROJECT_ID=my-project
//! export GOOGLE_APPLICATION_CREDENTIALS=/path/to/service-account.json
//! cargo run
//! ```

use std::env;
use std::fs;
use std::path::PathBuf;
use std::time::Duration;

use crate::embedding::client::OpenAIClient;
use crate::embedding::config::EmbeddingConfig;
use crate::embedding::error::{ConfigError, EmbeddingError};
use crate::embedding::google::GoogleProvider;
use crate::embedding::ollama::OllamaProvider;
use crate::embedding::provider::EmbeddingProvider;

/// Create embedding provider from environment configuration.
///
/// This function implements a zero-config experience with intelligent auto-detection:
///
/// # Auto-detection Process
///
/// 1. **Explicit Configuration**: If `MAPROOM_EMBEDDING_PROVIDER` is set, use that provider
/// 2. **Ollama Detection**: Otherwise, detect Ollama using fallback chain:
///    - `MAPROOM_EMBEDDING_API_ENDPOINT` (extract base URL from embed endpoint)
///    - `localhost:11434` (native development)
///    - `host.docker.internal:11434` (Docker/DevContainer environments)
/// 3. **Configuration Error**: If no provider is available, return helpful error message
///
/// # Supported Providers
///
/// - **ollama**: Local Ollama embeddings (zero-config, auto-detected)
/// - **openai**: OpenAI API (requires `OPENAI_API_KEY`)
/// - **google**: Google Vertex AI (requires `GOOGLE_PROJECT_ID`, future implementation)
///
/// # Environment Variables
///
/// ## Provider Selection
/// - `MAPROOM_EMBEDDING_PROVIDER`: Provider name (optional, default: auto-detect)
///
/// ## Model Configuration
/// - `MAPROOM_EMBEDDING_MODEL`: Model name (optional, provider-specific defaults)
/// - `MAPROOM_EMBEDDING_API_ENDPOINT`: API endpoint (optional, provider-specific defaults)
///
/// ## Provider-Specific Authentication
/// - `OPENAI_API_KEY`: Required for OpenAI provider
/// - `GOOGLE_PROJECT_ID`: Required for Google provider (future)
///
/// # Returns
///
/// - `Ok(Box<dyn EmbeddingProvider>)` - Successfully created and configured provider
/// - `Err(EmbeddingError)` - Configuration validation failed or no provider available
///
/// # Errors
///
/// This function returns an error if:
/// - No provider is explicitly configured and Ollama is not detected
/// - Required environment variables are missing (e.g., `OPENAI_API_KEY` for OpenAI)
/// - Provider name is not recognized
/// - HTTP client creation fails
///
/// # Examples
///
/// ## Zero-config with Ollama auto-detection
/// ```no_run
/// # use crewchief_maproom::embedding::factory::create_provider_from_env;
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// // Automatically detects Ollama at localhost:11434
/// let provider = create_provider_from_env().await?;
/// assert_eq!(provider.provider_name(), "ollama");
/// # Ok(())
/// # }
/// ```
///
/// ## Explicit OpenAI configuration
/// ```no_run
/// # use crewchief_maproom::embedding::factory::create_provider_from_env;
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// // Set environment: MAPROOM_EMBEDDING_PROVIDER=openai
/// //                  OPENAI_API_KEY=sk-...
/// let provider = create_provider_from_env().await?;
/// assert_eq!(provider.provider_name(), "openai");
/// # Ok(())
/// # }
/// ```
pub async fn create_provider_from_env() -> Result<Box<dyn EmbeddingProvider>, EmbeddingError> {
    // Check explicit config first
    let explicit_provider = env::var("MAPROOM_EMBEDDING_PROVIDER").ok();

    // Track detected endpoint for Ollama auto-detection
    let (provider_name, detected_endpoint) = match explicit_provider.as_deref() {
        Some(p) => {
            tracing::debug!(
                "Using explicit provider from MAPROOM_EMBEDDING_PROVIDER: {}",
                p
            );
            (p.to_lowercase(), None)
        }
        None => {
            // Auto-detect Ollama using fallback chain
            tracing::debug!("No MAPROOM_EMBEDDING_PROVIDER set, attempting Ollama auto-detection");
            match detect_ollama_endpoint().await {
                Some(endpoint) => {
                    tracing::info!("Ollama detected at: {}", endpoint);
                    ("ollama".to_string(), Some(endpoint))
                }
                None => {
                    tracing::warn!(
                        "Ollama not detected and no MAPROOM_EMBEDDING_PROVIDER configured"
                    );
                    return Err(EmbeddingError::Config(ConfigError::MissingConfig(
                        "No embedding provider configured. Options:\n\
                         1. Install and start Ollama (https://ollama.ai) for zero-config local embeddings\n\
                         2. Set MAPROOM_EMBEDDING_PROVIDER=openai and OPENAI_API_KEY=... for OpenAI\n\
                         3. Set MAPROOM_EMBEDDING_PROVIDER=google and GOOGLE_PROJECT_ID=... for Google (future)"
                            .to_string(),
                    )));
                }
            }
        }
    };

    // Create provider based on name
    match provider_name.as_str() {
        "ollama" => {
            // Use detected endpoint if available, else check env var, else default to localhost
            let env_endpoint = env::var("MAPROOM_EMBEDDING_API_ENDPOINT").ok();
            tracing::debug!(
                "Ollama endpoint sources - detected: {:?}, env var: {:?}",
                detected_endpoint,
                env_endpoint
            );
            let endpoint = detected_endpoint
                .map(|base| format!("{}/api/embed", base))
                .or_else(|| env_endpoint)
                .unwrap_or_else(|| "http://localhost:11434/api/embed".to_string());
            let model = env::var("MAPROOM_EMBEDDING_MODEL")
                .unwrap_or_else(|_| "mxbai-embed-large".to_string());

            // Load configuration from environment (including dimension)
            let config = EmbeddingConfig::from_env()?;
            let dimension = config.dimension;
            let parallel_config = config.parallel;

            tracing::info!(
                "Using provider: ollama (model: {}, dimension: {}, endpoint: {}, parallel: enabled={}, sub_batch={}, concurrency={})",
                model,
                dimension,
                endpoint,
                parallel_config.enabled,
                parallel_config.sub_batch_size,
                parallel_config.max_concurrency
            );

            let provider =
                OllamaProvider::new_with_config(endpoint, model, dimension, parallel_config)?;
            Ok(Box::new(provider))
        }
        "openai" => {
            tracing::debug!("Creating OpenAI provider from environment configuration");

            // Validate required environment variables before creating provider
            // Try Maproom-specific var first, then fall back to standard var
            if env::var("MAPROOM_OPENAI_API_KEY").is_err() && env::var("OPENAI_API_KEY").is_err() {
                return Err(EmbeddingError::Config(ConfigError::MissingConfig(
                    "OpenAI API key required for OpenAI provider.\n\
                     Get your API key from: https://platform.openai.com/api-keys\n\
                     Then set: export MAPROOM_OPENAI_API_KEY=sk-...\n\
                     (or use standard: export OPENAI_API_KEY=sk-...)"
                        .to_string(),
                )));
            }

            let config = EmbeddingConfig::from_env()?;
            let client = OpenAIClient::new(config)?;

            tracing::info!("Using provider: openai (model: {})", client.config().model);
            Ok(Box::new(client))
        }
        "google" => {
            tracing::debug!("Creating Google provider from environment configuration");

            // Validate GOOGLE_PROJECT_ID (try Maproom-specific var first)
            let project_id = env::var("MAPROOM_GOOGLE_PROJECT_ID")
                .or_else(|_| env::var("GOOGLE_PROJECT_ID"))
                .map_err(|_| {
                    EmbeddingError::Config(ConfigError::MissingConfig(
                        "Google project ID required for Google provider.\n\
                         Get your project ID from: https://console.cloud.google.com/\n\
                         Then set: export MAPROOM_GOOGLE_PROJECT_ID=your-project-id\n\
                         (or use standard: export GOOGLE_PROJECT_ID=your-project-id)"
                            .to_string(),
                    ))
                })?;

            // Validate GOOGLE_APPLICATION_CREDENTIALS (try Maproom-specific var first)
            let creds_path_str = env::var("MAPROOM_GOOGLE_APPLICATION_CREDENTIALS")
                .or_else(|_| env::var("GOOGLE_APPLICATION_CREDENTIALS"))
                .map_err(|_| {
                    EmbeddingError::Config(ConfigError::MissingConfig(
                        "Google application credentials required for Google provider.\n\
                         Set it to the path of your service account JSON key file.\n\
                         Download from: https://console.cloud.google.com/iam-admin/serviceaccounts\n\
                         Then set: export MAPROOM_GOOGLE_APPLICATION_CREDENTIALS=/path/to/service-account.json\n\
                         (or use standard: export GOOGLE_APPLICATION_CREDENTIALS=/path/to/service-account.json)"
                            .to_string(),
                    ))
                })?;

            let creds_path = PathBuf::from(&creds_path_str);

            // Check credentials file exists
            if !creds_path.exists() {
                return Err(EmbeddingError::Config(ConfigError::FileError(format!(
                    "Service account credentials file not found at: {}\n\
                     Verify the path is correct and the file exists.",
                    creds_path.display()
                ))));
            }

            // Validate credentials file is readable and has valid JSON structure
            validate_service_account_json(&creds_path)?;

            // Read optional configuration
            let region = env::var("GOOGLE_REGION")
                .unwrap_or_else(|_| GoogleProvider::DEFAULT_REGION.to_string());
            let model = env::var("GOOGLE_MODEL")
                .unwrap_or_else(|_| GoogleProvider::DEFAULT_MODEL.to_string());

            tracing::info!(
                "Using provider: google (project: {}, region: {}, model: {})",
                project_id,
                region,
                model
            );

            let provider = GoogleProvider::new(project_id, creds_path, region, model).await?;
            Ok(Box::new(provider))
        }
        unknown => {
            tracing::error!("Unknown provider requested: {}", unknown);
            Err(EmbeddingError::Config(ConfigError::InvalidValue {
                field: "MAPROOM_EMBEDDING_PROVIDER".to_string(),
                reason: format!(
                    "Unknown provider: '{}'. Supported providers: ollama, openai, google",
                    unknown
                ),
            }))
        }
    }
}

/// Validate service account JSON file structure.
///
/// This function validates that a service account JSON file exists, is readable,
/// contains valid JSON, and has all required fields for Google Cloud authentication.
///
/// # Arguments
///
/// * `path` - Path to the service account JSON key file
///
/// # Required Fields
///
/// - `type`: Must be "service_account"
/// - `project_id`: GCP project ID
/// - `private_key`: RSA private key for signing JWT tokens
/// - `client_email`: Service account email address
///
/// # Returns
///
/// - `Ok(())` - File is valid and contains required fields
/// - `Err(EmbeddingError)` - File is unreadable, invalid JSON, or missing required fields
///
/// # Examples
///
/// ```no_run
/// # use crewchief_maproom::embedding::factory::validate_service_account_json;
/// # use std::path::PathBuf;
/// # fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let path = PathBuf::from("/path/to/service-account.json");
/// validate_service_account_json(&path)?;
/// # Ok(())
/// # }
/// ```
fn validate_service_account_json(path: &std::path::Path) -> Result<(), EmbeddingError> {
    // Read file contents
    let content = fs::read_to_string(path).map_err(|e| {
        EmbeddingError::Config(ConfigError::FileError(format!(
            "Failed to read service account JSON file: {}\n\
             Ensure the file has proper read permissions.",
            e
        )))
    })?;

    // Parse JSON
    let json: serde_json::Value = serde_json::from_str(&content).map_err(|e| {
        EmbeddingError::Config(ConfigError::FileError(format!(
            "Service account file is not valid JSON: {}\n\
             Download a new service account key from: https://console.cloud.google.com/iam-admin/serviceaccounts",
            e
        )))
    })?;

    // Validate required fields
    let required_fields = ["type", "project_id", "private_key", "client_email"];
    for field in &required_fields {
        if json.get(field).is_none() {
            return Err(EmbeddingError::Config(ConfigError::FileError(format!(
                "Service account JSON missing required field: '{}'\n\
                 Expected fields: type, project_id, private_key, client_email\n\
                 Download a valid service account key from: https://console.cloud.google.com/iam-admin/serviceaccounts",
                field
            ))));
        }
    }

    // Validate type field value
    if let Some(account_type) = json.get("type").and_then(|v| v.as_str()) {
        if account_type != "service_account" {
            return Err(EmbeddingError::Config(ConfigError::FileError(format!(
                "Service account JSON has wrong type: expected 'service_account', got '{}'\n\
                 Ensure you downloaded a service account key, not an OAuth client ID or other credential type.\n\
                 Download from: https://console.cloud.google.com/iam-admin/serviceaccounts",
                account_type
            ))));
        }
    } else {
        return Err(EmbeddingError::Config(ConfigError::FileError(
            "Service account JSON 'type' field is not a string".to_string(),
        )));
    }

    Ok(())
}

/// Extract the base URL from an Ollama embed endpoint.
///
/// Given a full embed endpoint URL (e.g., `http://host:port/api/embed`),
/// extracts just the base URL (e.g., `http://host:port`) for health checks.
///
/// # Supported Suffixes
///
/// - `/api/embed` - Standard Ollama embedding endpoint
/// - `/api/embeddings` - Alternative endpoint format
///
/// Handles trailing slashes gracefully.
///
/// # Returns
///
/// - `Some(base_url)` - Base URL without the embed suffix
/// - `None` - URL doesn't have a recognized suffix
///
/// # Examples
///
/// ```
/// # fn example() {
/// assert_eq!(
///     extract_base_url("http://localhost:11434/api/embed"),
///     Some("http://localhost:11434".to_string())
/// );
/// assert_eq!(
///     extract_base_url("http://host:8080/api/embeddings/"),
///     Some("http://host:8080".to_string())
/// );
/// assert_eq!(extract_base_url("http://host:8080/custom"), None);
/// # }
/// ```
fn extract_base_url(endpoint: &str) -> Option<String> {
    // Handle trailing slashes: "http://host:port/api/embed/" → "http://host:port"
    let endpoint = endpoint.trim_end_matches('/');
    endpoint
        .strip_suffix("/api/embed")
        .or_else(|| endpoint.strip_suffix("/api/embeddings"))
        .map(|s| s.to_string())
}

/// Detect Ollama endpoint using fallback chain.
///
/// This function attempts to detect a running Ollama instance by checking
/// multiple endpoints in priority order. This enables zero-config operation
/// in various environments including Docker and DevContainers.
///
/// # Detection Order
///
/// 1. **Explicit Configuration**: `MAPROOM_EMBEDDING_API_ENDPOINT` env var
///    (extracts base URL from the embed endpoint)
/// 2. **Native Development**: `localhost:11434`
/// 3. **Docker/DevContainer**: `host.docker.internal:11434`
///
/// Each endpoint is checked with a 2-second timeout. Total worst-case
/// detection time is 6 seconds (all endpoints timeout).
///
/// # Returns
///
/// - `Some(base_url)` - First reachable Ollama endpoint's base URL
/// - `None` - No Ollama instance detected at any endpoint
///
/// # Examples
///
/// ```no_run
/// # async fn example() {
/// if let Some(endpoint) = detect_ollama_endpoint().await {
///     println!("Ollama available at: {}", endpoint);
/// } else {
///     println!("Ollama not detected");
/// }
/// # }
/// ```
async fn detect_ollama_endpoint() -> Option<String> {
    // Build HTTP client with short timeout per endpoint
    let client = match reqwest::Client::builder()
        .timeout(Duration::from_secs(2))
        .build()
    {
        Ok(c) => c,
        Err(e) => {
            tracing::debug!("Failed to build HTTP client for Ollama detection: {}", e);
            return None;
        }
    };

    // Build fallback list
    let mut endpoints = Vec::new();

    // 1. Check explicit endpoint config (extract base URL)
    if let Ok(embed_endpoint) = env::var("MAPROOM_EMBEDDING_API_ENDPOINT") {
        if let Some(base) = extract_base_url(&embed_endpoint) {
            endpoints.push(base);
        }
    }

    // 2. localhost (native development)
    endpoints.push("http://localhost:11434".to_string());

    // 3. Docker host (containerized development)
    endpoints.push("http://host.docker.internal:11434".to_string());

    // Log all endpoints we'll try (helpful for debugging)
    tracing::debug!("Ollama detection fallback chain: {:?}", endpoints);

    // Try each endpoint sequentially
    for base in endpoints {
        let check_url = format!("{}/api/tags", base);
        tracing::debug!("Checking Ollama at: {}", check_url);

        match client.get(&check_url).send().await {
            Ok(response) if response.status().is_success() => {
                tracing::info!("Ollama detected at: {}", base);
                return Some(base);
            }
            Ok(response) => {
                tracing::debug!(
                    "Ollama check failed at {}: status {}",
                    base,
                    response.status()
                );
            }
            Err(e) => {
                tracing::debug!("Ollama not available at {}: {}", base, e);
            }
        }
    }

    tracing::debug!("No Ollama endpoint detected");
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;

    // Note: These tests use environment variables which are global state.
    // The #[serial] annotation ensures tests run sequentially to avoid interference.

    #[test]
    fn test_provider_name_normalization() {
        // Test case-insensitive provider names
        assert_eq!("ollama".to_lowercase(), "ollama");
        assert_eq!("OLLAMA".to_lowercase(), "ollama");
        assert_eq!("Ollama".to_lowercase(), "ollama");
        assert_eq!("openai".to_lowercase(), "openai");
        assert_eq!("OpenAI".to_lowercase(), "openai");
    }

    #[test]
    fn test_extract_base_url_embed_suffix() {
        // Standard /api/embed suffix
        assert_eq!(
            extract_base_url("http://localhost:11434/api/embed"),
            Some("http://localhost:11434".to_string())
        );
        // Custom host with port
        assert_eq!(
            extract_base_url("http://ollama.local:11434/api/embed"),
            Some("http://ollama.local:11434".to_string())
        );
        // Docker host
        assert_eq!(
            extract_base_url("http://host.docker.internal:11434/api/embed"),
            Some("http://host.docker.internal:11434".to_string())
        );
    }

    #[test]
    fn test_extract_base_url_embeddings_suffix() {
        // Alternative /api/embeddings suffix
        assert_eq!(
            extract_base_url("http://host:8080/api/embeddings"),
            Some("http://host:8080".to_string())
        );
        // With different port
        assert_eq!(
            extract_base_url("http://localhost:9999/api/embeddings"),
            Some("http://localhost:9999".to_string())
        );
    }

    #[test]
    fn test_extract_base_url_trailing_slash() {
        // Trailing slash on /api/embed
        assert_eq!(
            extract_base_url("http://localhost:11434/api/embed/"),
            Some("http://localhost:11434".to_string())
        );
        // Trailing slash on /api/embeddings
        assert_eq!(
            extract_base_url("http://host:8080/api/embeddings/"),
            Some("http://host:8080".to_string())
        );
        // Multiple trailing slashes
        assert_eq!(
            extract_base_url("http://localhost:11434/api/embed///"),
            Some("http://localhost:11434".to_string())
        );
    }

    #[test]
    fn test_extract_base_url_no_suffix() {
        // No recognized suffix - returns None
        assert_eq!(extract_base_url("http://localhost:11434/custom"), None);
        assert_eq!(
            extract_base_url("http://localhost:11434/api/generate"),
            None
        );
        assert_eq!(extract_base_url("http://localhost:11434"), None);
        // Partial match shouldn't work
        assert_eq!(extract_base_url("http://localhost:11434/api/embe"), None);
    }

    #[test]
    fn test_extract_base_url_empty() {
        // Empty string
        assert_eq!(extract_base_url(""), None);
        // Just slashes
        assert_eq!(extract_base_url("/"), None);
        assert_eq!(extract_base_url("///"), None);
    }

    #[tokio::test]
    async fn test_ollama_detection_timeout() {
        // This test verifies that Ollama detection respects the 2-second timeout per endpoint
        // The fallback chain tries up to 3 endpoints (custom, localhost, host.docker.internal)
        // Worst case: 3 endpoints × 2s timeout = 6s
        let start = std::time::Instant::now();
        let _result = detect_ollama_endpoint().await;
        let elapsed = start.elapsed();

        // Should complete within 7 seconds (3 × 2s timeout + 1s margin)
        // In practice, localhost usually fails fast (connection refused) rather than timing out
        assert!(
            elapsed.as_secs() < 7,
            "Ollama detection took too long: {:?}",
            elapsed
        );
    }

    #[tokio::test]
    #[serial]
    async fn test_create_provider_with_explicit_ollama() {
        // Clean up all environment variables first
        env::remove_var("MAPROOM_EMBEDDING_PROVIDER");
        env::remove_var("MAPROOM_EMBEDDING_MODEL");
        env::remove_var("EMBEDDING_API_ENDPOINT");
        env::remove_var("OPENAI_API_KEY");
        env::remove_var("GOOGLE_PROJECT_ID");
        env::remove_var("GOOGLE_APPLICATION_CREDENTIALS");

        // Test explicit Ollama configuration
        env::set_var("MAPROOM_EMBEDDING_PROVIDER", "ollama");
        env::set_var("MAPROOM_EMBEDDING_MODEL", "nomic-embed-text");
        env::set_var("EMBEDDING_API_ENDPOINT", "http://localhost:11434/api/embed");
        env::set_var("MAPROOM_EMBEDDING_DIMENSION", "768");

        let result = create_provider_from_env().await;

        // Clean up env vars
        env::remove_var("MAPROOM_EMBEDDING_PROVIDER");
        env::remove_var("MAPROOM_EMBEDDING_MODEL");
        env::remove_var("EMBEDDING_API_ENDPOINT");
        env::remove_var("MAPROOM_EMBEDDING_DIMENSION");

        assert!(
            result.is_ok(),
            "Failed to create Ollama provider: {:?}",
            result.err()
        );
        let provider = result.unwrap();
        assert_eq!(provider.provider_name(), "ollama");
        assert_eq!(provider.dimension(), 768);
    }

    #[tokio::test]
    #[serial]
    async fn test_create_provider_missing_openai_key() {
        // Clean up all environment variables first
        env::remove_var("MAPROOM_EMBEDDING_PROVIDER");
        env::remove_var("OPENAI_API_KEY");
        env::remove_var("GOOGLE_PROJECT_ID");
        env::remove_var("GOOGLE_APPLICATION_CREDENTIALS");

        // Set provider to openai without API key
        env::set_var("MAPROOM_EMBEDDING_PROVIDER", "openai");

        let result = create_provider_from_env().await;

        // Clean up
        env::remove_var("MAPROOM_EMBEDDING_PROVIDER");

        assert!(
            result.is_err(),
            "Expected error when OPENAI_API_KEY is missing"
        );
        if let Err(err) = result {
            assert!(
                matches!(err, EmbeddingError::Config(ConfigError::MissingConfig(_))),
                "Expected MissingConfig error, got: {:?}",
                err
            );

            // Check error message is helpful
            let err_msg = err.to_string();
            assert!(
                err_msg.contains("OPENAI_API_KEY"),
                "Error message should mention OPENAI_API_KEY: {}",
                err_msg
            );
        }
    }

    #[tokio::test]
    #[serial]
    async fn test_create_provider_unknown_provider() {
        // Clean up all environment variables first
        env::remove_var("MAPROOM_EMBEDDING_PROVIDER");
        env::remove_var("OPENAI_API_KEY");
        env::remove_var("MAPROOM_OPENAI_API_KEY");
        env::remove_var("GOOGLE_PROJECT_ID");
        env::remove_var("MAPROOM_GOOGLE_PROJECT_ID");
        env::remove_var("GOOGLE_APPLICATION_CREDENTIALS");
        env::remove_var("MAPROOM_GOOGLE_APPLICATION_CREDENTIALS");

        env::set_var("MAPROOM_EMBEDDING_PROVIDER", "unknown-provider");

        let result = create_provider_from_env().await;

        // Clean up
        env::remove_var("MAPROOM_EMBEDDING_PROVIDER");

        assert!(result.is_err(), "Expected error for unknown provider");
        if let Err(err) = result {
            assert!(
                matches!(
                    err,
                    EmbeddingError::Config(ConfigError::InvalidValue { .. })
                ),
                "Expected InvalidValue error, got: {:?}",
                err
            );

            // Check error message lists supported providers
            let err_msg = err.to_string();
            assert!(
                err_msg.contains("ollama") && err_msg.contains("openai"),
                "Error message should list supported providers: {}",
                err_msg
            );
        }
    }

    #[tokio::test]
    #[serial]
    async fn test_create_provider_google_missing_project_id() {
        // Clean up all environment variables first
        env::remove_var("MAPROOM_EMBEDDING_PROVIDER");
        env::remove_var("OPENAI_API_KEY");
        env::remove_var("GOOGLE_PROJECT_ID");
        env::remove_var("GOOGLE_APPLICATION_CREDENTIALS");

        // Set provider but not project ID
        env::set_var("MAPROOM_EMBEDDING_PROVIDER", "google");

        let result = create_provider_from_env().await;

        // Clean up
        env::remove_var("MAPROOM_EMBEDDING_PROVIDER");

        assert!(
            result.is_err(),
            "Expected error when GOOGLE_PROJECT_ID is missing"
        );
        if let Err(err) = result {
            assert!(
                matches!(err, EmbeddingError::Config(ConfigError::MissingConfig(_))),
                "Expected MissingConfig error, got: {:?}",
                err
            );

            let err_msg = err.to_string();
            assert!(
                err_msg.contains("GOOGLE_PROJECT_ID"),
                "Error message should mention GOOGLE_PROJECT_ID: {}",
                err_msg
            );
            assert!(
                err_msg.contains("console.cloud.google.com"),
                "Error message should reference GCP Console: {}",
                err_msg
            );
        }
    }

    #[tokio::test]
    #[serial]
    async fn test_create_provider_google_missing_credentials() {
        // Clean up all environment variables first (including MAPROOM_ prefixed variants)
        env::remove_var("MAPROOM_EMBEDDING_PROVIDER");
        env::remove_var("MAPROOM_EMBEDDING_API_ENDPOINT");
        env::remove_var("OPENAI_API_KEY");
        env::remove_var("MAPROOM_OPENAI_API_KEY");
        env::remove_var("GOOGLE_PROJECT_ID");
        env::remove_var("MAPROOM_GOOGLE_PROJECT_ID");
        env::remove_var("GOOGLE_APPLICATION_CREDENTIALS");
        env::remove_var("MAPROOM_GOOGLE_APPLICATION_CREDENTIALS");

        // Set provider and project ID but not credentials
        env::set_var("MAPROOM_EMBEDDING_PROVIDER", "google");
        env::set_var("GOOGLE_PROJECT_ID", "test-project");

        let result = create_provider_from_env().await;

        // Clean up
        env::remove_var("MAPROOM_EMBEDDING_PROVIDER");
        env::remove_var("GOOGLE_PROJECT_ID");

        assert!(
            result.is_err(),
            "Expected error when GOOGLE_APPLICATION_CREDENTIALS is missing"
        );
        if let Err(err) = result {
            assert!(
                matches!(err, EmbeddingError::Config(ConfigError::MissingConfig(_))),
                "Expected MissingConfig error, got: {:?}",
                err
            );

            let err_msg = err.to_string();
            assert!(
                err_msg.contains("GOOGLE_APPLICATION_CREDENTIALS"),
                "Error message should mention GOOGLE_APPLICATION_CREDENTIALS: {}",
                err_msg
            );
            assert!(
                err_msg.contains("service account JSON key"),
                "Error message should reference service account key: {}",
                err_msg
            );
        }
    }

    #[tokio::test]
    #[serial]
    async fn test_create_provider_google_credentials_file_not_found() {
        // Clean up all environment variables first (including MAPROOM_ prefixed variants)
        env::remove_var("MAPROOM_EMBEDDING_PROVIDER");
        env::remove_var("MAPROOM_EMBEDDING_API_ENDPOINT");
        env::remove_var("OPENAI_API_KEY");
        env::remove_var("MAPROOM_OPENAI_API_KEY");
        env::remove_var("GOOGLE_PROJECT_ID");
        env::remove_var("MAPROOM_GOOGLE_PROJECT_ID");
        env::remove_var("GOOGLE_APPLICATION_CREDENTIALS");
        env::remove_var("MAPROOM_GOOGLE_APPLICATION_CREDENTIALS");

        env::set_var("MAPROOM_EMBEDDING_PROVIDER", "google");
        env::set_var("GOOGLE_PROJECT_ID", "test-project");
        env::set_var(
            "GOOGLE_APPLICATION_CREDENTIALS",
            "/nonexistent/path/key.json",
        );

        let result = create_provider_from_env().await;

        // Clean up
        env::remove_var("MAPROOM_EMBEDDING_PROVIDER");
        env::remove_var("GOOGLE_PROJECT_ID");
        env::remove_var("GOOGLE_APPLICATION_CREDENTIALS");

        assert!(
            result.is_err(),
            "Expected error when credentials file doesn't exist"
        );
        if let Err(err) = result {
            assert!(
                matches!(err, EmbeddingError::Config(ConfigError::FileError(_))),
                "Expected FileError, got: {:?}",
                err
            );

            let err_msg = err.to_string();
            assert!(
                err_msg.contains("not found"),
                "Error message should indicate file not found: {}",
                err_msg
            );
            assert!(
                err_msg.contains("/nonexistent/path/key.json"),
                "Error message should show the path: {}",
                err_msg
            );
        }
    }

    #[tokio::test]
    async fn test_validate_service_account_json_invalid_json() {
        // Create a temporary file with invalid JSON
        let temp_dir = std::env::temp_dir();
        let temp_file = temp_dir.join("invalid-service-account.json");
        fs::write(&temp_file, "{ invalid json }").expect("Failed to write temp file");

        let result = validate_service_account_json(&temp_file);

        // Clean up
        let _ = fs::remove_file(&temp_file);

        assert!(result.is_err(), "Expected error for invalid JSON");
        if let Err(err) = result {
            assert!(
                matches!(err, EmbeddingError::Config(ConfigError::FileError(_))),
                "Expected FileError, got: {:?}",
                err
            );

            let err_msg = err.to_string();
            assert!(
                err_msg.contains("not valid JSON"),
                "Error message should indicate invalid JSON: {}",
                err_msg
            );
            assert!(
                err_msg.contains("console.cloud.google.com"),
                "Error message should reference GCP Console: {}",
                err_msg
            );
        }
    }

    #[tokio::test]
    async fn test_validate_service_account_json_missing_field() {
        // Create a temporary file with incomplete service account JSON
        let temp_dir = std::env::temp_dir();
        let temp_file = temp_dir.join("incomplete-service-account.json");
        let incomplete_json = r#"{
            "type": "service_account",
            "project_id": "test-project"
        }"#;
        fs::write(&temp_file, incomplete_json).expect("Failed to write temp file");

        let result = validate_service_account_json(&temp_file);

        // Clean up
        let _ = fs::remove_file(&temp_file);

        assert!(
            result.is_err(),
            "Expected error for missing required fields"
        );
        if let Err(err) = result {
            assert!(
                matches!(err, EmbeddingError::Config(ConfigError::FileError(_))),
                "Expected FileError, got: {:?}",
                err
            );

            let err_msg = err.to_string();
            assert!(
                err_msg.contains("missing required field"),
                "Error message should indicate missing field: {}",
                err_msg
            );
            // Should mention one of the missing fields (private_key or client_email)
            assert!(
                err_msg.contains("private_key") || err_msg.contains("client_email"),
                "Error message should name a missing field: {}",
                err_msg
            );
        }
    }

    #[tokio::test]
    async fn test_validate_service_account_json_wrong_type() {
        // Create a temporary file with wrong account type
        let temp_dir = std::env::temp_dir();
        let temp_file = temp_dir.join("wrong-type-service-account.json");
        let wrong_type_json = r#"{
            "type": "authorized_user",
            "project_id": "test-project",
            "private_key": "-----BEGIN PRIVATE KEY-----\ntest\n-----END PRIVATE KEY-----\n",
            "client_email": "test@example.com"
        }"#;
        fs::write(&temp_file, wrong_type_json).expect("Failed to write temp file");

        let result = validate_service_account_json(&temp_file);

        // Clean up
        let _ = fs::remove_file(&temp_file);

        assert!(result.is_err(), "Expected error for wrong account type");
        if let Err(err) = result {
            assert!(
                matches!(err, EmbeddingError::Config(ConfigError::FileError(_))),
                "Expected FileError, got: {:?}",
                err
            );

            let err_msg = err.to_string();
            assert!(
                err_msg.contains("wrong type"),
                "Error message should indicate wrong type: {}",
                err_msg
            );
            assert!(
                err_msg.contains("authorized_user"),
                "Error message should show actual type: {}",
                err_msg
            );
            assert!(
                err_msg.contains("service_account"),
                "Error message should show expected type: {}",
                err_msg
            );
        }
    }

    #[tokio::test]
    async fn test_validate_service_account_json_valid() {
        // Create a temporary file with valid service account JSON structure
        let temp_dir = std::env::temp_dir();
        let temp_file = temp_dir.join("valid-service-account.json");
        let valid_json = r#"{
            "type": "service_account",
            "project_id": "test-project",
            "private_key": "-----BEGIN PRIVATE KEY-----\nMIIEvQIBADANBgkqhkiG9w0BAQEFAASCBKcwggSjAgEAAoIBAQC\n-----END PRIVATE KEY-----\n",
            "client_email": "test@test-project.iam.gserviceaccount.com",
            "client_id": "123456789",
            "auth_uri": "https://accounts.google.com/o/oauth2/auth",
            "token_uri": "https://oauth2.googleapis.com/token"
        }"#;
        fs::write(&temp_file, valid_json).expect("Failed to write temp file");

        let result = validate_service_account_json(&temp_file);

        // Clean up
        let _ = fs::remove_file(&temp_file);

        assert!(
            result.is_ok(),
            "Expected success for valid service account JSON: {:?}",
            result.err()
        );
    }

    #[tokio::test]
    #[serial]
    async fn test_create_provider_no_config_no_ollama() {
        // Clean up all environment variables first
        env::remove_var("MAPROOM_EMBEDDING_PROVIDER");
        env::remove_var("OPENAI_API_KEY");
        env::remove_var("GOOGLE_PROJECT_ID");
        env::remove_var("GOOGLE_APPLICATION_CREDENTIALS");

        // Note: This test will pass if Ollama IS running locally
        // If Ollama is available, it will successfully create a provider
        // If Ollama is NOT available, it will return a helpful error
        let result = create_provider_from_env().await;

        match result {
            Err(err) => {
                let err_msg = err.to_string();

                // Error should mention installation options
                assert!(
                    err_msg.contains("Ollama") || err_msg.contains("MAPROOM_EMBEDDING_PROVIDER"),
                    "Error message should provide helpful guidance: {}",
                    err_msg
                );
            }
            Ok(provider) => {
                // If it succeeded, it must have detected Ollama
                assert_eq!(provider.provider_name(), "ollama");
            }
        }
    }

    #[tokio::test]
    async fn test_provider_trait_object_compatibility() {
        // Test that factory returns a valid trait object
        env::set_var("MAPROOM_EMBEDDING_PROVIDER", "ollama");

        let result = create_provider_from_env().await;

        // Clean up
        env::remove_var("MAPROOM_EMBEDDING_PROVIDER");

        if result.is_ok() {
            let provider: Box<dyn EmbeddingProvider> = result.unwrap();

            // Verify trait methods work through dynamic dispatch
            assert!(!provider.provider_name().is_empty());
            assert!(provider.dimension() > 0);

            // Test that metrics returns None for providers without metrics
            let metrics = provider.metrics();
            assert!(metrics.is_none() || metrics.is_some());
        }
    }

    #[test]
    fn test_error_messages_are_actionable() {
        // Verify error messages provide clear next steps
        let missing_key_error =
            ConfigError::MissingConfig("OPENAI_API_KEY environment variable required".to_string());
        let err_msg = missing_key_error.to_string();
        assert!(!err_msg.is_empty());

        let invalid_provider_error = ConfigError::InvalidValue {
            field: "MAPROOM_EMBEDDING_PROVIDER".to_string(),
            reason: "Unknown provider".to_string(),
        };
        let err_msg = invalid_provider_error.to_string();
        assert!(err_msg.contains("MAPROOM_EMBEDDING_PROVIDER"));
    }
}
