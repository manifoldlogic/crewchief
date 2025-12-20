//! OpenAI API client with retry logic and error handling.

use crate::embedding::cache::Vector;
use crate::embedding::config::EmbeddingConfig;
use crate::embedding::error::{ApiError, EmbeddingError};
use crate::embedding::provider::{EmbeddingProvider, ProviderMetrics};
use async_trait::async_trait;
use reqwest::{Client, StatusCode};
use serde::Deserialize;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tokio::time::{sleep, Duration};
use tracing::{debug, info, warn};

/// OpenAI embedding API client.
pub struct OpenAIClient {
    /// HTTP client
    client: Client,
    /// Configuration
    config: Arc<EmbeddingConfig>,
    /// Cost tracking metrics
    metrics: Arc<CostMetrics>,
}

/// Cost tracking metrics for API usage.
#[derive(Debug, Default)]
pub struct CostMetrics {
    /// Total tokens processed
    pub total_tokens: AtomicU64,
    /// Total API requests made
    pub total_requests: AtomicU64,
    /// Total failed requests
    pub failed_requests: AtomicU64,
    /// Total retry attempts
    pub retry_attempts: AtomicU64,
}

impl CostMetrics {
    /// Get total tokens processed.
    pub fn total_tokens(&self) -> u64 {
        self.total_tokens.load(Ordering::Relaxed)
    }

    /// Get total requests made.
    pub fn total_requests(&self) -> u64 {
        self.total_requests.load(Ordering::Relaxed)
    }

    /// Get total failed requests.
    pub fn failed_requests(&self) -> u64 {
        self.failed_requests.load(Ordering::Relaxed)
    }

    /// Get total retry attempts.
    pub fn retry_attempts(&self) -> u64 {
        self.retry_attempts.load(Ordering::Relaxed)
    }

    /// Estimate cost in USD for text-embedding-3-small ($0.02 per 1M tokens).
    pub fn estimated_cost_usd(&self) -> f64 {
        let tokens = self.total_tokens() as f64;
        (tokens / 1_000_000.0) * 0.02
    }

    /// Reset all metrics.
    pub fn reset(&self) {
        self.total_tokens.store(0, Ordering::Relaxed);
        self.total_requests.store(0, Ordering::Relaxed);
        self.failed_requests.store(0, Ordering::Relaxed);
        self.retry_attempts.store(0, Ordering::Relaxed);
    }
}

/// OpenAI API response structure.
#[derive(Debug, Deserialize)]
struct EmbeddingResponse {
    data: Vec<EmbeddingData>,
    usage: Usage,
}

#[derive(Debug, Deserialize)]
struct EmbeddingData {
    embedding: Vec<f32>,
    index: usize,
}

#[derive(Debug, Deserialize)]
struct Usage {
    total_tokens: u64,
}

/// Ollama API response structure.
#[derive(Debug, Deserialize)]
struct OllamaEmbeddingResponse {
    #[allow(dead_code)]
    model: String,
    embeddings: Vec<Vec<f32>>,
}

/// OpenAI API error response.
#[derive(Debug, Deserialize)]
struct ErrorResponse {
    error: ErrorDetail,
}

#[derive(Debug, Deserialize)]
struct ErrorDetail {
    message: String,
    #[serde(rename = "type")]
    #[allow(dead_code)]
    error_type: String,
    #[allow(dead_code)]
    code: Option<String>,
}

impl OpenAIClient {
    /// Create a new OpenAI client.
    pub fn new(config: EmbeddingConfig) -> Result<Self, EmbeddingError> {
        config.validate()?;

        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .pool_max_idle_per_host(10) // Connection pooling for performance
            .pool_idle_timeout(Duration::from_secs(90))
            .http2_keep_alive_interval(Some(Duration::from_secs(30)))
            .http2_keep_alive_timeout(Duration::from_secs(10))
            .build()
            .map_err(EmbeddingError::Network)?;

        Ok(Self {
            client,
            config: Arc::new(config),
            metrics: Arc::new(CostMetrics::default()),
        })
    }

    /// Generate embeddings for a batch of texts with retry logic.
    pub async fn embed_batch(&self, texts: Vec<String>) -> Result<Vec<Vector>, EmbeddingError> {
        if texts.is_empty() {
            return Ok(Vec::new());
        }

        if texts.len() > self.config.batch_size {
            return Err(EmbeddingError::InvalidInput(format!(
                "Batch size {} exceeds maximum {}",
                texts.len(),
                self.config.batch_size
            )));
        }

        debug!("Embedding batch of {} texts", texts.len());

        let mut attempt = 0;
        let max_attempts = self.config.retry.max_attempts;

        loop {
            match self.try_embed_batch(&texts).await {
                Ok(embeddings) => {
                    info!(
                        "Successfully embedded {} texts (attempt {})",
                        texts.len(),
                        attempt + 1
                    );
                    return Ok(embeddings);
                }
                Err(EmbeddingError::Api(api_err))
                    if api_err.is_retryable() && attempt < max_attempts - 1 =>
                {
                    attempt += 1;
                    let delay = api_err
                        .retry_delay_ms()
                        .unwrap_or_else(|| self.config.retry.delay_for_attempt(attempt));

                    warn!(
                        "API error (attempt {}/{}): {}. Retrying in {}ms",
                        attempt, max_attempts, api_err, delay
                    );

                    self.metrics.retry_attempts.fetch_add(1, Ordering::Relaxed);
                    sleep(Duration::from_millis(delay)).await;
                }
                Err(e) => {
                    self.metrics.failed_requests.fetch_add(1, Ordering::Relaxed);
                    return Err(e);
                }
            }
        }
    }

    /// Single attempt to embed a batch (without retry logic).
    async fn try_embed_batch(&self, texts: &[String]) -> Result<Vec<Vector>, EmbeddingError> {
        use crate::embedding::config::Provider;

        let api_key = self.config.api_key.as_ref();

        // Build request based on provider
        let request = match self.config.provider {
            Provider::OpenAI => {
                let key = api_key.ok_or_else(|| {
                    EmbeddingError::Config(crate::embedding::error::ConfigError::MissingConfig(
                        "API key for OpenAI".to_string(),
                    ))
                })?;

                self.client
                    .post(self.config.api_endpoint_url())
                    .header("Authorization", format!("Bearer {}", key))
                    .header("Content-Type", "application/json")
            }
            Provider::Ollama => {
                // Ollama doesn't require API key
                self.client
                    .post(self.config.api_endpoint_url())
                    .header("Content-Type", "application/json")
            }
            Provider::Cohere => {
                let key = api_key.ok_or_else(|| {
                    EmbeddingError::Config(crate::embedding::error::ConfigError::MissingConfig(
                        "API key for Cohere".to_string(),
                    ))
                })?;

                self.client
                    .post(self.config.api_endpoint_url())
                    .header("Authorization", format!("Bearer {}", key))
                    .header("Content-Type", "application/json")
            }
            Provider::Google => {
                // Google uses factory system, not this client
                // Return error directing user to use MAPROOM_EMBEDDING_PROVIDER env var
                return Err(EmbeddingError::Config(crate::embedding::error::ConfigError::InvalidValue {
                    field: "provider".to_string(),
                    reason: "Google provider requires using MAPROOM_EMBEDDING_PROVIDER=google environment variable. \
                             The legacy OpenAIClient does not support Google Vertex AI. \
                             Use create_provider_from_env() for Google support.".to_string(),
                }));
            }
            Provider::Local => {
                // Local models don't require API key
                self.client
                    .post(self.config.api_endpoint_url())
                    .header("Content-Type", "application/json")
            }
        };

        // Build request body based on provider
        let body = if self.config.provider == Provider::Ollama {
            // Ollama uses "input" field (can be string or array)
            serde_json::json!({
                "model": self.config.model,
                "input": texts,
            })
        } else {
            // OpenAI and others use "input" field with dimensions
            serde_json::json!({
                "input": texts,
                "model": self.config.model,
                "dimensions": self.config.dimension,
            })
        };

        let response = request.json(&body).send().await?;

        self.metrics.total_requests.fetch_add(1, Ordering::Relaxed);

        let status = response.status();

        if !status.is_success() {
            return Err(self.handle_error_response(status, response).await);
        }

        // Parse response based on provider
        let embeddings = if self.config.provider == Provider::Ollama {
            // Parse Ollama response format
            let ollama_response: OllamaEmbeddingResponse = response.json().await?;

            // Estimate tokens for Ollama (since it doesn't return usage)
            // Use a conservative estimate of 1 token per 4 characters
            let total_chars: usize = texts.iter().map(|s| s.len()).sum();
            let estimated_tokens = (total_chars / 4) as u64;

            self.metrics
                .total_tokens
                .fetch_add(estimated_tokens, Ordering::Relaxed);

            ollama_response.embeddings
        } else {
            // Parse OpenAI-format response
            let openai_response: EmbeddingResponse = response.json().await?;

            // Update metrics with actual token usage
            self.metrics
                .total_tokens
                .fetch_add(openai_response.usage.total_tokens, Ordering::Relaxed);

            // Sort by index to ensure correct order
            let mut embeddings: Vec<_> = openai_response.data.into_iter().collect();
            embeddings.sort_by_key(|d| d.index);

            embeddings.into_iter().map(|d| d.embedding).collect()
        };

        // Validate all embeddings have the expected dimension (contract guarantee)
        let expected_dim = self.config.dimension;
        for embedding in embeddings.iter() {
            if embedding.len() != expected_dim {
                use crate::embedding::error::DimensionMismatchError;
                return Err(EmbeddingError::DimensionMismatch(
                    DimensionMismatchError::new(
                        expected_dim,
                        embedding.len(),
                        format!("{:?}", self.config.provider),
                        self.config.model.clone(),
                        self.config.dimension,
                    ),
                ));
            }
        }

        Ok(embeddings)
    }

    /// Handle error responses from the API.
    async fn handle_error_response(
        &self,
        status: StatusCode,
        response: reqwest::Response,
    ) -> EmbeddingError {
        use crate::embedding::config::Provider;

        // Try to parse error response
        let error_detail = response
            .json::<ErrorResponse>()
            .await
            .ok()
            .map(|e| e.error.message)
            .unwrap_or_else(|| "Unknown error".to_string());

        // Include provider context in error messages
        let provider_name = match self.config.provider {
            Provider::OpenAI => "OpenAI",
            Provider::Ollama => "Ollama",
            Provider::Cohere => "Cohere",
            Provider::Google => "Google",
            Provider::Local => "Local",
        };

        let api_error = match status {
            StatusCode::UNAUTHORIZED => {
                ApiError::Authentication(format!("{} API: {}", provider_name, error_detail))
            }
            StatusCode::BAD_REQUEST => {
                ApiError::BadRequest(format!("{} API: {}", provider_name, error_detail))
            }
            StatusCode::TOO_MANY_REQUESTS => {
                // Default to 1 second if no retry-after header
                ApiError::RateLimit {
                    retry_after_ms: 1000,
                }
            }
            StatusCode::FORBIDDEN => {
                if error_detail.to_lowercase().contains("quota") {
                    ApiError::QuotaExceeded(format!("{} API: {}", provider_name, error_detail))
                } else {
                    ApiError::Authentication(format!("{} API: {}", provider_name, error_detail))
                }
            }
            StatusCode::SERVICE_UNAVAILABLE => {
                ApiError::ModelUnavailable(format!("{} API: {}", provider_name, error_detail))
            }
            _ if status.is_server_error() => ApiError::ServerError {
                status: status.as_u16(),
                message: format!("{} API: {}", provider_name, error_detail),
            },
            _ => ApiError::InvalidResponse(format!(
                "{} API - HTTP {}: {}",
                provider_name, status, error_detail
            )),
        };

        EmbeddingError::Api(api_error)
    }

    /// Embed a single text.
    pub async fn embed_text(&self, text: String) -> Result<Vector, EmbeddingError> {
        let results = self.embed_batch(vec![text]).await?;
        results
            .into_iter()
            .next()
            .ok_or_else(|| EmbeddingError::Other("No embedding returned".to_string()))
    }

    /// Get cost tracking metrics.
    pub fn metrics(&self) -> &CostMetrics {
        &self.metrics
    }

    /// Get configuration.
    pub fn config(&self) -> &EmbeddingConfig {
        &self.config
    }

    /// Generate embeddings for a batch with parallel sub-batching for improved throughput.
    ///
    /// This method splits the input batch into smaller sub-batches and processes them
    /// concurrently to maximize throughput. This is particularly effective for Ollama
    /// which processes sequential items in a batch one-by-one on the server side.
    ///
    /// # Arguments
    /// * `texts` - Input texts to embed
    /// * `sub_batch_size` - Size of each concurrent sub-batch (default: 25)
    /// * `max_concurrency` - Maximum concurrent requests (default: 4)
    pub async fn embed_batch_parallel(
        &self,
        texts: Vec<String>,
        sub_batch_size: Option<usize>,
        max_concurrency: Option<usize>,
    ) -> Result<Vec<Vector>, EmbeddingError> {
        if texts.is_empty() {
            return Ok(Vec::new());
        }

        let sub_batch_size = sub_batch_size.unwrap_or(25);
        let max_concurrency = max_concurrency.unwrap_or(4);

        // If the batch is small enough, just process it directly
        if texts.len() <= sub_batch_size {
            return self.embed_batch(texts).await;
        }

        debug!(
            "Processing {} texts with parallel batching (sub_batch_size: {}, concurrency: {})",
            texts.len(),
            sub_batch_size,
            max_concurrency
        );

        // Split into sub-batches
        let sub_batches: Vec<Vec<String>> = texts
            .chunks(sub_batch_size)
            .map(|chunk| chunk.to_vec())
            .collect();

        // Process sub-batches with controlled concurrency using semaphore
        use tokio::sync::Semaphore;
        let semaphore = Arc::new(Semaphore::new(max_concurrency));
        let mut handles = Vec::new();

        for (batch_idx, sub_batch) in sub_batches.into_iter().enumerate() {
            let client = self.clone();
            let permit = semaphore
                .clone()
                .acquire_owned()
                .await
                .map_err(|e| EmbeddingError::Other(format!("Semaphore error: {}", e)))?;

            let handle = tokio::spawn(async move {
                let result = client.embed_batch(sub_batch).await;
                drop(permit); // Release semaphore
                (batch_idx, result)
            });

            handles.push(handle);
        }

        // Collect results in order
        let mut results: Vec<(usize, Result<Vec<Vector>, EmbeddingError>)> = Vec::new();
        for handle in handles {
            let (idx, result) = handle
                .await
                .map_err(|e| EmbeddingError::Other(format!("Task join error: {}", e)))?;
            results.push((idx, result));
        }

        // Sort by batch index to maintain order
        results.sort_by_key(|(idx, _)| *idx);

        // Flatten results
        let mut all_embeddings = Vec::new();
        for (_, result) in results {
            let embeddings = result?;
            all_embeddings.extend(embeddings);
        }

        Ok(all_embeddings)
    }

    /// Clone the client (shares the underlying HTTP client and config)
    fn clone(&self) -> Self {
        Self {
            client: self.client.clone(),
            config: self.config.clone(),
            metrics: self.metrics.clone(),
        }
    }
}

/// Implement EmbeddingProvider trait for OpenAIClient.
///
/// This implementation wraps the existing OpenAIClient methods to conform to the
/// EmbeddingProvider trait interface. All methods delegate to the existing
/// implementations to preserve backward compatibility and maintain the existing
/// retry logic, caching, and error handling.
#[async_trait]
impl EmbeddingProvider for OpenAIClient {
    /// Generate embedding for a single text.
    ///
    /// Delegates to the existing `embed_text()` method and converts errors
    /// to the trait's EmbeddingError type.
    async fn embed(&self, text: String) -> Result<Vector, EmbeddingError> {
        self.embed_text(text).await
    }

    /// Generate embeddings for a batch of texts.
    ///
    /// Delegates to the existing `embed_batch()` method which implements
    /// efficient batching, retry logic, and error handling.
    async fn embed_batch(&self, texts: Vec<String>) -> Result<Vec<Vector>, EmbeddingError> {
        self.embed_batch(texts).await
    }

    /// Get the embedding dimension for this provider.
    ///
    /// Returns 1536 for OpenAI's text-embedding-3-small model.
    fn dimension(&self) -> usize {
        self.config.dimension
    }

    /// Get the provider name.
    ///
    /// Returns "openai" as the identifier for this provider.
    fn provider_name(&self) -> &'static str {
        "openai"
    }

    /// Get provider-specific metrics.
    ///
    /// Returns cost tracking data including total requests, tokens processed,
    /// failed requests, and estimated cost in USD.
    fn metrics(&self) -> Option<ProviderMetrics> {
        Some(ProviderMetrics {
            total_requests: self.metrics.total_requests(),
            total_tokens: self.metrics.total_tokens(),
            failed_requests: self.metrics.failed_requests(),
            estimated_cost_usd: self.metrics.estimated_cost_usd(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::embedding::config::{CacheConfig, ParallelConfig, Provider, RetryConfig};

    fn test_config() -> EmbeddingConfig {
        EmbeddingConfig {
            provider: Provider::OpenAI,
            model: "text-embedding-3-small".to_string(),
            dimension: 1536,
            cache: CacheConfig::default(),
            batch_size: 100,
            retry: RetryConfig::default(),
            api_key: Some("test-key".to_string()),
            api_endpoint: None,
            parallel: ParallelConfig::default(),
        }
    }

    #[test]
    fn test_client_creation() {
        let config = test_config();
        let client = OpenAIClient::new(config);
        assert!(client.is_ok());
    }

    #[test]
    fn test_client_creation_without_api_key() {
        let mut config = test_config();
        config.api_key = None;
        config.provider = Provider::Local; // Local provider doesn't need API key
        let client = OpenAIClient::new(config);
        assert!(client.is_ok());
    }

    #[test]
    fn test_ollama_client_creation() {
        let config = EmbeddingConfig {
            provider: Provider::Ollama,
            model: "nomic-embed-text".to_string(),
            dimension: 768,
            cache: CacheConfig::default(),
            batch_size: 100,
            retry: RetryConfig::default(),
            api_key: None, // Ollama doesn't need API key
            api_endpoint: None,
            parallel: ParallelConfig::default(),
        };
        let client = OpenAIClient::new(config);
        assert!(client.is_ok());

        let client = client.unwrap();
        assert_eq!(client.config().provider, Provider::Ollama);
        assert_eq!(client.config().model, "nomic-embed-text");
        assert_eq!(
            client.config().api_endpoint_url(),
            "http://localhost:11434/api/embed"
        );
    }

    #[test]
    fn test_cost_metrics() {
        let metrics = CostMetrics::default();

        metrics.total_tokens.store(1_000_000, Ordering::Relaxed);
        assert_eq!(metrics.total_tokens(), 1_000_000);
        assert_eq!(metrics.estimated_cost_usd(), 0.02);

        metrics.total_tokens.store(500_000, Ordering::Relaxed);
        assert_eq!(metrics.estimated_cost_usd(), 0.01);

        metrics.reset();
        assert_eq!(metrics.total_tokens(), 0);
        assert_eq!(metrics.estimated_cost_usd(), 0.0);
    }

    #[test]
    fn test_batch_size_validation() {
        let config = test_config();
        let client = OpenAIClient::new(config).unwrap();

        let large_batch: Vec<String> = (0..200).map(|i| format!("text {}", i)).collect();

        // This would fail async, but we can test the error type
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let result = client.embed_batch(large_batch).await;
            assert!(result.is_err());
            if let Err(EmbeddingError::InvalidInput(msg)) = result {
                assert!(msg.contains("exceeds maximum"));
            } else {
                panic!("Expected InvalidInput error");
            }
        });
    }

    #[test]
    fn test_empty_batch() {
        let config = test_config();
        let client = OpenAIClient::new(config).unwrap();

        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let result = client.embed_batch(vec![]).await;
            assert!(result.is_ok());
            assert_eq!(result.unwrap().len(), 0);
        });
    }

    #[test]
    fn test_error_response_parsing() {
        let error_json = r#"{
            "error": {
                "message": "Invalid API key",
                "type": "invalid_request_error",
                "code": "invalid_api_key"
            }
        }"#;

        let error: ErrorResponse = serde_json::from_str(error_json).unwrap();
        assert_eq!(error.error.message, "Invalid API key");
        assert_eq!(error.error.error_type, "invalid_request_error");
        assert_eq!(error.error.code, Some("invalid_api_key".to_string()));
    }

    #[tokio::test]
    async fn test_metrics_tracking() {
        let config = test_config();
        let client = OpenAIClient::new(config).unwrap();

        // Initial metrics should be zero
        assert_eq!(client.metrics().total_requests(), 0);
        assert_eq!(client.metrics().total_tokens(), 0);

        // Test metrics reset
        client.metrics().total_requests.store(10, Ordering::Relaxed);
        client.metrics().total_tokens.store(5000, Ordering::Relaxed);

        assert_eq!(client.metrics().total_requests(), 10);
        assert_eq!(client.metrics().total_tokens(), 5000);

        client.metrics().reset();
        assert_eq!(client.metrics().total_requests(), 0);
        assert_eq!(client.metrics().total_tokens(), 0);
    }

    #[test]
    fn test_embedding_provider_trait_implementation() {
        let config = test_config();
        let client = OpenAIClient::new(config).unwrap();

        // Test provider_name() through trait
        assert_eq!(EmbeddingProvider::provider_name(&client), "openai");

        // Test dimension() through trait
        assert_eq!(EmbeddingProvider::dimension(&client), 1536);

        // Test metrics() through trait returns Some
        let metrics = EmbeddingProvider::metrics(&client);
        assert!(metrics.is_some());
        let metrics = metrics.unwrap();
        assert_eq!(metrics.total_requests, 0);
        assert_eq!(metrics.total_tokens, 0);
        assert_eq!(metrics.failed_requests, 0);
        assert_eq!(metrics.estimated_cost_usd, 0.0);
    }

    #[tokio::test]
    async fn test_embedding_provider_trait_object() {
        let config = test_config();
        let client = OpenAIClient::new(config).unwrap();

        // Test that OpenAIClient can be used as a trait object
        let provider: Box<dyn EmbeddingProvider> = Box::new(client);

        assert_eq!(provider.provider_name(), "openai");
        assert_eq!(provider.dimension(), 1536);

        // Test empty batch through trait
        let result = provider.embed_batch(vec![]).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 0);
    }

    #[test]
    fn test_provider_metrics_conversion() {
        let config = test_config();
        let client = OpenAIClient::new(config).unwrap();

        // Set some metrics
        client.metrics.total_requests.store(100, Ordering::Relaxed);
        client.metrics.total_tokens.store(50000, Ordering::Relaxed);
        client.metrics.failed_requests.store(5, Ordering::Relaxed);

        // Get metrics through trait
        let provider_metrics = EmbeddingProvider::metrics(&client).unwrap();

        assert_eq!(provider_metrics.total_requests, 100);
        assert_eq!(provider_metrics.total_tokens, 50000);
        assert_eq!(provider_metrics.failed_requests, 5);
        assert_eq!(provider_metrics.estimated_cost_usd, 0.001); // 50000 tokens * $0.02 / 1M
    }
}
