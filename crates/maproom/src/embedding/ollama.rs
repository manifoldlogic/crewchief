//! Ollama embedding provider implementation.
//!
//! This module provides integration with Ollama for local embedding generation.
//! Ollama runs locally at `http://localhost:11434` with the nomic-embed-text model,
//! which produces 768-dimensional embeddings.
//!
//! # Features
//!
//! - Zero-config local embeddings (no API keys required)
//! - 768-dimensional vectors (nomic-embed-text model)
//! - Concurrent batch processing with semaphore limiting
//! - Retry logic for transient failures
//! - Configurable endpoint and model
//!
//! # Examples
//!
//! ```no_run
//! use crewchief_maproom::embedding::ollama::OllamaProvider;
//! use crewchief_maproom::embedding::provider::EmbeddingProvider;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Create provider with default settings
//!     let provider = OllamaProvider::new(
//!         "http://localhost:11434/api/embed".to_string(),
//!         "nomic-embed-text".to_string()
//!     )?;
//!
//!     // Generate single embedding
//!     let embedding = provider.embed("Hello, world!".to_string()).await?;
//!     assert_eq!(embedding.len(), 768);
//!
//!     // Generate batch with concurrent requests
//!     let texts = vec!["First".to_string(), "Second".to_string()];
//!     let embeddings = provider.embed_batch(texts).await?;
//!     assert_eq!(embeddings.len(), 2);
//!
//!     Ok(())
//! }
//! ```

use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::Semaphore;

use crate::embedding::error::{ApiError, EmbeddingError};
use crate::embedding::provider::{EmbeddingProvider, Vector};

/// Request payload for Ollama embedding API.
#[derive(Serialize)]
struct OllamaRequest {
    /// Model name (e.g., "nomic-embed-text")
    model: String,
    /// Texts to embed (batch API format)
    input: Vec<String>,
}

/// Response payload from Ollama embedding API.
#[derive(Deserialize)]
struct OllamaResponse {
    /// Array of embedding vectors (typically single element for single input)
    embeddings: Vec<Vec<f32>>,
}

/// Ollama embedding provider for local embeddings.
///
/// This provider integrates with Ollama running locally to generate embeddings
/// using the nomic-embed-text model (768 dimensions). It uses Ollama's batch API
/// to send multiple texts in a single HTTP request for improved performance.
///
/// # Configuration
///
/// - **Endpoint**: Default `http://localhost:11434/api/embed`
/// - **Model**: Default `nomic-embed-text`
/// - **Timeout**: 60 seconds per request
///
/// # Thread Safety
///
/// This provider is `Clone` and can be safely shared across async tasks.
#[derive(Clone)]
pub struct OllamaProvider {
    /// HTTP client for making requests
    client: Client,
    /// Ollama API endpoint URL
    endpoint: String,
    /// Model name (e.g., "nomic-embed-text")
    model: String,
    /// Semaphore to limit concurrent requests (reserved for future use)
    #[allow(dead_code)]
    semaphore: Arc<Semaphore>,
}

impl OllamaProvider {
    /// Default endpoint for Ollama embedding API.
    pub const DEFAULT_ENDPOINT: &'static str = "http://localhost:11434/api/embed";

    /// Default model for embeddings.
    pub const DEFAULT_MODEL: &'static str = "nomic-embed-text";

    /// Maximum concurrent requests to avoid overwhelming Ollama.
    const MAX_CONCURRENT_REQUESTS: usize = 10;

    /// Request timeout in seconds (increased for larger batches).
    const REQUEST_TIMEOUT_SECS: u64 = 60;

    /// Create a new OllamaProvider with specified endpoint and model.
    ///
    /// # Arguments
    ///
    /// * `endpoint` - Ollama API endpoint URL (e.g., "http://localhost:11434/api/embed")
    /// * `model` - Model name (e.g., "nomic-embed-text")
    ///
    /// # Returns
    ///
    /// - `Ok(OllamaProvider)` - Successfully created provider
    /// - `Err(EmbeddingError)` - If HTTP client creation fails
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use crewchief_maproom::embedding::ollama::OllamaProvider;
    ///
    /// let provider = OllamaProvider::new(
    ///     "http://localhost:11434/api/embed".to_string(),
    ///     "nomic-embed-text".to_string()
    /// )?;
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn new(endpoint: String, model: String) -> Result<Self, EmbeddingError> {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(Self::REQUEST_TIMEOUT_SECS))
            .build()?;

        Ok(Self {
            client,
            endpoint,
            model,
            semaphore: Arc::new(Semaphore::new(Self::MAX_CONCURRENT_REQUESTS)),
        })
    }

    /// Create a new OllamaProvider with default settings.
    ///
    /// Uses default endpoint (`http://localhost:11434/api/embed`) and model (`nomic-embed-text`).
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use crewchief_maproom::embedding::ollama::OllamaProvider;
    ///
    /// let provider = OllamaProvider::default_config()?;
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn default_config() -> Result<Self, EmbeddingError> {
        Self::new(
            Self::DEFAULT_ENDPOINT.to_string(),
            Self::DEFAULT_MODEL.to_string(),
        )
    }

    /// Embed a batch of texts using a single HTTP request.
    ///
    /// This method uses Ollama's batch API to send multiple texts in one request,
    /// significantly reducing HTTP overhead compared to individual requests.
    ///
    /// # Arguments
    ///
    /// * `texts` - Vector of texts to embed
    ///
    /// # Returns
    ///
    /// - `Ok(Vec<Vector>)` - Vector of embeddings (same length as input)
    /// - `Err(EmbeddingError)` - If the API call fails
    ///
    /// # Error Handling
    ///
    /// This method does NOT fall back to single-text requests on failure.
    /// Failures return an error with context including batch size for debugging.
    async fn embed_batch_raw(&self, texts: Vec<String>) -> Result<Vec<Vector>, EmbeddingError> {
        if texts.is_empty() {
            return Ok(Vec::new());
        }

        let batch_size = texts.len();

        let response = self
            .client
            .post(&self.endpoint)
            .json(&OllamaRequest {
                model: self.model.clone(),
                input: texts,
            })
            .send()
            .await
            .map_err(|e| {
                tracing::error!("Failed to send batch of {} texts: {}", batch_size, e);
                EmbeddingError::Network(e)
            })?;

        let status = response.status();
        if !status.is_success() {
            let error_msg = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());

            return Err(EmbeddingError::Api(match status.as_u16() {
                429 => ApiError::RateLimit {
                    retry_after_ms: 1000,
                },
                500..=599 => ApiError::ServerError {
                    status: status.as_u16(),
                    message: format!("Batch of {} texts failed: {}", batch_size, error_msg),
                },
                401 => ApiError::Authentication(error_msg),
                400 => ApiError::BadRequest(format!(
                    "Batch of {} texts rejected: {}",
                    batch_size, error_msg
                )),
                _ => ApiError::InvalidResponse(format!(
                    "HTTP {} for batch of {} texts: {}",
                    status, batch_size, error_msg
                )),
            }));
        }

        let body: OllamaResponse = response.json().await.map_err(|e| {
            EmbeddingError::Api(ApiError::InvalidResponse(format!(
                "Failed to parse batch response for {} texts: {}",
                batch_size, e
            )))
        })?;

        // Validate response has expected number of embeddings
        if body.embeddings.len() != batch_size {
            return Err(EmbeddingError::Api(ApiError::InvalidResponse(format!(
                "Batch size mismatch: sent {} texts but got {} embeddings",
                batch_size,
                body.embeddings.len()
            ))));
        }

        let expected_dim = self.dimension();

        // Validate all embeddings have correct dimension
        for (i, embedding) in body.embeddings.iter().enumerate() {
            if embedding.len() != expected_dim {
                return Err(EmbeddingError::Api(ApiError::InvalidResponse(format!(
                    "Dimension mismatch in batch at index {}: expected {} dimensions but got {}",
                    i,
                    expected_dim,
                    embedding.len()
                ))));
            }
        }

        Ok(body.embeddings)
    }
}

#[async_trait]
impl EmbeddingProvider for OllamaProvider {
    /// Generate embedding vector for a single text.
    ///
    /// This method calls the Ollama API to generate a 768-dimensional embedding
    /// vector for the input text. Internally, it wraps the text in a batch of one
    /// to use Ollama's batch API endpoint.
    ///
    /// # Arguments
    ///
    /// * `text` - The text to embed
    ///
    /// # Returns
    ///
    /// - `Ok(Vector)` - 768-dimensional embedding vector
    /// - `Err(EmbeddingError)` - If the API call fails
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use crewchief_maproom::embedding::ollama::OllamaProvider;
    /// # use crewchief_maproom::embedding::provider::EmbeddingProvider;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let provider = OllamaProvider::default_config()?;
    /// let embedding = provider.embed("Hello, world!".to_string()).await?;
    /// assert_eq!(embedding.len(), 768);
    /// # Ok(())
    /// # }
    /// ```
    async fn embed(&self, text: String) -> Result<Vector, EmbeddingError> {
        let embeddings = self.embed_batch_raw(vec![text]).await?;
        // Safe to unwrap because we validated response length in embed_batch_raw
        Ok(embeddings.into_iter().next().unwrap())
    }

    /// Generate embeddings for a batch of texts.
    ///
    /// This method uses Ollama's batch API to send all texts in a single HTTP request,
    /// providing significant performance improvements over individual requests.
    ///
    /// # Arguments
    ///
    /// * `texts` - Vector of texts to embed
    ///
    /// # Returns
    ///
    /// - `Ok(Vec<Vector>)` - Vector of 768-dimensional embeddings (same length as input)
    /// - `Err(EmbeddingError)` - If the batch request fails
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use crewchief_maproom::embedding::ollama::OllamaProvider;
    /// # use crewchief_maproom::embedding::provider::EmbeddingProvider;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let provider = OllamaProvider::default_config()?;
    /// let texts = vec!["First".to_string(), "Second".to_string()];
    /// let embeddings = provider.embed_batch(texts).await?;
    /// assert_eq!(embeddings.len(), 2);
    /// # Ok(())
    /// # }
    /// ```
    async fn embed_batch(&self, texts: Vec<String>) -> Result<Vec<Vector>, EmbeddingError> {
        self.embed_batch_raw(texts).await
    }

    /// Get the embedding dimension for this provider.
    ///
    /// Ollama's nomic-embed-text model produces 768-dimensional embeddings.
    ///
    /// # Returns
    ///
    /// Always returns 768.
    fn dimension(&self) -> usize {
        768 // nomic-embed-text fixed dimension
    }

    /// Get the provider name identifier.
    ///
    /// # Returns
    ///
    /// Always returns "ollama".
    fn provider_name(&self) -> &'static str {
        "ollama"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ollama_provider_creation() {
        let provider = OllamaProvider::new(
            "http://localhost:11434/api/embed".to_string(),
            "nomic-embed-text".to_string(),
        );
        assert!(provider.is_ok());

        let provider = provider.unwrap();
        assert_eq!(provider.dimension(), 768);
        assert_eq!(provider.provider_name(), "ollama");
    }

    #[test]
    fn test_ollama_provider_default_config() {
        let provider = OllamaProvider::default_config();
        assert!(provider.is_ok());

        let provider = provider.unwrap();
        assert_eq!(provider.endpoint, OllamaProvider::DEFAULT_ENDPOINT);
        assert_eq!(provider.model, OllamaProvider::DEFAULT_MODEL);
    }

    #[test]
    fn test_ollama_provider_clone() {
        let provider = OllamaProvider::default_config().unwrap();
        let cloned = provider.clone();

        assert_eq!(provider.dimension(), cloned.dimension());
        assert_eq!(provider.provider_name(), cloned.provider_name());
    }

    #[test]
    fn test_ollama_request_serialization_single() {
        let request = OllamaRequest {
            model: "nomic-embed-text".to_string(),
            input: vec!["test text".to_string()],
        };

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("nomic-embed-text"));
        assert!(json.contains("test text"));
        assert!(json.contains("\"input\":["));
    }

    #[test]
    fn test_ollama_request_serialization_batch() {
        let request = OllamaRequest {
            model: "nomic-embed-text".to_string(),
            input: vec!["text1".to_string(), "text2".to_string()],
        };

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("\"model\":\"nomic-embed-text\""));
        assert!(json.contains("\"input\":[\"text1\",\"text2\"]"));
    }

    #[test]
    fn test_ollama_response_deserialization_single() {
        let json = r#"{"embeddings":[[0.1,0.2,0.3]]}"#;
        let response: OllamaResponse = serde_json::from_str(json).unwrap();

        assert_eq!(response.embeddings.len(), 1);
        assert_eq!(response.embeddings[0].len(), 3);
        assert_eq!(response.embeddings[0][0], 0.1);
    }

    #[test]
    fn test_ollama_response_deserialization_batch() {
        let json = r#"{"embeddings":[[0.1,0.2],[0.3,0.4]]}"#;
        let response: OllamaResponse = serde_json::from_str(json).unwrap();

        assert_eq!(response.embeddings.len(), 2);
        assert_eq!(response.embeddings[0].len(), 2);
        assert_eq!(response.embeddings[1].len(), 2);
        assert_eq!(response.embeddings[0][0], 0.1);
        assert_eq!(response.embeddings[0][1], 0.2);
        assert_eq!(response.embeddings[1][0], 0.3);
        assert_eq!(response.embeddings[1][1], 0.4);
    }

    #[tokio::test]
    async fn test_embed_batch_empty_input() {
        let provider = OllamaProvider::default_config().unwrap();
        let result = provider.embed_batch(vec![]).await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 0);
    }

    #[tokio::test]
    async fn test_embed_batch_raw_empty_returns_empty() {
        let provider = OllamaProvider::default_config().unwrap();
        let result = provider.embed_batch_raw(vec![]).await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 0);
    }

    #[tokio::test]
    #[ignore] // Requires running Ollama
    async fn test_ollama_batch_api_integration() {
        let provider = OllamaProvider::default_config().unwrap();
        let texts = vec!["hello".to_string(), "world".to_string()];
        let embeddings = provider.embed_batch(texts).await.unwrap();

        assert_eq!(embeddings.len(), 2);
        assert_eq!(embeddings[0].len(), 768);
        assert_eq!(embeddings[1].len(), 768);
    }

    #[tokio::test]
    #[ignore] // Requires running Ollama
    async fn test_ollama_single_embed_uses_batch_api() {
        let provider = OllamaProvider::default_config().unwrap();
        let embedding = provider.embed("test".to_string()).await.unwrap();

        assert_eq!(embedding.len(), 768);
    }
}
