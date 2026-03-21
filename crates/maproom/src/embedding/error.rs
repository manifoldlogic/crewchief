//! Error types for the embedding service.

use thiserror::Error;

/// Main error type for embedding operations.
#[derive(Error, Debug)]
pub enum EmbeddingError {
    /// API-related errors (OpenAI, Cohere, etc.)
    #[error("API error: {0}")]
    Api(#[from] ApiError),

    /// Cache-related errors
    #[error("Cache error: {0}")]
    Cache(#[from] CacheError),

    /// Configuration-related errors
    #[error("Configuration error: {0}")]
    Config(#[from] ConfigError),

    /// Network/HTTP errors
    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),

    /// JSON parsing errors
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    /// Invalid input
    #[error("Invalid input: {0}")]
    InvalidInput(String),

    /// Dimension mismatch error with configuration context
    #[error("{0}")]
    DimensionMismatch(DimensionMismatchError),

    /// Generic error
    #[error("Embedding error: {0}")]
    Other(String),
}

/// Detailed dimension mismatch error with configuration context.
#[derive(Debug)]
pub struct DimensionMismatchError {
    /// Expected dimension (from configuration)
    pub expected: usize,
    /// Actual dimension (from API response)
    pub actual: usize,
    /// Provider name
    pub provider: String,
    /// Model name
    pub model: String,
    /// Configured dimension
    pub configured_dimension: usize,
}

impl DimensionMismatchError {
    /// Create a new dimension mismatch error.
    pub fn new(
        expected: usize,
        actual: usize,
        provider: String,
        model: String,
        configured_dimension: usize,
    ) -> Self {
        Self {
            expected,
            actual,
            provider,
            model,
            configured_dimension,
        }
    }
}

impl std::fmt::Display for DimensionMismatchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let inferred_provider = infer_provider_from_dimension(self.actual);

        write!(
            f,
            "Dimension mismatch: expected {} dimensions but got {}.\n\n\
             Current configuration:\n\
             - Provider: {}\n\
             - Model: {}\n\
             - Dimension: {}\n\n\
             This usually means the embedding provider configuration doesn't match the actual provider.\n\n\
             Solutions:\n\
             1. Set provider explicitly:\n\
                export MAPROOM_EMBEDDING_PROVIDER={}\n\
             2. Set dimension to match your model:\n\
                export MAPROOM_EMBEDDING_DIMENSION={}\n\
             3. Skip embeddings (the default):\n\
                maproom scan  (without --generate-embeddings)\n\n\
             See troubleshooting guide: .crewchief/claude-code-plugins/plugins/maproom/skills/maproom-search/references/troubleshooting.md",
            self.expected,
            self.actual,
            self.provider,
            self.model,
            self.configured_dimension,
            inferred_provider,
            self.actual
        )
    }
}

impl std::error::Error for DimensionMismatchError {}

/// Infer the most likely provider from embedding dimension.
///
/// This helps users understand what provider their API is actually using
/// based on the dimension returned.
fn infer_provider_from_dimension(dim: usize) -> &'static str {
    match dim {
        768 => "ollama  # or set MAPROOM_EMBEDDING_MODEL=nomic-embed-text",
        1024 => "ollama  # or set MAPROOM_EMBEDDING_MODEL=mxbai-embed-large",
        1536 => "openai",
        _ => "unknown  # custom dimension, set both MAPROOM_EMBEDDING_PROVIDER and MAPROOM_EMBEDDING_DIMENSION",
    }
}

/// API-specific errors with retryability information.
#[derive(Error, Debug)]
pub enum ApiError {
    /// Rate limit exceeded (HTTP 429) - retryable
    #[error("Rate limit exceeded, retry after {retry_after_ms}ms")]
    RateLimit { retry_after_ms: u64 },

    /// Server error (HTTP 500+) - retryable
    #[error("Server error (status {status}): {message}")]
    ServerError { status: u16, message: String },

    /// Authentication error (HTTP 401) - not retryable
    #[error("Authentication failed: {0}")]
    Authentication(String),

    /// Bad request (HTTP 400) - not retryable
    #[error("Bad request: {0}")]
    BadRequest(String),

    /// API quota exceeded - not retryable
    #[error("API quota exceeded: {0}")]
    QuotaExceeded(String),

    /// Model not available
    #[error("Model not available: {0}")]
    ModelUnavailable(String),

    /// Invalid API response
    #[error("Invalid response: {0}")]
    InvalidResponse(String),
}

impl ApiError {
    /// Check if this error is retryable.
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            ApiError::RateLimit { .. } | ApiError::ServerError { .. }
        )
    }

    /// Get retry delay in milliseconds if applicable.
    pub fn retry_delay_ms(&self) -> Option<u64> {
        match self {
            ApiError::RateLimit { retry_after_ms } => Some(*retry_after_ms),
            ApiError::ServerError { .. } => Some(1000), // Default 1s delay
            _ => None,
        }
    }
}

/// Cache-related errors.
#[derive(Error, Debug)]
pub enum CacheError {
    /// Cache write failed
    #[error("Failed to write to cache: {0}")]
    WriteFailed(String),

    /// Cache read failed
    #[error("Failed to read from cache: {0}")]
    ReadFailed(String),

    /// Cache corruption detected
    #[error("Cache corruption detected: {0}")]
    Corruption(String),

    /// Cache lock error
    #[error("Failed to acquire cache lock: {0}")]
    LockError(String),
}

/// Configuration-related errors.
#[derive(Error, Debug)]
pub enum ConfigError {
    /// Missing required configuration
    #[error("Missing required configuration: {0}")]
    MissingConfig(String),

    /// Invalid configuration value
    #[error("Invalid configuration value for {field}: {reason}")]
    InvalidValue { field: String, reason: String },

    /// Environment variable not found
    #[error("Environment variable not found: {0}")]
    EnvVarNotFound(String),

    /// Configuration file error
    #[error("Configuration file error: {0}")]
    FileError(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dimension_mismatch_error_message() {
        // Test that dimension mismatch error includes all required elements
        let error = DimensionMismatchError::new(
            1536,
            1024,
            "OpenAI".to_string(),
            "text-embedding-3-small".to_string(),
            1536,
        );

        let error_msg = format!("{}", error);

        // Verify message contains expected elements from acceptance criteria
        assert!(
            error_msg.contains("expected 1536"),
            "Error should contain expected dimension"
        );
        assert!(
            error_msg.contains("got 1024"),
            "Error should contain actual dimension"
        );
        assert!(
            error_msg.contains("Provider: OpenAI"),
            "Error should contain provider name"
        );
        assert!(
            error_msg.contains("Model: text-embedding-3-small"),
            "Error should contain model name"
        );
        assert!(
            error_msg.contains("Dimension: 1536"),
            "Error should contain configured dimension"
        );
        assert!(
            error_msg.contains("MAPROOM_EMBEDDING_PROVIDER"),
            "Error should suggest MAPROOM_EMBEDDING_PROVIDER env var"
        );
        assert!(
            error_msg.contains("MAPROOM_EMBEDDING_DIMENSION"),
            "Error should suggest MAPROOM_EMBEDDING_DIMENSION env var"
        );
        assert!(
            error_msg.contains("without --generate-embeddings"),
            "Error should suggest skipping embeddings"
        );
        assert!(
            error_msg.contains("Solutions:"),
            "Error should have Solutions section"
        );

        // Test provider inference for common dimensions
        assert!(
            error_msg.contains("ollama"),
            "Error should infer ollama for 1024 dimensions"
        );
    }

    #[test]
    fn test_dimension_mismatch_inference_768() {
        // Test inference for 768-dim models (nomic-embed-text)
        let error = DimensionMismatchError::new(
            1536,
            768,
            "OpenAI".to_string(),
            "text-embedding-3-small".to_string(),
            1536,
        );

        let error_msg = format!("{}", error);
        assert!(
            error_msg.contains("ollama"),
            "Error should infer ollama for 768 dimensions"
        );
        assert!(
            error_msg.contains("nomic-embed-text"),
            "Error should suggest nomic-embed-text for 768 dimensions"
        );
    }

    #[test]
    fn test_dimension_mismatch_inference_1536() {
        // Test inference for OpenAI dimension
        let error = DimensionMismatchError::new(
            1024,
            1536,
            "Ollama".to_string(),
            "mxbai-embed-large".to_string(),
            1024,
        );

        let error_msg = format!("{}", error);
        assert!(
            error_msg.contains("openai"),
            "Error should infer openai for 1536 dimensions"
        );
    }

    #[test]
    fn test_dimension_mismatch_unknown_dimension() {
        // Test handling of unknown/custom dimensions
        let error = DimensionMismatchError::new(
            1024,
            512,
            "Custom".to_string(),
            "custom-model".to_string(),
            1024,
        );

        let error_msg = format!("{}", error);
        assert!(
            error_msg.contains("unknown"),
            "Error should indicate unknown provider for custom dimensions"
        );
        assert!(
            error_msg.contains("custom dimension"),
            "Error should mention custom dimension"
        );
    }

    #[test]
    fn test_api_error_retryability() {
        let rate_limit = ApiError::RateLimit {
            retry_after_ms: 1000,
        };
        assert!(rate_limit.is_retryable());
        assert_eq!(rate_limit.retry_delay_ms(), Some(1000));

        let server_error = ApiError::ServerError {
            status: 500,
            message: "Internal server error".to_string(),
        };
        assert!(server_error.is_retryable());
        assert_eq!(server_error.retry_delay_ms(), Some(1000));

        let auth_error = ApiError::Authentication("Invalid key".to_string());
        assert!(!auth_error.is_retryable());
        assert_eq!(auth_error.retry_delay_ms(), None);

        let bad_request = ApiError::BadRequest("Invalid input".to_string());
        assert!(!bad_request.is_retryable());
        assert_eq!(bad_request.retry_delay_ms(), None);
    }

    #[test]
    fn test_error_conversions() {
        let api_err = ApiError::RateLimit {
            retry_after_ms: 2000,
        };
        let embedding_err: EmbeddingError = api_err.into();
        assert!(matches!(embedding_err, EmbeddingError::Api(_)));

        let cache_err = CacheError::WriteFailed("disk full".to_string());
        let embedding_err: EmbeddingError = cache_err.into();
        assert!(matches!(embedding_err, EmbeddingError::Cache(_)));

        let config_err = ConfigError::MissingConfig("api_key".to_string());
        let embedding_err: EmbeddingError = config_err.into();
        assert!(matches!(embedding_err, EmbeddingError::Config(_)));
    }

    #[test]
    fn test_error_display() {
        let err = ApiError::RateLimit {
            retry_after_ms: 5000,
        };
        assert_eq!(err.to_string(), "Rate limit exceeded, retry after 5000ms");

        let err = ConfigError::MissingConfig("OPENAI_API_KEY".to_string());
        assert_eq!(
            err.to_string(),
            "Missing required configuration: OPENAI_API_KEY"
        );

        let err = CacheError::LockError("timeout".to_string());
        assert_eq!(err.to_string(), "Failed to acquire cache lock: timeout");
    }
}
