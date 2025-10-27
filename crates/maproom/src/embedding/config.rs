//! Configuration structures for the embedding service.

use crate::embedding::error::{ConfigError, EmbeddingError};
use serde::{Deserialize, Serialize};
use std::env;

/// Embedding provider type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Provider {
    /// OpenAI embedding API
    OpenAI,
    /// Cohere embedding API
    Cohere,
    /// Ollama embedding API
    Ollama,
    /// Local embedding model
    Local,
}

impl Default for Provider {
    fn default() -> Self {
        Self::OpenAI
    }
}

impl std::str::FromStr for Provider {
    type Err = ConfigError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "openai" => Ok(Self::OpenAI),
            "cohere" => Ok(Self::Cohere),
            "ollama" => Ok(Self::Ollama),
            "local" => Ok(Self::Local),
            _ => Err(ConfigError::InvalidValue {
                field: "provider".to_string(),
                reason: format!("Unknown provider: {}", s),
            }),
        }
    }
}

/// Embedding service configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingConfig {
    /// Provider to use for embeddings
    pub provider: Provider,

    /// Model name (e.g., "text-embedding-3-small")
    pub model: String,

    /// Embedding dimension
    pub dimension: usize,

    /// Cache configuration
    pub cache: CacheConfig,

    /// Batch processing configuration
    pub batch_size: usize,

    /// Retry configuration
    pub retry: RetryConfig,

    /// API key (loaded from environment)
    #[serde(skip)]
    pub api_key: Option<String>,

    /// API endpoint (optional override)
    pub api_endpoint: Option<String>,
}

impl Default for EmbeddingConfig {
    fn default() -> Self {
        Self {
            provider: Provider::OpenAI,
            model: "text-embedding-3-small".to_string(),
            dimension: 1536,
            cache: CacheConfig::default(),
            batch_size: 100,
            retry: RetryConfig::default(),
            api_key: None,
            api_endpoint: None,
        }
    }
}

impl EmbeddingConfig {
    /// Create a new configuration with defaults.
    pub fn new() -> Self {
        Self::default()
    }

    /// Load configuration from environment variables.
    pub fn from_env() -> Result<Self, EmbeddingError> {
        let mut config = Self::default();

        // Load provider
        if let Ok(provider) = env::var("EMBEDDING_PROVIDER") {
            config.provider = provider.parse()?;
        }

        // Load model
        if let Ok(model) = env::var("EMBEDDING_MODEL") {
            config.model = model;
        }

        // Load dimension
        if let Ok(dim) = env::var("EMBEDDING_DIMENSION") {
            config.dimension = dim.parse().map_err(|_| ConfigError::InvalidValue {
                field: "EMBEDDING_DIMENSION".to_string(),
                reason: "Must be a positive integer".to_string(),
            })?;
        }

        // Load cache size
        if let Ok(size) = env::var("EMBEDDING_CACHE_SIZE") {
            config.cache.max_entries = size.parse().map_err(|_| ConfigError::InvalidValue {
                field: "EMBEDDING_CACHE_SIZE".to_string(),
                reason: "Must be a positive integer".to_string(),
            })?;
        }

        // Load cache TTL
        if let Ok(ttl) = env::var("EMBEDDING_CACHE_TTL") {
            config.cache.ttl_seconds = ttl.parse().map_err(|_| ConfigError::InvalidValue {
                field: "EMBEDDING_CACHE_TTL".to_string(),
                reason: "Must be a positive integer".to_string(),
            })?;
        }

        // Load batch size
        if let Ok(batch) = env::var("EMBEDDING_BATCH_SIZE") {
            config.batch_size = batch.parse().map_err(|_| ConfigError::InvalidValue {
                field: "EMBEDDING_BATCH_SIZE".to_string(),
                reason: "Must be a positive integer".to_string(),
            })?;
        }

        // Load retry max attempts
        if let Ok(max_attempts) = env::var("EMBEDDING_RETRY_MAX_ATTEMPTS") {
            config.retry.max_attempts = max_attempts.parse().map_err(|_| {
                ConfigError::InvalidValue {
                    field: "EMBEDDING_RETRY_MAX_ATTEMPTS".to_string(),
                    reason: "Must be a positive integer".to_string(),
                }
            })?;
        }

        // Load API key based on provider
        config.api_key = match config.provider {
            Provider::OpenAI => env::var("OPENAI_API_KEY").ok(),
            Provider::Cohere => env::var("COHERE_API_KEY").ok(),
            Provider::Ollama => None, // Ollama runs locally, no API key needed
            Provider::Local => None, // Local models don't need API keys
        };

        // Load API endpoint override
        config.api_endpoint = env::var("EMBEDDING_API_ENDPOINT").ok();

        Ok(config)
    }

    /// Validate the configuration.
    pub fn validate(&self) -> Result<(), ConfigError> {
        // Check API key for cloud providers
        if matches!(self.provider, Provider::OpenAI | Provider::Cohere) && self.api_key.is_none()
        {
            return Err(ConfigError::MissingConfig(format!(
                "API key for {:?} provider",
                self.provider
            )));
        }

        // Check dimension
        if self.dimension == 0 {
            return Err(ConfigError::InvalidValue {
                field: "dimension".to_string(),
                reason: "Must be greater than 0".to_string(),
            });
        }

        // Validate Ollama-specific model requirements
        if self.provider == Provider::Ollama && self.model == "nomic-embed-text" {
            if self.dimension != 768 {
                return Err(ConfigError::InvalidValue {
                    field: "dimension".to_string(),
                    reason: format!(
                        "Ollama provider with nomic-embed-text requires dimension=768, got {}",
                        self.dimension
                    ),
                });
            }
        }

        // Check batch size
        if self.batch_size == 0 || self.batch_size > 1000 {
            return Err(ConfigError::InvalidValue {
                field: "batch_size".to_string(),
                reason: "Must be between 1 and 1000".to_string(),
            });
        }

        // Validate cache config
        self.cache.validate()?;

        // Validate retry config
        self.retry.validate()?;

        Ok(())
    }

    /// Get the API endpoint URL.
    pub fn api_endpoint_url(&self) -> String {
        if let Some(endpoint) = &self.api_endpoint {
            endpoint.clone()
        } else {
            match self.provider {
                Provider::OpenAI => "https://api.openai.com/v1/embeddings".to_string(),
                Provider::Cohere => "https://api.cohere.ai/v1/embed".to_string(),
                Provider::Ollama => "http://localhost:11434/api/embeddings".to_string(),
                Provider::Local => "http://localhost:8080/embeddings".to_string(),
            }
        }
    }
}

/// Cache configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheConfig {
    /// Maximum number of entries in the cache
    pub max_entries: usize,

    /// Time-to-live for cache entries (seconds)
    pub ttl_seconds: u64,

    /// Enable cache metrics tracking
    pub enable_metrics: bool,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            max_entries: 10_000,
            ttl_seconds: 3600, // 1 hour
            enable_metrics: true,
        }
    }
}

impl CacheConfig {
    /// Validate cache configuration.
    pub fn validate(&self) -> Result<(), ConfigError> {
        if self.max_entries == 0 {
            return Err(ConfigError::InvalidValue {
                field: "cache.max_entries".to_string(),
                reason: "Must be greater than 0".to_string(),
            });
        }

        // TTL of 0 is allowed (means immediate expiration, useful for testing)

        Ok(())
    }
}

/// Retry configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryConfig {
    /// Maximum number of retry attempts
    pub max_attempts: usize,

    /// Initial retry delay (milliseconds)
    pub initial_delay_ms: u64,

    /// Exponential backoff multiplier
    pub backoff_multiplier: f32,

    /// Maximum retry delay (milliseconds)
    pub max_delay_ms: u64,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            initial_delay_ms: 1000, // 1 second
            backoff_multiplier: 2.0,
            max_delay_ms: 60000, // 60 seconds
        }
    }
}

impl RetryConfig {
    /// Validate retry configuration.
    pub fn validate(&self) -> Result<(), ConfigError> {
        if self.max_attempts == 0 {
            return Err(ConfigError::InvalidValue {
                field: "retry.max_attempts".to_string(),
                reason: "Must be greater than 0".to_string(),
            });
        }

        if self.initial_delay_ms == 0 {
            return Err(ConfigError::InvalidValue {
                field: "retry.initial_delay_ms".to_string(),
                reason: "Must be greater than 0".to_string(),
            });
        }

        if self.backoff_multiplier <= 1.0 {
            return Err(ConfigError::InvalidValue {
                field: "retry.backoff_multiplier".to_string(),
                reason: "Must be greater than 1.0".to_string(),
            });
        }

        if self.max_delay_ms < self.initial_delay_ms {
            return Err(ConfigError::InvalidValue {
                field: "retry.max_delay_ms".to_string(),
                reason: "Must be >= initial_delay_ms".to_string(),
            });
        }

        Ok(())
    }

    /// Calculate retry delay for the given attempt number (0-indexed).
    pub fn delay_for_attempt(&self, attempt: usize) -> u64 {
        if attempt == 0 {
            return 0;
        }

        let delay = (self.initial_delay_ms as f32)
            * self.backoff_multiplier.powi((attempt - 1) as i32);
        delay.min(self.max_delay_ms as f32) as u64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provider_parsing() {
        assert_eq!("openai".parse::<Provider>().unwrap(), Provider::OpenAI);
        assert_eq!("cohere".parse::<Provider>().unwrap(), Provider::Cohere);
        assert_eq!("ollama".parse::<Provider>().unwrap(), Provider::Ollama);
        assert_eq!("local".parse::<Provider>().unwrap(), Provider::Local);
        assert_eq!("OpenAI".parse::<Provider>().unwrap(), Provider::OpenAI);
        assert!("unknown".parse::<Provider>().is_err());
    }

    #[test]
    fn test_provider_parsing_case_insensitive() {
        // Test case-insensitive parsing for Ollama
        assert_eq!("ollama".parse::<Provider>().unwrap(), Provider::Ollama);
        assert_eq!("Ollama".parse::<Provider>().unwrap(), Provider::Ollama);
        assert_eq!("OLLAMA".parse::<Provider>().unwrap(), Provider::Ollama);
        assert_eq!("OlLaMa".parse::<Provider>().unwrap(), Provider::Ollama);
    }

    #[test]
    fn test_provider_serialization() {
        // Test serde serialization with rename_all = "lowercase"
        let provider = Provider::Ollama;
        let serialized = serde_json::to_string(&provider).unwrap();
        assert_eq!(serialized, r#""ollama""#);

        let provider = Provider::OpenAI;
        let serialized = serde_json::to_string(&provider).unwrap();
        assert_eq!(serialized, r#""openai""#);

        let provider = Provider::Cohere;
        let serialized = serde_json::to_string(&provider).unwrap();
        assert_eq!(serialized, r#""cohere""#);

        let provider = Provider::Local;
        let serialized = serde_json::to_string(&provider).unwrap();
        assert_eq!(serialized, r#""local""#);
    }

    #[test]
    fn test_provider_deserialization() {
        // Test serde deserialization
        let provider: Provider = serde_json::from_str(r#""ollama""#).unwrap();
        assert_eq!(provider, Provider::Ollama);

        let provider: Provider = serde_json::from_str(r#""openai""#).unwrap();
        assert_eq!(provider, Provider::OpenAI);

        let provider: Provider = serde_json::from_str(r#""cohere""#).unwrap();
        assert_eq!(provider, Provider::Cohere);

        let provider: Provider = serde_json::from_str(r#""local""#).unwrap();
        assert_eq!(provider, Provider::Local);

        // Invalid provider should fail
        assert!(serde_json::from_str::<Provider>(r#""invalid""#).is_err());
    }

    #[test]
    fn test_default_config() {
        let config = EmbeddingConfig::default();
        assert_eq!(config.provider, Provider::OpenAI);
        assert_eq!(config.model, "text-embedding-3-small");
        assert_eq!(config.dimension, 1536);
        assert_eq!(config.batch_size, 100);
        assert_eq!(config.cache.max_entries, 10_000);
        assert_eq!(config.cache.ttl_seconds, 3600);
        assert_eq!(config.retry.max_attempts, 3);
    }

    #[test]
    fn test_config_validation() {
        let mut config = EmbeddingConfig::default();
        assert!(config.validate().is_err()); // Missing API key

        config.api_key = Some("test-key".to_string());
        assert!(config.validate().is_ok());

        config.dimension = 0;
        assert!(config.validate().is_err());

        config.dimension = 1536;
        config.batch_size = 0;
        assert!(config.validate().is_err());

        config.batch_size = 2000;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_retry_delay_calculation() {
        let retry = RetryConfig::default();

        assert_eq!(retry.delay_for_attempt(0), 0);
        assert_eq!(retry.delay_for_attempt(1), 1000);
        assert_eq!(retry.delay_for_attempt(2), 2000);
        assert_eq!(retry.delay_for_attempt(3), 4000);
        assert_eq!(retry.delay_for_attempt(4), 8000);
    }

    #[test]
    fn test_retry_max_delay() {
        let retry = RetryConfig {
            max_delay_ms: 5000,
            ..Default::default()
        };

        assert_eq!(retry.delay_for_attempt(10), 5000); // Capped at max
    }

    #[test]
    fn test_api_endpoint_url() {
        let mut config = EmbeddingConfig::default();
        assert_eq!(
            config.api_endpoint_url(),
            "https://api.openai.com/v1/embeddings"
        );

        config.provider = Provider::Cohere;
        assert_eq!(
            config.api_endpoint_url(),
            "https://api.cohere.ai/v1/embed"
        );

        config.provider = Provider::Ollama;
        assert_eq!(
            config.api_endpoint_url(),
            "http://localhost:11434/api/embeddings"
        );

        config.provider = Provider::Local;
        assert_eq!(
            config.api_endpoint_url(),
            "http://localhost:8080/embeddings"
        );

        config.api_endpoint = Some("https://custom.endpoint.com".to_string());
        assert_eq!(config.api_endpoint_url(), "https://custom.endpoint.com");
    }

    #[test]
    fn test_cache_config_validation() {
        let mut cache = CacheConfig::default();
        assert!(cache.validate().is_ok());

        cache.max_entries = 0;
        assert!(cache.validate().is_err());

        // TTL of 0 is now allowed (immediate expiration for testing)
        cache.max_entries = 100;
        cache.ttl_seconds = 0;
        assert!(cache.validate().is_ok());
    }

    #[test]
    fn test_retry_config_validation() {
        let mut retry = RetryConfig::default();
        assert!(retry.validate().is_ok());

        retry.max_attempts = 0;
        assert!(retry.validate().is_err());

        retry.max_attempts = 3;
        retry.backoff_multiplier = 1.0;
        assert!(retry.validate().is_err());

        retry.backoff_multiplier = 2.0;
        retry.max_delay_ms = 500;
        assert!(retry.validate().is_err());
    }

    #[test]
    fn test_ollama_validation_no_api_key() {
        // Ollama should not require an API key
        let config = EmbeddingConfig {
            provider: Provider::Ollama,
            model: "nomic-embed-text".to_string(),
            dimension: 768,
            api_key: None,
            ..Default::default()
        };
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_local_validation_no_api_key() {
        // Local provider should not require an API key
        let config = EmbeddingConfig {
            provider: Provider::Local,
            model: "custom-model".to_string(),
            dimension: 512,
            api_key: None,
            ..Default::default()
        };
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_ollama_nomic_embed_text_correct_dimension() {
        // nomic-embed-text with correct dimension should pass
        let config = EmbeddingConfig {
            provider: Provider::Ollama,
            model: "nomic-embed-text".to_string(),
            dimension: 768,
            api_key: None,
            ..Default::default()
        };
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_ollama_nomic_embed_text_wrong_dimension() {
        // nomic-embed-text with wrong dimension should fail
        let config = EmbeddingConfig {
            provider: Provider::Ollama,
            model: "nomic-embed-text".to_string(),
            dimension: 512,
            api_key: None,
            ..Default::default()
        };
        let result = config.validate();
        assert!(result.is_err());

        if let Err(ConfigError::InvalidValue { field, reason }) = result {
            assert_eq!(field, "dimension");
            assert!(reason.contains("nomic-embed-text"));
            assert!(reason.contains("768"));
            assert!(reason.contains("512"));
        } else {
            panic!("Expected InvalidValue error");
        }
    }

    #[test]
    fn test_ollama_other_models_flexible_dimensions() {
        // Other Ollama models should accept any valid dimension
        let config = EmbeddingConfig {
            provider: Provider::Ollama,
            model: "llama2".to_string(),
            dimension: 512,
            api_key: None,
            ..Default::default()
        };
        assert!(config.validate().is_ok());

        let config = EmbeddingConfig {
            provider: Provider::Ollama,
            model: "mistral".to_string(),
            dimension: 1024,
            api_key: None,
            ..Default::default()
        };
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_openai_requires_api_key() {
        // OpenAI should require an API key
        let mut config = EmbeddingConfig {
            provider: Provider::OpenAI,
            model: "text-embedding-3-small".to_string(),
            dimension: 1536,
            api_key: None,
            ..Default::default()
        };
        assert!(config.validate().is_err());

        config.api_key = Some("sk-test-key".to_string());
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_cohere_requires_api_key() {
        // Cohere should require an API key
        let mut config = EmbeddingConfig {
            provider: Provider::Cohere,
            model: "embed-english-v3.0".to_string(),
            dimension: 1024,
            api_key: None,
            ..Default::default()
        };
        assert!(config.validate().is_err());

        config.api_key = Some("cohere-test-key".to_string());
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_custom_endpoint_override() {
        // Custom endpoint should override default
        let config = EmbeddingConfig {
            provider: Provider::Ollama,
            model: "nomic-embed-text".to_string(),
            dimension: 768,
            api_key: None,
            api_endpoint: Some("http://custom-ollama:8080/api/embeddings".to_string()),
            ..Default::default()
        };
        assert_eq!(
            config.api_endpoint_url(),
            "http://custom-ollama:8080/api/embeddings"
        );
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_endpoint_defaults_all_providers() {
        // Test default endpoints for all providers
        let mut config = EmbeddingConfig::default();

        config.provider = Provider::OpenAI;
        assert_eq!(
            config.api_endpoint_url(),
            "https://api.openai.com/v1/embeddings"
        );

        config.provider = Provider::Cohere;
        assert_eq!(
            config.api_endpoint_url(),
            "https://api.cohere.ai/v1/embed"
        );

        config.provider = Provider::Ollama;
        assert_eq!(
            config.api_endpoint_url(),
            "http://localhost:11434/api/embeddings"
        );

        config.provider = Provider::Local;
        assert_eq!(
            config.api_endpoint_url(),
            "http://localhost:8080/embeddings"
        );
    }
}
