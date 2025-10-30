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
    /// Text to embed
    input: String,
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
/// using the nomic-embed-text model (768 dimensions). It uses concurrent requests
/// with semaphore limiting to avoid overwhelming the local Ollama instance.
///
/// # Configuration
///
/// - **Endpoint**: Default `http://localhost:11434/api/embed`
/// - **Model**: Default `nomic-embed-text`
/// - **Timeout**: 30 seconds per request
/// - **Concurrency**: Max 10 simultaneous requests
///
/// # Thread Safety
///
/// This provider is `Clone` and can be safely shared across async tasks.
/// The internal semaphore ensures concurrency limits are respected.
#[derive(Clone)]
pub struct OllamaProvider {
    /// HTTP client for making requests
    client: Client,
    /// Ollama API endpoint URL
    endpoint: String,
    /// Model name (e.g., "nomic-embed-text")
    model: String,
    /// Semaphore to limit concurrent requests
    semaphore: Arc<Semaphore>,
}

impl OllamaProvider {
    /// Default endpoint for Ollama embedding API.
    pub const DEFAULT_ENDPOINT: &'static str = "http://localhost:11434/api/embed";

    /// Default model for embeddings.
    pub const DEFAULT_MODEL: &'static str = "nomic-embed-text";

    /// Maximum concurrent requests to avoid overwhelming Ollama.
    const MAX_CONCURRENT_REQUESTS: usize = 10;

    /// Request timeout in seconds.
    const REQUEST_TIMEOUT_SECS: u64 = 30;

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

    /// Make a single embedding request with retry logic.
    ///
    /// This method implements retry logic for transient failures such as
    /// temporary Ollama overload or network issues.
    async fn embed_with_retry(&self, text: String) -> Result<Vector, EmbeddingError> {
        const MAX_RETRIES: u32 = 3;
        const RETRY_DELAY_MS: u64 = 1000;

        let mut last_error = None;

        for attempt in 0..MAX_RETRIES {
            match self.embed_single(&text).await {
                Ok(vector) => return Ok(vector),
                Err(e) => {
                    // Check if error is retryable
                    let should_retry = match &e {
                        EmbeddingError::Network(_) => true,
                        EmbeddingError::Api(api_err) => api_err.is_retryable(),
                        _ => false,
                    };

                    if !should_retry || attempt == MAX_RETRIES - 1 {
                        return Err(e);
                    }

                    last_error = Some(e);
                    tokio::time::sleep(std::time::Duration::from_millis(
                        RETRY_DELAY_MS * (attempt as u64 + 1),
                    ))
                    .await;
                }
            }
        }

        Err(last_error.unwrap_or_else(|| {
            EmbeddingError::Other("All retry attempts failed".to_string())
        }))
    }

    /// Make a single embedding request without retry.
    async fn embed_single(&self, text: &str) -> Result<Vector, EmbeddingError> {
        let response = self
            .client
            .post(&self.endpoint)
            .json(&OllamaRequest {
                model: self.model.clone(),
                input: text.to_string(),
            })
            .send()
            .await?;

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
                    message: error_msg,
                },
                401 => ApiError::Authentication(error_msg),
                400 => ApiError::BadRequest(error_msg),
                _ => ApiError::InvalidResponse(format!("HTTP {}: {}", status, error_msg)),
            }));
        }

        let body: OllamaResponse = response.json().await?;

        if body.embeddings.is_empty() {
            return Err(EmbeddingError::Api(ApiError::InvalidResponse(
                "Empty embeddings array in response".to_string(),
            )));
        }

        let embedding = &body.embeddings[0];
        let expected_dim = self.dimension();

        // Validate dimension matches expected value (contract guarantee)
        if embedding.len() != expected_dim {
            return Err(EmbeddingError::Api(ApiError::InvalidResponse(
                format!(
                    "Dimension mismatch: expected {} dimensions but got {}",
                    expected_dim,
                    embedding.len()
                ),
            )));
        }

        Ok(embedding.clone())
    }
}

#[async_trait]
impl EmbeddingProvider for OllamaProvider {
    /// Generate embedding vector for a single text.
    ///
    /// This method calls the Ollama API to generate a 768-dimensional embedding
    /// vector for the input text. It includes retry logic for transient failures.
    ///
    /// # Arguments
    ///
    /// * `text` - The text to embed
    ///
    /// # Returns
    ///
    /// - `Ok(Vector)` - 768-dimensional embedding vector
    /// - `Err(EmbeddingError)` - If the API call fails after retries
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
        self.embed_with_retry(text).await
    }

    /// Generate embeddings for a batch of texts.
    ///
    /// Since Ollama doesn't support native batching, this method uses concurrent
    /// requests with semaphore limiting to process multiple texts efficiently
    /// while avoiding overwhelming the local Ollama instance.
    ///
    /// # Arguments
    ///
    /// * `texts` - Vector of texts to embed
    ///
    /// # Returns
    ///
    /// - `Ok(Vec<Vector>)` - Vector of 768-dimensional embeddings (same length as input)
    /// - `Err(EmbeddingError)` - If any embedding fails
    ///
    /// # Concurrency
    ///
    /// This method limits concurrent requests to 10 to avoid overwhelming Ollama.
    /// The semaphore ensures this limit is respected even when processing large batches.
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
        if texts.is_empty() {
            return Ok(Vec::new());
        }

        // Spawn concurrent tasks with semaphore limiting
        let mut tasks = Vec::with_capacity(texts.len());

        for text in texts {
            let provider = self.clone();
            let semaphore = self.semaphore.clone();

            tasks.push(tokio::spawn(async move {
                // Acquire permit before making request
                let _permit = semaphore.acquire().await.map_err(|e| {
                    EmbeddingError::Other(format!("Failed to acquire semaphore: {}", e))
                })?;

                // Make request (permit automatically released when dropped)
                provider.embed(text).await
            }));
        }

        // Collect results
        let mut results = Vec::with_capacity(tasks.len());
        for task in tasks {
            let result = task.await.map_err(|e| {
                EmbeddingError::Other(format!("Task join error: {}", e))
            })??;
            results.push(result);
        }

        Ok(results)
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
    fn test_ollama_request_serialization() {
        let request = OllamaRequest {
            model: "nomic-embed-text".to_string(),
            input: "test text".to_string(),
        };

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("nomic-embed-text"));
        assert!(json.contains("test text"));
    }

    #[test]
    fn test_ollama_response_deserialization() {
        let json = r#"{"embeddings":[[0.1,0.2,0.3]]}"#;
        let response: OllamaResponse = serde_json::from_str(json).unwrap();

        assert_eq!(response.embeddings.len(), 1);
        assert_eq!(response.embeddings[0].len(), 3);
        assert_eq!(response.embeddings[0][0], 0.1);
    }

    #[tokio::test]
    async fn test_embed_batch_empty_input() {
        let provider = OllamaProvider::default_config().unwrap();
        let result = provider.embed_batch(vec![]).await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 0);
    }
}
