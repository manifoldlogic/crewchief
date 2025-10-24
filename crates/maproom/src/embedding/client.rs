//! OpenAI API client with retry logic and error handling.

use crate::embedding::cache::Vector;
use crate::embedding::config::EmbeddingConfig;
use crate::embedding::error::{ApiError, EmbeddingError};
use reqwest::{Client, StatusCode};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
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

/// OpenAI API request structure.
#[derive(Debug, Serialize)]
struct EmbeddingRequest {
    input: Vec<String>,
    model: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    dimensions: Option<usize>,
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
            .build()
            .map_err(|e| EmbeddingError::Network(e))?;

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
                Err(EmbeddingError::Api(api_err)) if api_err.is_retryable() && attempt < max_attempts - 1 => {
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
        let api_key = self.config.api_key.as_ref().ok_or_else(|| {
            EmbeddingError::Config(crate::embedding::error::ConfigError::MissingConfig(
                "API key".to_string(),
            ))
        })?;

        let request = EmbeddingRequest {
            input: texts.to_vec(),
            model: self.config.model.clone(),
            dimensions: Some(self.config.dimension),
        };

        let response = self
            .client
            .post(&self.config.api_endpoint_url())
            .header("Authorization", format!("Bearer {}", api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await?;

        self.metrics.total_requests.fetch_add(1, Ordering::Relaxed);

        let status = response.status();

        if !status.is_success() {
            return Err(self.handle_error_response(status, response).await);
        }

        let response: EmbeddingResponse = response.json().await?;

        // Update metrics
        self.metrics
            .total_tokens
            .fetch_add(response.usage.total_tokens, Ordering::Relaxed);

        // Sort by index to ensure correct order
        let mut embeddings: Vec<_> = response.data.into_iter().collect();
        embeddings.sort_by_key(|d| d.index);

        Ok(embeddings.into_iter().map(|d| d.embedding).collect())
    }

    /// Handle error responses from the API.
    async fn handle_error_response(
        &self,
        status: StatusCode,
        response: reqwest::Response,
    ) -> EmbeddingError {
        // Try to parse error response
        let error_detail = response
            .json::<ErrorResponse>()
            .await
            .ok()
            .map(|e| e.error.message)
            .unwrap_or_else(|| "Unknown error".to_string());

        let api_error = match status {
            StatusCode::UNAUTHORIZED => ApiError::Authentication(error_detail),
            StatusCode::BAD_REQUEST => ApiError::BadRequest(error_detail),
            StatusCode::TOO_MANY_REQUESTS => {
                // Default to 1 second if no retry-after header
                ApiError::RateLimit {
                    retry_after_ms: 1000,
                }
            }
            StatusCode::FORBIDDEN => {
                if error_detail.to_lowercase().contains("quota") {
                    ApiError::QuotaExceeded(error_detail)
                } else {
                    ApiError::Authentication(error_detail)
                }
            }
            StatusCode::SERVICE_UNAVAILABLE => ApiError::ModelUnavailable(error_detail),
            _ if status.is_server_error() => ApiError::ServerError {
                status: status.as_u16(),
                message: error_detail,
            },
            _ => ApiError::InvalidResponse(format!("HTTP {}: {}", status, error_detail)),
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::embedding::config::{CacheConfig, Provider, RetryConfig};

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
}
