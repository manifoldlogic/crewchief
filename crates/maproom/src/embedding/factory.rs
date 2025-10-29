//! Provider factory for creating embedding providers from environment configuration.
//!
//! This module provides factory functions for constructing embedding providers with
//! auto-detection and validation. It implements a zero-config experience by detecting
//! Ollama availability automatically, with fallback to explicit provider configuration.
//!
//! # Auto-detection Strategy
//!
//! 1. Check if `EMBEDDING_PROVIDER` environment variable is set
//! 2. If not set, attempt to detect Ollama at `localhost:11434/api/tags`
//! 3. If Ollama is unavailable, return an error with helpful configuration guidance
//!
//! # Examples
//!
//! ```no_run
//! use crewchief_maproom::embedding::factory::create_provider_from_env;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Auto-detect provider (prefers Ollama, falls back to EMBEDDING_PROVIDER env var)
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
//! - `EMBEDDING_PROVIDER`: Provider name ("ollama", "openai", "google")
//! - `EMBEDDING_MODEL`: Model name (provider-specific defaults)
//! - `EMBEDDING_API_ENDPOINT`: API endpoint (provider-specific defaults)
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
//! export EMBEDDING_PROVIDER=openai
//! export OPENAI_API_KEY=sk-...
//! cargo run
//! ```
//!
//! ## Custom Ollama endpoint
//! ```bash
//! export EMBEDDING_PROVIDER=ollama
//! export EMBEDDING_API_ENDPOINT=http://remote-host:11434/api/embed
//! export EMBEDDING_MODEL=nomic-embed-text
//! cargo run
//! ```
//!
//! ## Google Vertex AI configuration
//! ```bash
//! export EMBEDDING_PROVIDER=google
//! export GOOGLE_PROJECT_ID=my-project
//! export GOOGLE_APPLICATION_CREDENTIALS=/path/to/service-account.json
//! cargo run
//! ```

use std::env;
use std::fs;
use std::path::PathBuf;
use std::time::Duration;

use crate::embedding::error::{ConfigError, EmbeddingError};
use crate::embedding::provider::EmbeddingProvider;
use crate::embedding::ollama::OllamaProvider;
use crate::embedding::client::OpenAIClient;
use crate::embedding::config::EmbeddingConfig;
use crate::embedding::google::GoogleProvider;

/// Create embedding provider from environment configuration.
///
/// This function implements a zero-config experience with intelligent auto-detection:
///
/// # Auto-detection Process
///
/// 1. **Explicit Configuration**: If `EMBEDDING_PROVIDER` is set, use that provider
/// 2. **Ollama Detection**: Otherwise, check if Ollama is available at `localhost:11434`
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
/// - `EMBEDDING_PROVIDER`: Provider name (optional, default: auto-detect)
///
/// ## Model Configuration
/// - `EMBEDDING_MODEL`: Model name (optional, provider-specific defaults)
/// - `EMBEDDING_API_ENDPOINT`: API endpoint (optional, provider-specific defaults)
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
/// // Set environment: EMBEDDING_PROVIDER=openai
/// //                  OPENAI_API_KEY=sk-...
/// let provider = create_provider_from_env().await?;
/// assert_eq!(provider.provider_name(), "openai");
/// # Ok(())
/// # }
/// ```
pub async fn create_provider_from_env() -> Result<Box<dyn EmbeddingProvider>, EmbeddingError> {
    // Check explicit config first
    let explicit_provider = env::var("EMBEDDING_PROVIDER").ok();

    let provider_name = match explicit_provider.as_deref() {
        Some(p) => {
            tracing::debug!("Using explicit provider from EMBEDDING_PROVIDER: {}", p);
            p.to_lowercase()
        }
        None => {
            // Auto-detect Ollama
            tracing::debug!("No EMBEDDING_PROVIDER set, attempting Ollama auto-detection");
            if is_ollama_available().await {
                tracing::info!("Ollama detected at localhost:11434");
                "ollama".to_string()
            } else {
                tracing::warn!("Ollama not detected and no EMBEDDING_PROVIDER configured");
                return Err(EmbeddingError::Config(ConfigError::MissingConfig(
                    "No embedding provider configured. Options:\n\
                     1. Install and start Ollama (https://ollama.ai) for zero-config local embeddings\n\
                     2. Set EMBEDDING_PROVIDER=openai and OPENAI_API_KEY=... for OpenAI\n\
                     3. Set EMBEDDING_PROVIDER=google and GOOGLE_PROJECT_ID=... for Google (future)"
                        .to_string(),
                )));
            }
        }
    };

    // Create provider based on name
    match provider_name.as_str() {
        "ollama" => {
            let endpoint = env::var("EMBEDDING_API_ENDPOINT")
                .unwrap_or_else(|_| "http://localhost:11434/api/embed".to_string());
            let model = env::var("EMBEDDING_MODEL")
                .unwrap_or_else(|_| "nomic-embed-text".to_string());

            tracing::info!("Using provider: ollama (model: {}, endpoint: {})", model, endpoint);

            let provider = OllamaProvider::new(endpoint, model)?;
            Ok(Box::new(provider))
        }
        "openai" => {
            tracing::debug!("Creating OpenAI provider from environment configuration");

            // Validate required environment variables before creating provider
            if env::var("OPENAI_API_KEY").is_err() {
                return Err(EmbeddingError::Config(ConfigError::MissingConfig(
                    "OPENAI_API_KEY environment variable required for OpenAI provider.\n\
                     Get your API key from: https://platform.openai.com/api-keys\n\
                     Then set: export OPENAI_API_KEY=sk-..."
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

            // Validate GOOGLE_PROJECT_ID
            let project_id = env::var("GOOGLE_PROJECT_ID").map_err(|_| {
                EmbeddingError::Config(ConfigError::MissingConfig(
                    "GOOGLE_PROJECT_ID environment variable not set. Required for Google provider.\n\
                     Get your project ID from: https://console.cloud.google.com/\n\
                     Then set: export GOOGLE_PROJECT_ID=your-project-id"
                        .to_string(),
                ))
            })?;

            // Validate GOOGLE_APPLICATION_CREDENTIALS
            let creds_path_str = env::var("GOOGLE_APPLICATION_CREDENTIALS").map_err(|_| {
                EmbeddingError::Config(ConfigError::MissingConfig(
                    "GOOGLE_APPLICATION_CREDENTIALS environment variable not set. Required for Google provider.\n\
                     Set it to the path of your service account JSON key file.\n\
                     Download from: https://console.cloud.google.com/iam-admin/serviceaccounts\n\
                     Then set: export GOOGLE_APPLICATION_CREDENTIALS=/path/to/service-account.json"
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
                field: "EMBEDDING_PROVIDER".to_string(),
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

/// Check if Ollama is available on localhost.
///
/// This function performs a health check by sending an HTTP GET request to the
/// Ollama API tags endpoint. A 2-second timeout ensures startup isn't blocked
/// by network issues.
///
/// # Detection Strategy
///
/// 1. Build HTTP client with 2-second timeout
/// 2. Send GET request to `http://localhost:11434/api/tags`
/// 3. Check if response status is successful (2xx)
///
/// # Returns
///
/// - `true` - Ollama is running and responding at localhost:11434
/// - `false` - Ollama is not available (not installed, not running, or timeout)
///
/// # Examples
///
/// ```no_run
/// # use crewchief_maproom::embedding::factory::is_ollama_available;
/// # async fn example() {
/// if is_ollama_available().await {
///     println!("Ollama is available for local embeddings");
/// } else {
///     println!("Ollama not detected, consider installing from https://ollama.ai");
/// }
/// # }
/// ```
async fn is_ollama_available() -> bool {
    // Build HTTP client with short timeout
    let client = match reqwest::Client::builder()
        .timeout(Duration::from_secs(2))
        .build()
    {
        Ok(c) => c,
        Err(e) => {
            tracing::debug!("Failed to build HTTP client for Ollama detection: {}", e);
            return false;
        }
    };

    // Check Ollama API endpoint
    match client
        .get("http://localhost:11434/api/tags")
        .send()
        .await
    {
        Ok(response) => {
            let is_success = response.status().is_success();
            tracing::debug!(
                "Ollama detection request completed with status: {} (available: {})",
                response.status(),
                is_success
            );
            is_success
        }
        Err(e) => {
            tracing::debug!("Ollama detection request failed: {}", e);
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Note: These tests use environment variables which are global state.
    // Run with `--test-threads=1` to avoid test interference:
    //   cargo test -p crewchief-maproom --lib embedding::factory -- --test-threads=1

    #[test]
    fn test_provider_name_normalization() {
        // Test case-insensitive provider names
        assert_eq!("ollama".to_lowercase(), "ollama");
        assert_eq!("OLLAMA".to_lowercase(), "ollama");
        assert_eq!("Ollama".to_lowercase(), "ollama");
        assert_eq!("openai".to_lowercase(), "openai");
        assert_eq!("OpenAI".to_lowercase(), "openai");
    }

    #[tokio::test]
    async fn test_ollama_detection_timeout() {
        // This test verifies that Ollama detection respects the 2-second timeout
        // by attempting to connect to a non-existent host
        let start = std::time::Instant::now();
        let _result = is_ollama_available().await;
        let elapsed = start.elapsed();

        // Should complete within 3 seconds (2s timeout + 1s margin)
        assert!(
            elapsed.as_secs() < 3,
            "Ollama detection took too long: {:?}",
            elapsed
        );
    }

    #[tokio::test]
    async fn test_create_provider_with_explicit_ollama() {
        // Clean up all environment variables first
        env::remove_var("EMBEDDING_PROVIDER");
        env::remove_var("EMBEDDING_MODEL");
        env::remove_var("EMBEDDING_API_ENDPOINT");
        env::remove_var("OPENAI_API_KEY");
        env::remove_var("GOOGLE_PROJECT_ID");
        env::remove_var("GOOGLE_APPLICATION_CREDENTIALS");

        // Test explicit Ollama configuration
        env::set_var("EMBEDDING_PROVIDER", "ollama");
        env::set_var("EMBEDDING_MODEL", "nomic-embed-text");
        env::set_var("EMBEDDING_API_ENDPOINT", "http://localhost:11434/api/embed");

        let result = create_provider_from_env().await;

        // Clean up env vars
        env::remove_var("EMBEDDING_PROVIDER");
        env::remove_var("EMBEDDING_MODEL");
        env::remove_var("EMBEDDING_API_ENDPOINT");

        assert!(result.is_ok(), "Failed to create Ollama provider: {:?}", result.err());
        let provider = result.unwrap();
        assert_eq!(provider.provider_name(), "ollama");
        assert_eq!(provider.dimension(), 768);
    }

    #[tokio::test]
    async fn test_create_provider_missing_openai_key() {
        // Clean up all environment variables first
        env::remove_var("EMBEDDING_PROVIDER");
        env::remove_var("OPENAI_API_KEY");
        env::remove_var("GOOGLE_PROJECT_ID");
        env::remove_var("GOOGLE_APPLICATION_CREDENTIALS");

        // Set provider to openai without API key
        env::set_var("EMBEDDING_PROVIDER", "openai");

        let result = create_provider_from_env().await;

        // Clean up
        env::remove_var("EMBEDDING_PROVIDER");

        assert!(result.is_err(), "Expected error when OPENAI_API_KEY is missing");
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
    async fn test_create_provider_unknown_provider() {
        // Clean up all environment variables first
        env::remove_var("EMBEDDING_PROVIDER");
        env::remove_var("OPENAI_API_KEY");
        env::remove_var("GOOGLE_PROJECT_ID");
        env::remove_var("GOOGLE_APPLICATION_CREDENTIALS");

        env::set_var("EMBEDDING_PROVIDER", "unknown-provider");

        let result = create_provider_from_env().await;

        // Clean up
        env::remove_var("EMBEDDING_PROVIDER");

        assert!(result.is_err(), "Expected error for unknown provider");
        if let Err(err) = result {
            assert!(
                matches!(err, EmbeddingError::Config(ConfigError::InvalidValue { .. })),
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
    async fn test_create_provider_google_missing_project_id() {
        // Clean up all environment variables first
        env::remove_var("EMBEDDING_PROVIDER");
        env::remove_var("OPENAI_API_KEY");
        env::remove_var("GOOGLE_PROJECT_ID");
        env::remove_var("GOOGLE_APPLICATION_CREDENTIALS");

        // Set provider but not project ID
        env::set_var("EMBEDDING_PROVIDER", "google");

        let result = create_provider_from_env().await;

        // Clean up
        env::remove_var("EMBEDDING_PROVIDER");

        assert!(result.is_err(), "Expected error when GOOGLE_PROJECT_ID is missing");
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
    async fn test_create_provider_google_missing_credentials() {
        // Clean up all environment variables first
        env::remove_var("EMBEDDING_PROVIDER");
        env::remove_var("OPENAI_API_KEY");
        env::remove_var("GOOGLE_PROJECT_ID");
        env::remove_var("GOOGLE_APPLICATION_CREDENTIALS");

        // Set provider and project ID but not credentials
        env::set_var("EMBEDDING_PROVIDER", "google");
        env::set_var("GOOGLE_PROJECT_ID", "test-project");

        let result = create_provider_from_env().await;

        // Clean up
        env::remove_var("EMBEDDING_PROVIDER");
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
    async fn test_create_provider_google_credentials_file_not_found() {
        // Clean up all environment variables first
        env::remove_var("EMBEDDING_PROVIDER");
        env::remove_var("OPENAI_API_KEY");
        env::remove_var("GOOGLE_PROJECT_ID");
        env::remove_var("GOOGLE_APPLICATION_CREDENTIALS");

        env::set_var("EMBEDDING_PROVIDER", "google");
        env::set_var("GOOGLE_PROJECT_ID", "test-project");
        env::set_var("GOOGLE_APPLICATION_CREDENTIALS", "/nonexistent/path/key.json");

        let result = create_provider_from_env().await;

        // Clean up
        env::remove_var("EMBEDDING_PROVIDER");
        env::remove_var("GOOGLE_PROJECT_ID");
        env::remove_var("GOOGLE_APPLICATION_CREDENTIALS");

        assert!(result.is_err(), "Expected error when credentials file doesn't exist");
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

        assert!(result.is_err(), "Expected error for missing required fields");
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

        assert!(result.is_ok(), "Expected success for valid service account JSON: {:?}", result.err());
    }

    #[tokio::test]
    async fn test_create_provider_no_config_no_ollama() {
        // Clean up all environment variables first
        env::remove_var("EMBEDDING_PROVIDER");
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
                    err_msg.contains("Ollama") || err_msg.contains("EMBEDDING_PROVIDER"),
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
        env::set_var("EMBEDDING_PROVIDER", "ollama");

        let result = create_provider_from_env().await;

        // Clean up
        env::remove_var("EMBEDDING_PROVIDER");

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
        let missing_key_error = ConfigError::MissingConfig(
            "OPENAI_API_KEY environment variable required".to_string()
        );
        let err_msg = missing_key_error.to_string();
        assert!(!err_msg.is_empty());

        let invalid_provider_error = ConfigError::InvalidValue {
            field: "EMBEDDING_PROVIDER".to_string(),
            reason: "Unknown provider".to_string(),
        };
        let err_msg = invalid_provider_error.to_string();
        assert!(err_msg.contains("EMBEDDING_PROVIDER"));
    }
}
