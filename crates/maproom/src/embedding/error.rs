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

    /// Generic error
    #[error("Embedding error: {0}")]
    Other(String),
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
