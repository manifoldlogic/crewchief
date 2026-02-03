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
    /// Google Vertex AI embedding API
    Google,
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
            "google" => Ok(Self::Google),
            "local" => Ok(Self::Local),
            _ => Err(ConfigError::InvalidValue {
                field: "provider".to_string(),
                reason: format!(
                    "Unknown provider: {}. Supported: openai, cohere, ollama, google, local",
                    s
                ),
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

    /// Parallel processing configuration
    pub parallel: ParallelConfig,
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
            parallel: ParallelConfig::default(),
        }
    }
}

impl EmbeddingConfig {
    /// Create a new configuration with defaults.
    pub fn new() -> Self {
        Self::default()
    }

    /// Load configuration from environment variables with optional provider override.
    ///
    /// This method enables factory-detected providers (e.g., auto-detected Ollama)
    /// to be correctly propagated during configuration loading. The provider override
    /// is applied before loading environment variables, so explicit env vars always win.
    ///
    /// # Arguments
    ///
    /// * `provider_override` - Optional provider to use if not specified in environment.
    ///                        Applied before env var loading, so env vars take precedence.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use crewchief_maproom::embedding::config::{EmbeddingConfig, Provider};
    ///
    /// // Factory-detected Ollama without env vars
    /// let config = EmbeddingConfig::from_env_with_provider(Some(Provider::Ollama))?;
    /// // Will use Provider::Ollama, infer model and dimension
    ///
    /// // Env vars override programmatic provider
    /// std::env::set_var("MAPROOM_EMBEDDING_PROVIDER", "openai");
    /// let config = EmbeddingConfig::from_env_with_provider(Some(Provider::Ollama))?;
    /// // Will use Provider::OpenAI from env var, not Ollama from override
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn from_env_with_provider(
        provider_override: Option<Provider>,
    ) -> Result<Self, EmbeddingError> {
        let mut config = Self::default();

        // Apply programmatic provider override first if provided
        if let Some(p) = provider_override {
            config.provider = p;
        }

        // Load provider from env (can override programmatic setting)
        if let Ok(provider) = env::var("MAPROOM_EMBEDDING_PROVIDER") {
            config.provider = provider.parse()?;
        }

        // Load model
        if let Ok(model) = env::var("MAPROOM_EMBEDDING_MODEL") {
            config.model = model;
        }

        // NEW: Default to Ollama model if provider is Ollama and model is still OpenAI default
        // This ensures inference sees the correct model in zero-config scenarios
        // Note: Config defaults are OpenAI-centric (dimension: 1536, model: "text-embedding-3-small")
        // but factory defaults to Ollama with mxbai-embed-large when auto-detecting
        if config.provider == Provider::Ollama && config.model == "text-embedding-3-small" {
            config.model = "mxbai-embed-large".to_string();
            tracing::debug!("Defaulting to mxbai-embed-large for Ollama provider");
        }

        // Track whether dimension was explicitly set (clearer than checking is_err() later)
        let explicit_dimension = env::var("MAPROOM_EMBEDDING_DIMENSION").ok();

        // NEW: Infer dimension for Ollama if not explicitly configured
        // This fixes the bug where zero-config setups use wrong default dimension
        // Precedence: explicit > inferred > default
        if explicit_dimension.is_none() && config.provider == Provider::Ollama {
            if let Some(inferred_dim) = infer_ollama_dimension(&config.model) {
                tracing::debug!(
                    "Inferred dimension {} for Ollama model '{}'",
                    inferred_dim,
                    config.model
                );
                config.dimension = inferred_dim;
            } else {
                tracing::warn!(
                    "Unknown Ollama model '{}'. Cannot infer embedding dimension. \
                     Please set MAPROOM_EMBEDDING_DIMENSION explicitly for custom models. \
                     Defaulting to {} dimensions - this may cause errors if incorrect.",
                    config.model,
                    config.dimension
                );
            }
        }

        // Apply explicit dimension if provided (overrides inference)
        if let Some(dim_str) = explicit_dimension {
            config.dimension = dim_str.parse().map_err(|_| ConfigError::InvalidValue {
                field: "EMBEDDING_DIMENSION".to_string(),
                reason: "Must be a positive integer".to_string(),
            })?;
        }

        // Load cache size
        if let Ok(size) = env::var("MAPROOM_EMBEDDING_CACHE_SIZE") {
            config.cache.max_entries = size.parse().map_err(|_| ConfigError::InvalidValue {
                field: "EMBEDDING_CACHE_SIZE".to_string(),
                reason: "Must be a positive integer".to_string(),
            })?;
        }

        // Load cache TTL
        if let Ok(ttl) = env::var("MAPROOM_EMBEDDING_CACHE_TTL") {
            config.cache.ttl_seconds = ttl.parse().map_err(|_| ConfigError::InvalidValue {
                field: "EMBEDDING_CACHE_TTL".to_string(),
                reason: "Must be a positive integer".to_string(),
            })?;
        }

        // Load batch size
        if let Ok(batch) = env::var("MAPROOM_EMBEDDING_BATCH_SIZE") {
            config.batch_size = batch.parse().map_err(|_| ConfigError::InvalidValue {
                field: "EMBEDDING_BATCH_SIZE".to_string(),
                reason: "Must be a positive integer".to_string(),
            })?;
        }

        // Load retry max attempts
        if let Ok(max_attempts) = env::var("MAPROOM_EMBEDDING_RETRY_MAX_ATTEMPTS") {
            config.retry.max_attempts =
                max_attempts
                    .parse()
                    .map_err(|_| ConfigError::InvalidValue {
                        field: "EMBEDDING_RETRY_MAX_ATTEMPTS".to_string(),
                        reason: "Must be a positive integer".to_string(),
                    })?;
        }

        // Load API key based on provider
        // Try Maproom-specific env vars first, then fall back to standard vars
        config.api_key = match config.provider {
            Provider::OpenAI => env::var("MAPROOM_OPENAI_API_KEY")
                .or_else(|_| env::var("OPENAI_API_KEY"))
                .ok(),
            Provider::Cohere => env::var("MAPROOM_COHERE_API_KEY")
                .or_else(|_| env::var("COHERE_API_KEY"))
                .ok(),
            Provider::Ollama => None, // Ollama runs locally, no API key needed
            Provider::Google => None, // Google uses service account JSON, not API key
            Provider::Local => None,  // Local models don't need API keys
        };

        // Provider-aware endpoint loading and validation (PROVFIX-1001)
        //
        // This validation prevents cross-provider endpoint pollution, which was causing
        // critical bugs where cloud providers (OpenAI, Cohere) would inherit Ollama's
        // default endpoint from Docker Compose environment variables.
        //
        // Example of the bug this prevents:
        //   - Docker Compose sets: EMBEDDING_API_ENDPOINT=http://ollama:11434
        //   - User configures: MAPROOM_EMBEDDING_PROVIDER=openai
        //   - Without validation: OpenAI attempts connection to localhost:11434 (fails)
        //   - With validation: OpenAI ignores Ollama endpoint, uses api.openai.com (works)
        //
        // Validation rules by provider:
        //   - OpenAI: Only endpoints containing "openai.com" accepted
        //   - Cohere: Only endpoints containing "cohere" accepted
        //   - Ollama/Local: Any endpoint accepted (flexible for self-hosting)
        //   - Google: Ignores EMBEDDING_API_ENDPOINT (uses region-based construction)
        //
        // See PROVFIX project documentation for full context on this critical fix.
        //
        if let Ok(endpoint) = env::var("MAPROOM_EMBEDDING_API_ENDPOINT") {
            match config.provider {
                Provider::OpenAI => {
                    // Only accept OpenAI endpoints
                    if endpoint.contains("openai.com") {
                        config.api_endpoint = Some(endpoint);
                    }
                    // Otherwise ignore - wrong provider's endpoint
                }
                Provider::Cohere => {
                    // Only accept Cohere endpoints
                    if endpoint.contains("cohere") {
                        config.api_endpoint = Some(endpoint);
                    }
                    // Otherwise ignore - wrong provider's endpoint
                }
                Provider::Ollama | Provider::Local => {
                    // Accept any endpoint for Ollama and Local providers
                    config.api_endpoint = Some(endpoint);
                }
                Provider::Google => {
                    // Google doesn't use EMBEDDING_API_ENDPOINT
                    // Endpoint is constructed from region/project
                    // Ignore any endpoint setting
                }
            }
        }

        // Load parallel processing configuration
        if let Ok(enabled) = env::var("MAPROOM_EMBEDDING_PARALLEL_ENABLED") {
            config.parallel.enabled = enabled.parse().unwrap_or(true);
        }

        if let Ok(sub_batch) = env::var("MAPROOM_EMBEDDING_PARALLEL_SUB_BATCH_SIZE") {
            config.parallel.sub_batch_size =
                sub_batch.parse().map_err(|_| ConfigError::InvalidValue {
                    field: "EMBEDDING_PARALLEL_SUB_BATCH_SIZE".to_string(),
                    reason: "Must be a positive integer".to_string(),
                })?;
        }

        if let Ok(concurrency) = env::var("MAPROOM_EMBEDDING_PARALLEL_MAX_CONCURRENCY") {
            config.parallel.max_concurrency =
                concurrency.parse().map_err(|_| ConfigError::InvalidValue {
                    field: "EMBEDDING_PARALLEL_MAX_CONCURRENCY".to_string(),
                    reason: "Must be a positive integer".to_string(),
                })?;
        }

        Ok(config)
    }

    /// Load configuration from environment variables.
    ///
    /// This is a convenience method that delegates to `from_env_with_provider(None)`.
    /// Use `from_env_with_provider` when you need to provide a programmatic provider
    /// override (e.g., factory-detected Ollama).
    pub fn from_env() -> Result<Self, EmbeddingError> {
        Self::from_env_with_provider(None)
    }

    /// Validate the configuration.
    pub fn validate(&self) -> Result<(), ConfigError> {
        // Check API key for cloud providers
        if matches!(self.provider, Provider::OpenAI | Provider::Cohere) && self.api_key.is_none() {
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

        // Validate Ollama-specific model/dimension combinations (warnings only)
        if self.provider == Provider::Ollama {
            match self.model.as_str() {
                "nomic-embed-text" if self.dimension != 768 => {
                    tracing::warn!(
                        "nomic-embed-text typically uses 768 dimensions, got {}. \
                         Ensure your Ollama model is configured correctly.",
                        self.dimension
                    );
                }
                "mxbai-embed-large" if self.dimension != 1024 => {
                    tracing::warn!(
                        "mxbai-embed-large typically uses 1024 dimensions, got {}. \
                         Ensure your Ollama model is configured correctly.",
                        self.dimension
                    );
                }
                _ => {
                    // Other models: no specific validation, trust user configuration
                }
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

        // Validate parallel config
        self.parallel.validate()?;

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
                Provider::Ollama => "http://localhost:11434/api/embed".to_string(),
                Provider::Google => {
                    // Google endpoint is region-specific and constructed by GoogleProvider
                    // Default to us-central1 for compatibility
                    let region =
                        env::var("GOOGLE_REGION").unwrap_or_else(|_| "us-central1".to_string());
                    let project =
                        env::var("GOOGLE_PROJECT_ID").unwrap_or_else(|_| "unknown".to_string());
                    format!("https://{}-aiplatform.googleapis.com/v1/projects/{}/locations/{}/publishers/google/models/textembedding-gecko@003:predict",
                            region, project, region)
                }
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

/// Parallel processing configuration for batch embedding.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParallelConfig {
    /// Enable parallel batch processing (split batches into concurrent sub-batches)
    pub enabled: bool,

    /// Size of each sub-batch when parallel processing is enabled
    pub sub_batch_size: usize,

    /// Maximum number of concurrent requests
    pub max_concurrency: usize,
}

impl Default for ParallelConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            sub_batch_size: 50, // Updated for EMBPERF-2001: better throughput
            max_concurrency: 8, // Updated for EMBPERF-2001: higher concurrency
        }
    }
}

impl ParallelConfig {
    /// Create a parallel config optimized for Google Vertex AI.
    ///
    /// Google Vertex AI has different optimal settings than local Ollama due to
    /// the I/O-bound nature of cloud API calls vs local inference.
    ///
    /// # Default Values
    ///
    /// - `enabled`: `true` - Parallel processing is enabled by default
    /// - `sub_batch_size`: `200` - Near the 250 API limit with 20% safety margin
    /// - `max_concurrency`: `16` - Higher concurrency for network-bound operations
    ///
    /// # Rationale
    ///
    /// **Sub-batch size (200):** The Vertex AI API accepts up to 250 texts per
    /// request. Using 200 provides a safety margin for variable token lengths
    /// while still maximizing throughput per request.
    ///
    /// **Concurrency (16):** Cloud APIs are I/O-bound (waiting for network),
    /// so higher concurrency is beneficial. 16 concurrent requests provides
    /// good throughput without hitting rate limits on typical quotas.
    ///
    /// # When to Use
    ///
    /// Use `google_defaults()` when:
    /// - Creating a `GoogleProvider` programmatically
    /// - You need Google-optimized parallel settings
    ///
    /// Use `ParallelConfig::default()` (Ollama defaults) when:
    /// - Using local Ollama provider
    /// - CPU/GPU bound inference where high concurrency causes contention
    ///
    /// # Examples
    ///
    /// ```rust
    /// use crewchief_maproom::embedding::config::ParallelConfig;
    ///
    /// let config = ParallelConfig::google_defaults();
    /// assert!(config.enabled);
    /// assert_eq!(config.sub_batch_size, 200);
    /// assert_eq!(config.max_concurrency, 16);
    /// ```
    ///
    /// # See Also
    ///
    /// - [`ParallelConfig::default()`] for Ollama-optimized defaults
    /// - [`GoogleProvider::new_with_config()`](crate::embedding::google::GoogleProvider::new_with_config)
    ///   for creating a provider with custom parallel settings
    pub fn google_defaults() -> Self {
        Self {
            enabled: true,
            sub_batch_size: 200,
            max_concurrency: 16,
        }
    }

    /// Validate parallel configuration.
    pub fn validate(&self) -> Result<(), ConfigError> {
        if self.sub_batch_size == 0 {
            return Err(ConfigError::InvalidValue {
                field: "parallel.sub_batch_size".to_string(),
                reason: "Must be greater than 0".to_string(),
            });
        }

        if self.max_concurrency == 0 {
            return Err(ConfigError::InvalidValue {
                field: "parallel.max_concurrency".to_string(),
                reason: "Must be greater than 0".to_string(),
            });
        }

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

        let delay =
            (self.initial_delay_ms as f32) * self.backoff_multiplier.powi((attempt - 1) as i32);
        delay.min(self.max_delay_ms as f32) as u64
    }
}

/// Infer embedding dimension from known Ollama model names.
///
/// Uses prefix matching to handle model tags (e.g., "mxbai-embed-large:latest").
/// Returns the expected dimension for well-known models, or None for unknown models.
/// This enables zero-config workflows where dimension is automatically determined
/// from the model name without requiring explicit MAPROOM_EMBEDDING_DIMENSION.
///
/// # Supported Models
///
/// - `nomic-embed-text*`: 768 dimensions (matches tags like "nomic-embed-text:latest")
/// - `mxbai-embed-large*`: 1024 dimensions (matches tags like "mxbai-embed-large:v1")
///
/// # Returns
///
/// - `Some(dimension)` for known models
/// - `None` for unknown models (caller should warn and use default)
fn infer_ollama_dimension(model: &str) -> Option<usize> {
    if model.starts_with("nomic-embed-text") {
        Some(768)
    } else if model.starts_with("mxbai-embed-large") {
        Some(1024)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;

    #[test]
    fn test_provider_parsing() {
        assert_eq!("openai".parse::<Provider>().unwrap(), Provider::OpenAI);
        assert_eq!("cohere".parse::<Provider>().unwrap(), Provider::Cohere);
        assert_eq!("ollama".parse::<Provider>().unwrap(), Provider::Ollama);
        assert_eq!("google".parse::<Provider>().unwrap(), Provider::Google);
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
        assert_eq!(config.api_endpoint_url(), "https://api.cohere.ai/v1/embed");

        config.provider = Provider::Ollama;
        assert_eq!(
            config.api_endpoint_url(),
            "http://localhost:11434/api/embed"
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
        // nomic-embed-text with wrong dimension should now pass with warning (not error)
        let config = EmbeddingConfig {
            provider: Provider::Ollama,
            model: "nomic-embed-text".to_string(),
            dimension: 512,
            api_key: None,
            ..Default::default()
        };
        let result = config.validate();
        // Should pass validation (warnings logged, but no error)
        assert!(result.is_ok());
    }

    #[test]
    fn test_ollama_mxbai_embed_large_dimension_1024() {
        // mxbai-embed-large with correct dimension should pass
        let config = EmbeddingConfig {
            provider: Provider::Ollama,
            model: "mxbai-embed-large".to_string(),
            dimension: 1024,
            api_key: None,
            ..Default::default()
        };
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_ollama_mxbai_embed_large_wrong_dimension() {
        // mxbai-embed-large with wrong dimension should pass with warning (not error)
        let config = EmbeddingConfig {
            provider: Provider::Ollama,
            model: "mxbai-embed-large".to_string(),
            dimension: 768,
            api_key: None,
            ..Default::default()
        };
        let result = config.validate();
        // Should pass validation (warnings logged, but no error)
        assert!(result.is_ok());
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
        assert_eq!(config.api_endpoint_url(), "https://api.cohere.ai/v1/embed");

        config.provider = Provider::Ollama;
        assert_eq!(
            config.api_endpoint_url(),
            "http://localhost:11434/api/embed"
        );

        config.provider = Provider::Google;
        // Google endpoint should be constructed with region and project
        let endpoint = config.api_endpoint_url();
        assert!(endpoint.contains("aiplatform.googleapis.com"));
        assert!(endpoint.contains("textembedding-gecko@003:predict"));

        config.provider = Provider::Local;
        assert_eq!(
            config.api_endpoint_url(),
            "http://localhost:8080/embeddings"
        );
    }

    #[test]
    fn test_infer_ollama_dimension_known_models() {
        assert_eq!(infer_ollama_dimension("nomic-embed-text"), Some(768));
        assert_eq!(infer_ollama_dimension("mxbai-embed-large"), Some(1024));
    }

    #[test]
    fn test_infer_ollama_dimension_with_tags() {
        assert_eq!(infer_ollama_dimension("nomic-embed-text:latest"), Some(768));
        assert_eq!(
            infer_ollama_dimension("mxbai-embed-large:latest"),
            Some(1024)
        );
        assert_eq!(infer_ollama_dimension("mxbai-embed-large:v1"), Some(1024));
    }

    #[test]
    fn test_infer_ollama_dimension_unknown_model() {
        assert_eq!(infer_ollama_dimension("custom-model"), None);
        assert_eq!(infer_ollama_dimension("unknown"), None);
    }

    // Integration tests for dimension inference in from_env()

    #[test]
    #[serial]
    fn test_from_env_infers_dimension_mxbai() {
        // Test that mxbai-embed-large model infers 1024 dimensions
        env::set_var("MAPROOM_EMBEDDING_PROVIDER", "ollama");
        env::set_var("MAPROOM_EMBEDDING_MODEL", "mxbai-embed-large");
        env::remove_var("MAPROOM_EMBEDDING_DIMENSION");

        let config = EmbeddingConfig::from_env().unwrap();
        assert_eq!(config.dimension, 1024);
        assert_eq!(config.model, "mxbai-embed-large");

        // Cleanup
        env::remove_var("MAPROOM_EMBEDDING_PROVIDER");
        env::remove_var("MAPROOM_EMBEDDING_MODEL");
    }

    #[test]
    #[serial]
    fn test_from_env_infers_dimension_nomic() {
        // Test that nomic-embed-text model infers 768 dimensions
        env::set_var("MAPROOM_EMBEDDING_PROVIDER", "ollama");
        env::set_var("MAPROOM_EMBEDDING_MODEL", "nomic-embed-text");
        env::remove_var("MAPROOM_EMBEDDING_DIMENSION");

        let config = EmbeddingConfig::from_env().unwrap();
        assert_eq!(config.dimension, 768);
        assert_eq!(config.model, "nomic-embed-text");

        // Cleanup
        env::remove_var("MAPROOM_EMBEDDING_PROVIDER");
        env::remove_var("MAPROOM_EMBEDDING_MODEL");
    }

    #[test]
    #[serial]
    fn test_from_env_explicit_dimension_overrides_inference() {
        // Test that explicit dimension overrides inference
        env::set_var("MAPROOM_EMBEDDING_PROVIDER", "ollama");
        env::set_var("MAPROOM_EMBEDDING_MODEL", "mxbai-embed-large");
        env::set_var("MAPROOM_EMBEDDING_DIMENSION", "2048");

        let config = EmbeddingConfig::from_env().unwrap();
        assert_eq!(config.dimension, 2048); // Explicit wins over inferred 1024
        assert_eq!(config.model, "mxbai-embed-large");

        // Cleanup
        env::remove_var("MAPROOM_EMBEDDING_PROVIDER");
        env::remove_var("MAPROOM_EMBEDDING_MODEL");
        env::remove_var("MAPROOM_EMBEDDING_DIMENSION");
    }

    #[test]
    #[serial]
    fn test_from_env_unknown_model_keeps_default() {
        // Test that unknown Ollama model uses default dimension (1536)
        env::set_var("MAPROOM_EMBEDDING_PROVIDER", "ollama");
        env::set_var("MAPROOM_EMBEDDING_MODEL", "custom-unknown-model");
        env::remove_var("MAPROOM_EMBEDDING_DIMENSION");

        let config = EmbeddingConfig::from_env().unwrap();
        assert_eq!(config.dimension, 1536); // Default dimension kept
        assert_eq!(config.model, "custom-unknown-model");

        // Cleanup
        env::remove_var("MAPROOM_EMBEDDING_PROVIDER");
        env::remove_var("MAPROOM_EMBEDDING_MODEL");
    }

    #[test]
    #[serial]
    fn test_from_env_inference_only_for_ollama() {
        // Test that inference doesn't affect non-Ollama providers
        env::set_var("MAPROOM_EMBEDDING_PROVIDER", "openai");
        env::set_var("MAPROOM_EMBEDDING_MODEL", "mxbai-embed-large");
        env::remove_var("MAPROOM_EMBEDDING_DIMENSION");

        let config = EmbeddingConfig::from_env().unwrap();
        assert_eq!(config.dimension, 1536); // Default OpenAI dimension, not inferred
        assert_eq!(config.model, "mxbai-embed-large");

        // Cleanup
        env::remove_var("MAPROOM_EMBEDDING_PROVIDER");
        env::remove_var("MAPROOM_EMBEDDING_MODEL");
    }

    #[test]
    #[serial]
    fn test_from_env_zero_config_ollama() {
        // Test true zero-config: no env vars set, provider is Ollama (from default)
        // Actually, default provider is OpenAI, so we need to set provider to Ollama
        // This tests the model defaulting + inference flow
        env::set_var("MAPROOM_EMBEDDING_PROVIDER", "ollama");
        env::remove_var("MAPROOM_EMBEDDING_MODEL");
        env::remove_var("MAPROOM_EMBEDDING_DIMENSION");

        let config = EmbeddingConfig::from_env().unwrap();
        assert_eq!(config.provider, Provider::Ollama);
        assert_eq!(config.model, "mxbai-embed-large"); // Defaulted from OpenAI default
        assert_eq!(config.dimension, 1024); // Inferred from defaulted model

        // Cleanup
        env::remove_var("MAPROOM_EMBEDDING_PROVIDER");
    }

    // Tests for from_env_with_provider() (MPRSKL.1001)

    #[test]
    #[serial]
    fn test_from_env_with_provider_none() {
        // Test that None behaves same as from_env()
        env::set_var("MAPROOM_EMBEDDING_PROVIDER", "openai");
        env::set_var("MAPROOM_EMBEDDING_MODEL", "text-embedding-3-small");
        env::remove_var("MAPROOM_EMBEDDING_DIMENSION");

        let config_from_env = EmbeddingConfig::from_env().unwrap();
        let config_with_none = EmbeddingConfig::from_env_with_provider(None).unwrap();

        assert_eq!(config_from_env.provider, config_with_none.provider);
        assert_eq!(config_from_env.model, config_with_none.model);
        assert_eq!(config_from_env.dimension, config_with_none.dimension);

        // Cleanup
        env::remove_var("MAPROOM_EMBEDDING_PROVIDER");
        env::remove_var("MAPROOM_EMBEDDING_MODEL");
    }

    #[test]
    #[serial]
    fn test_from_env_with_provider_ollama() {
        // Test that Provider::Ollama override enables dimension inference
        // No env vars set - pure programmatic override
        env::remove_var("MAPROOM_EMBEDDING_PROVIDER");
        env::remove_var("MAPROOM_EMBEDDING_MODEL");
        env::remove_var("MAPROOM_EMBEDDING_DIMENSION");

        let config = EmbeddingConfig::from_env_with_provider(Some(Provider::Ollama)).unwrap();

        assert_eq!(config.provider, Provider::Ollama);
        assert_eq!(config.model, "mxbai-embed-large"); // Auto-defaulted for Ollama
        assert_eq!(config.dimension, 1024); // Inferred from mxbai-embed-large
    }

    #[test]
    #[serial]
    fn test_from_env_with_provider_env_override() {
        // Test that env var overrides programmatic provider
        env::set_var("MAPROOM_EMBEDDING_PROVIDER", "openai");
        env::remove_var("MAPROOM_EMBEDDING_MODEL");
        env::remove_var("MAPROOM_EMBEDDING_DIMENSION");

        // Programmatic override says Ollama, but env var says OpenAI
        let config = EmbeddingConfig::from_env_with_provider(Some(Provider::Ollama)).unwrap();

        assert_eq!(config.provider, Provider::OpenAI); // Env var wins
        assert_eq!(config.model, "text-embedding-3-small"); // OpenAI default
        assert_eq!(config.dimension, 1536); // OpenAI default (no inference for non-Ollama)

        // Cleanup
        env::remove_var("MAPROOM_EMBEDDING_PROVIDER");
    }

    // Tests for ParallelConfig::google_defaults() (GVERTEX.1005)

    #[test]
    fn test_parallel_config_google_defaults() {
        let config = ParallelConfig::google_defaults();
        assert!(config.enabled);
        assert_eq!(config.sub_batch_size, 200);
        assert_eq!(config.max_concurrency, 16);
    }

    #[test]
    fn test_parallel_config_google_defaults_values() {
        // Individual field assertions for clarity
        let config = ParallelConfig::google_defaults();

        // enabled should be true for parallel processing
        assert!(
            config.enabled,
            "Google defaults should have parallel processing enabled"
        );

        // sub_batch_size should be 200 (near 250 API limit with safety margin)
        assert_eq!(
            config.sub_batch_size, 200,
            "Google defaults should use sub_batch_size=200 (near 250 API limit)"
        );

        // max_concurrency should be 16 (higher for I/O-bound cloud API)
        assert_eq!(
            config.max_concurrency, 16,
            "Google defaults should use max_concurrency=16 (optimized for cloud API)"
        );
    }
}

/// Tests for endpoint resolution with provider-aware validation (PROVFIX-1002)
#[cfg(test)]
mod config_endpoint_tests {
    use super::*;
    use serial_test::serial;

    // OpenAI Provider Tests

    #[test]
    #[serial]
    fn test_openai_uses_default_endpoint() {
        // No MAPROOM_EMBEDDING_API_ENDPOINT set
        env::remove_var("MAPROOM_EMBEDDING_API_ENDPOINT");
        env::set_var("MAPROOM_EMBEDDING_PROVIDER", "openai");

        let config = EmbeddingConfig::from_env().unwrap();
        assert_eq!(
            config.api_endpoint_url(),
            "https://api.openai.com/v1/embeddings"
        );

        // Cleanup
        env::remove_var("MAPROOM_EMBEDDING_PROVIDER");
    }

    #[test]
    #[serial]
    fn test_openai_ignores_ollama_endpoint() {
        // THIS IS THE BUG TEST - verify fix prevents regression
        // Set up: Ollama endpoint in environment (like Docker Compose default)
        env::set_var(
            "MAPROOM_EMBEDDING_API_ENDPOINT",
            "http://localhost:11434/api/embed",
        );
        env::set_var("MAPROOM_EMBEDDING_PROVIDER", "openai");

        let config = EmbeddingConfig::from_env().unwrap();

        // Assert: OpenAI should use its default, NOT the Ollama endpoint
        assert_eq!(
            config.api_endpoint_url(),
            "https://api.openai.com/v1/embeddings"
        );

        // Cleanup
        env::remove_var("MAPROOM_EMBEDDING_API_ENDPOINT");
        env::remove_var("MAPROOM_EMBEDDING_PROVIDER");
    }

    #[test]
    #[serial]
    fn test_openai_accepts_custom_openai_endpoint() {
        // Allow explicit OpenAI endpoint override
        env::set_var(
            "MAPROOM_EMBEDDING_API_ENDPOINT",
            "https://api.openai.com/v2/embeddings",
        );
        env::set_var("MAPROOM_EMBEDDING_PROVIDER", "openai");

        let config = EmbeddingConfig::from_env().unwrap();
        assert_eq!(
            config.api_endpoint_url(),
            "https://api.openai.com/v2/embeddings"
        );

        // Cleanup
        env::remove_var("MAPROOM_EMBEDDING_API_ENDPOINT");
        env::remove_var("MAPROOM_EMBEDDING_PROVIDER");
    }

    // Cohere Provider Tests

    #[test]
    #[serial]
    fn test_cohere_uses_default_endpoint() {
        // No MAPROOM_EMBEDDING_API_ENDPOINT set
        env::remove_var("MAPROOM_EMBEDDING_API_ENDPOINT");
        env::set_var("MAPROOM_EMBEDDING_PROVIDER", "cohere");

        let config = EmbeddingConfig::from_env().unwrap();
        assert_eq!(config.api_endpoint_url(), "https://api.cohere.ai/v1/embed");

        // Cleanup
        env::remove_var("MAPROOM_EMBEDDING_PROVIDER");
    }

    #[test]
    #[serial]
    fn test_cohere_ignores_wrong_endpoint() {
        // Cohere should ignore Ollama endpoint
        env::set_var(
            "MAPROOM_EMBEDDING_API_ENDPOINT",
            "http://localhost:11434/api/embed",
        );
        env::set_var("MAPROOM_EMBEDDING_PROVIDER", "cohere");

        let config = EmbeddingConfig::from_env().unwrap();
        assert_eq!(config.api_endpoint_url(), "https://api.cohere.ai/v1/embed");

        // Cleanup
        env::remove_var("MAPROOM_EMBEDDING_API_ENDPOINT");
        env::remove_var("MAPROOM_EMBEDDING_PROVIDER");
    }

    // Ollama Provider Tests

    #[test]
    #[serial]
    fn test_ollama_uses_custom_endpoint() {
        env::set_var(
            "MAPROOM_EMBEDDING_API_ENDPOINT",
            "http://custom:8080/api/embed",
        );
        env::set_var("MAPROOM_EMBEDDING_PROVIDER", "ollama");

        let config = EmbeddingConfig::from_env().unwrap();
        assert_eq!(config.api_endpoint_url(), "http://custom:8080/api/embed");

        // Cleanup
        env::remove_var("MAPROOM_EMBEDDING_API_ENDPOINT");
        env::remove_var("MAPROOM_EMBEDDING_PROVIDER");
    }

    #[test]
    #[serial]
    fn test_ollama_uses_default_if_no_override() {
        env::remove_var("MAPROOM_EMBEDDING_API_ENDPOINT");
        env::set_var("MAPROOM_EMBEDDING_PROVIDER", "ollama");

        let config = EmbeddingConfig::from_env().unwrap();
        assert_eq!(
            config.api_endpoint_url(),
            "http://localhost:11434/api/embed"
        );

        // Cleanup
        env::remove_var("MAPROOM_EMBEDDING_PROVIDER");
    }

    // Google Provider Tests

    #[test]
    #[serial]
    fn test_google_ignores_embedding_api_endpoint() {
        env::set_var(
            "MAPROOM_EMBEDDING_API_ENDPOINT",
            "http://localhost:11434/api/embed",
        );
        env::set_var("MAPROOM_EMBEDDING_PROVIDER", "google");
        env::set_var("GOOGLE_REGION", "us-central1");
        env::set_var("GOOGLE_PROJECT_ID", "test-project");

        let config = EmbeddingConfig::from_env().unwrap();
        // Should use region-based URL, not MAPROOM_EMBEDDING_API_ENDPOINT
        let endpoint = config.api_endpoint_url();
        assert!(endpoint.contains("us-central1"));
        assert!(endpoint.contains("aiplatform.googleapis.com"));
        assert!(!endpoint.contains("11434"));

        // Cleanup
        env::remove_var("MAPROOM_EMBEDDING_API_ENDPOINT");
        env::remove_var("MAPROOM_EMBEDDING_PROVIDER");
        env::remove_var("GOOGLE_REGION");
        env::remove_var("GOOGLE_PROJECT_ID");
    }
}
