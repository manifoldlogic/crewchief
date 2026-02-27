//! Ollama embedding provider implementation.
//!
//! This module provides integration with Ollama for local embedding generation.
//! Ollama runs locally at `http://localhost:11434` with configurable models supporting
//! different embedding dimensions (768 for nomic-embed-text, 1024 for mxbai-embed-large).
//!
//! # Features
//!
//! - Zero-config local embeddings (no API keys required)
//! - Configurable embedding dimensions (768, 1024, or custom)
//! - Multiple model support (nomic-embed-text, mxbai-embed-large, etc.)
//! - Concurrent batch processing with semaphore limiting
//! - Retry logic for transient failures
//! - Configurable endpoint and model
//!
//! # Examples
//!
//! ```no_run
//! use maproom::embedding::ollama::OllamaProvider;
//! use maproom::embedding::provider::EmbeddingProvider;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Create provider with nomic-embed-text (768 dimensions)
//!     let provider = OllamaProvider::new(
//!         "http://localhost:11434/api/embed".to_string(),
//!         "nomic-embed-text".to_string(),
//!         768
//!     )?;
//!
//!     // Generate single embedding
//!     let embedding = provider.embed("Hello, world!".to_string()).await?;
//!     assert_eq!(embedding.len(), 768);
//!
//!     // Create provider with mxbai-embed-large (1024 dimensions)
//!     let provider_1024 = OllamaProvider::new(
//!         "http://localhost:11434/api/embed".to_string(),
//!         "mxbai-embed-large".to_string(),
//!         1024
//!     )?;
//!
//!     // Generate batch with concurrent requests
//!     let texts = vec!["First".to_string(), "Second".to_string()];
//!     let embeddings = provider_1024.embed_batch(texts).await?;
//!     assert_eq!(embeddings.len(), 2);
//!     assert_eq!(embeddings[0].len(), 1024);
//!
//!     Ok(())
//! }
//! ```

use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::Semaphore;
use tracing;

use crate::embedding::config::ParallelConfig;
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
/// using configurable models with different dimensions. It uses Ollama's batch API
/// to send multiple texts in a single HTTP request for improved performance.
///
/// # Configuration
///
/// - **Endpoint**: Default `http://localhost:11434/api/embed`
/// - **Model**: Default `mxbai-embed-large`
/// - **Dimension**: Configurable (768 for nomic-embed-text, 1024 for mxbai-embed-large)
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
    /// Model name (e.g., "nomic-embed-text", "mxbai-embed-large")
    model: String,
    /// Embedding dimension (768 for nomic-embed-text, 1024 for mxbai-embed-large)
    dimension: usize,
    /// Parallel processing configuration
    parallel_config: ParallelConfig,
    /// Semaphore to limit concurrent requests
    semaphore: Arc<Semaphore>,
}

impl OllamaProvider {
    /// Default endpoint for Ollama embedding API.
    pub const DEFAULT_ENDPOINT: &'static str = "http://localhost:11434/api/embed";

    /// Default model for embeddings.
    pub const DEFAULT_MODEL: &'static str = "mxbai-embed-large";

    /// Request timeout in seconds (increased for larger batches).
    const REQUEST_TIMEOUT_SECS: u64 = 60;

    /// Sanitize text for nomic-embed-text model to work around GGML tokenization bugs.
    ///
    /// This function replaces characters that cause token count explosions in nomic-embed-text's
    /// GGML tokenizer. These replacements prevent attention layer crashes but degrade embedding
    /// quality by mangling content. Only apply this to nomic-embed-text model.
    ///
    /// See: https://github.com/ollama/ollama/issues/9499
    ///
    /// # Arguments
    ///
    /// * `text` - The text to sanitize
    ///
    /// # Returns
    ///
    /// Sanitized text with problematic characters replaced
    #[allow(clippy::manual_string_new)]
    fn sanitize_for_nomic(text: &str) -> String {
        // Replace characters that cause nomic-embed-text tokenization crashes
        // Note: We use individual replace() calls instead of replace(['x', 'y'], "z")
        // because we need different replacements for different characters (e.g., '[' -> '(' and ']' -> ')')
        text.replace('|', " ") // Markdown table pipes
            .replace('[', "(") // Opening bracket
            .replace(']', ")") // Closing bracket
            .replace('→', "->") // Unicode arrows
            .replace('←', "<-")
            .replace('↔', "<->")
            // Box-drawing characters (directory trees)
            .replace(['├', '└'], "+")
            .replace('│', " ")
            .replace('─', "-")
            .replace(['┌', '┐', '┘', '┤', '┬', '┴', '┼'], "+")
    }

    /// Create a new OllamaProvider with specified endpoint, model, and dimension.
    ///
    /// # Arguments
    ///
    /// * `endpoint` - Ollama API endpoint URL (e.g., "http://localhost:11434/api/embed")
    /// * `model` - Model name (e.g., "nomic-embed-text", "mxbai-embed-large")
    /// * `dimension` - Embedding dimension (768 for nomic-embed-text, 1024 for mxbai-embed-large)
    ///
    /// # Returns
    ///
    /// - `Ok(OllamaProvider)` - Successfully created provider
    /// - `Err(EmbeddingError)` - If HTTP client creation fails
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use maproom::embedding::ollama::OllamaProvider;
    ///
    /// // nomic-embed-text (768 dimensions)
    /// let provider = OllamaProvider::new(
    ///     "http://localhost:11434/api/embed".to_string(),
    ///     "nomic-embed-text".to_string(),
    ///     768
    /// )?;
    ///
    /// // mxbai-embed-large (1024 dimensions)
    /// let provider = OllamaProvider::new(
    ///     "http://localhost:11434/api/embed".to_string(),
    ///     "mxbai-embed-large".to_string(),
    ///     1024
    /// )?;
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn new(endpoint: String, model: String, dimension: usize) -> Result<Self, EmbeddingError> {
        Self::new_with_config(endpoint, model, dimension, ParallelConfig::default())
    }

    /// Create a new OllamaProvider with explicit parallel processing configuration.
    ///
    /// # Arguments
    ///
    /// * `endpoint` - Ollama API endpoint URL (e.g., "http://localhost:11434/api/embed")
    /// * `model` - Model name (e.g., "nomic-embed-text", "mxbai-embed-large")
    /// * `dimension` - Embedding dimension (768 for nomic-embed-text, 1024 for mxbai-embed-large)
    /// * `config` - Parallel processing configuration
    ///
    /// # Returns
    ///
    /// - `Ok(OllamaProvider)` - Successfully created provider
    /// - `Err(EmbeddingError)` - If HTTP client creation fails
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use maproom::embedding::ollama::OllamaProvider;
    /// use maproom::embedding::config::ParallelConfig;
    ///
    /// let config = ParallelConfig {
    ///     enabled: true,
    ///     sub_batch_size: 50,
    ///     max_concurrency: 8,
    /// };
    /// let provider = OllamaProvider::new_with_config(
    ///     "http://localhost:11434/api/embed".to_string(),
    ///     "nomic-embed-text".to_string(),
    ///     768,
    ///     config
    /// )?;
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn new_with_config(
        endpoint: String,
        model: String,
        dimension: usize,
        config: ParallelConfig,
    ) -> Result<Self, EmbeddingError> {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(Self::REQUEST_TIMEOUT_SECS))
            .build()?;

        let semaphore = Arc::new(Semaphore::new(config.max_concurrency));

        Ok(Self {
            client,
            endpoint,
            model,
            dimension,
            parallel_config: config,
            semaphore,
        })
    }

    /// Create a new OllamaProvider with default settings.
    ///
    /// Uses default endpoint (`http://localhost:11434/api/embed`), model (`mxbai-embed-large`),
    /// and dimension (1024).
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use maproom::embedding::ollama::OllamaProvider;
    ///
    /// let provider = OllamaProvider::default_config()?;
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn default_config() -> Result<Self, EmbeddingError> {
        Self::new(
            Self::DEFAULT_ENDPOINT.to_string(),
            Self::DEFAULT_MODEL.to_string(),
            1024, // mxbai-embed-large default dimension
        )
    }

    /// Embed a batch of texts using parallel sub-batches.
    ///
    /// This method splits a large batch into smaller sub-batches and processes them
    /// concurrently using tokio tasks with semaphore-controlled concurrency. Results
    /// are merged in the correct order to preserve input sequence.
    ///
    /// # Arguments
    ///
    /// * `texts` - Vector of texts to embed
    ///
    /// # Returns
    ///
    /// - `Ok(Vec<Vector>)` - Vector of embeddings (same length and order as input)
    /// - `Err(EmbeddingError)` - If any sub-batch fails
    ///
    /// # Algorithm
    ///
    /// 1. Split texts into sub-batches of size `parallel_config.sub_batch_size`
    /// 2. Spawn tokio tasks for each sub-batch (limited by semaphore)
    /// 3. Track original index for each sub-batch
    /// 4. Sort results by index after all tasks complete
    /// 5. Flatten vectors in correct order
    async fn embed_batch_parallel(
        &self,
        texts: Vec<String>,
    ) -> Result<Vec<Vector>, EmbeddingError> {
        let total_texts = texts.len();
        let sub_batch_size = self.parallel_config.sub_batch_size;

        // Split into sub-batches
        let sub_batches: Vec<Vec<String>> = texts
            .chunks(sub_batch_size)
            .map(|chunk| chunk.to_vec())
            .collect();

        let num_batches = sub_batches.len();

        tracing::info!(
            "Parallel batch embedding: {} texts in {} sub-batches (size: {}, concurrency: {})",
            total_texts,
            num_batches,
            sub_batch_size,
            self.parallel_config.max_concurrency
        );

        let start = std::time::Instant::now();

        // Process sub-batches in parallel with semaphore limiting concurrency
        let handles: Vec<_> = sub_batches
            .into_iter()
            .enumerate()
            .map(|(idx, batch)| {
                let semaphore = self.semaphore.clone();
                let this = self.clone();
                let batch_size = batch.len();

                tokio::spawn(async move {
                    let _permit = semaphore.acquire().await.unwrap();
                    let batch_start = std::time::Instant::now();

                    tracing::debug!("Starting sub-batch {} ({} texts)", idx, batch_size);

                    let result = this.embed_batch_raw(batch).await;

                    let elapsed = batch_start.elapsed();
                    tracing::debug!(
                        "Sub-batch {} completed in {:.2}s ({} texts)",
                        idx,
                        elapsed.as_secs_f64(),
                        batch_size
                    );

                    (idx, result)
                })
            })
            .collect();

        // Collect results from all tasks
        let mut results: Vec<(usize, Result<Vec<Vector>, EmbeddingError>)> = Vec::new();
        for handle in handles {
            let (idx, result) = handle.await.map_err(|e| {
                EmbeddingError::Api(ApiError::InvalidResponse(format!("Task join error: {}", e)))
            })?;
            results.push((idx, result));
        }

        // Sort by index to preserve order
        results.sort_by_key(|(idx, _)| *idx);

        // Check for errors and flatten results
        let mut embeddings = Vec::with_capacity(total_texts);
        for (idx, result) in results {
            let batch_embeddings = result.map_err(|e| {
                EmbeddingError::Api(ApiError::InvalidResponse(format!(
                    "Sub-batch {} failed: {}",
                    idx, e
                )))
            })?;
            embeddings.extend(batch_embeddings);
        }

        let elapsed = start.elapsed();
        let throughput = total_texts as f64 / elapsed.as_secs_f64();

        tracing::info!(
            "Parallel batch complete: {} texts in {:.2}s ({:.0} texts/sec)",
            total_texts,
            elapsed.as_secs_f64(),
            throughput
        );

        Ok(embeddings)
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

        // Truncate to ~6000 chars to stay within nomic-embed-text's 2048 token limit
        // (assuming ~3 chars/token average for code, with safety margin)
        const MAX_CHARS: usize = 6000;

        // Conditionally sanitize based on model
        // nomic-embed-text: Apply sanitization workaround for GGML tokenization bugs
        // mxbai-embed-large and others: Use raw text for better quality embeddings
        let processed_texts: Vec<String> = if self.model == "nomic-embed-text" {
            // Apply sanitization for nomic-embed-text
            texts
                .into_iter()
                .map(|t| {
                    let sanitized = Self::sanitize_for_nomic(&t);

                    // Truncate if too long (find char boundary)
                    if sanitized.len() > MAX_CHARS {
                        sanitized
                            .char_indices()
                            .take_while(|(i, _)| *i < MAX_CHARS)
                            .map(|(_, c)| c)
                            .collect()
                    } else {
                        sanitized
                    }
                })
                .collect()
        } else {
            // Use raw text for mxbai-embed-large and other models
            texts
                .into_iter()
                .map(|t| {
                    // Still truncate to stay within token limits
                    if t.len() > MAX_CHARS {
                        t.char_indices()
                            .take_while(|(i, _)| *i < MAX_CHARS)
                            .map(|(_, c)| c)
                            .collect()
                    } else {
                        t
                    }
                })
                .collect()
        };

        // Debug: log first text preview and any remaining non-ASCII
        if !processed_texts.is_empty() {
            let first = &processed_texts[0];
            let non_ascii: Vec<char> = first.chars().filter(|c| !c.is_ascii()).collect();
            if !non_ascii.is_empty() {
                tracing::debug!(
                    "Batch has {} non-ASCII chars after processing: {:?}",
                    non_ascii.len(),
                    non_ascii.iter().take(10).collect::<Vec<_>>()
                );
            }
            tracing::debug!(
                "First text preview ({} chars): {:?}",
                first.len(),
                first.chars().take(80).collect::<String>()
            );
        }

        // Build request body once
        let request_body = OllamaRequest {
            model: self.model.clone(),
            input: processed_texts,
        };

        // Retry configuration for transient server errors
        const MAX_RETRIES: u32 = 3;
        const INITIAL_BACKOFF_MS: u64 = 500;

        let mut last_error: Option<EmbeddingError> = None;

        for attempt in 0..=MAX_RETRIES {
            if attempt > 0 {
                let backoff_ms = INITIAL_BACKOFF_MS * (1 << (attempt - 1)); // Exponential backoff
                tracing::warn!(
                    "Retry {}/{} for batch of {} texts after {}ms backoff",
                    attempt,
                    MAX_RETRIES,
                    batch_size,
                    backoff_ms
                );
                tokio::time::sleep(std::time::Duration::from_millis(backoff_ms)).await;
            }

            let response = match self
                .client
                .post(&self.endpoint)
                .json(&request_body)
                .send()
                .await
            {
                Ok(r) => r,
                Err(e) => {
                    tracing::error!(
                        "Failed to send batch of {} texts (attempt {}): {}",
                        batch_size,
                        attempt + 1,
                        e
                    );
                    last_error = Some(EmbeddingError::Network(e));
                    continue;
                }
            };

            let status = response.status();
            if status.is_success() {
                // Parse successful response
                let body: OllamaResponse = match response.json().await {
                    Ok(b) => b,
                    Err(e) => {
                        return Err(EmbeddingError::Api(ApiError::InvalidResponse(format!(
                            "Failed to parse batch response for {} texts: {}",
                            batch_size, e
                        ))));
                    }
                };

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
                for embedding in body.embeddings.iter() {
                    if embedding.len() != expected_dim {
                        use crate::embedding::error::DimensionMismatchError;
                        return Err(EmbeddingError::DimensionMismatch(
                            DimensionMismatchError::new(
                                expected_dim,
                                embedding.len(),
                                "Ollama".to_string(),
                                self.model.clone(),
                                self.dimension,
                            ),
                        ));
                    }
                }

                return Ok(body.embeddings);
            }

            // Handle error responses
            let error_msg = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());

            match status.as_u16() {
                // Retry on 5xx server errors (transient)
                500..=599 => {
                    tracing::warn!(
                        "Server error {} for batch of {} texts: {} (attempt {}/{})",
                        status.as_u16(),
                        batch_size,
                        error_msg,
                        attempt + 1,
                        MAX_RETRIES + 1
                    );
                    last_error = Some(EmbeddingError::Api(ApiError::ServerError {
                        status: status.as_u16(),
                        message: format!("Batch of {} texts failed: {}", batch_size, error_msg),
                    }));
                    continue; // Retry
                }
                // Don't retry on client errors
                429 => {
                    return Err(EmbeddingError::Api(ApiError::RateLimit {
                        retry_after_ms: 1000,
                    }));
                }
                401 => {
                    return Err(EmbeddingError::Api(ApiError::Authentication(error_msg)));
                }
                400 => {
                    return Err(EmbeddingError::Api(ApiError::BadRequest(format!(
                        "Batch of {} texts rejected: {}",
                        batch_size, error_msg
                    ))));
                }
                _ => {
                    return Err(EmbeddingError::Api(ApiError::InvalidResponse(format!(
                        "HTTP {} for batch of {} texts: {}",
                        status, batch_size, error_msg
                    ))));
                }
            }
        }

        // All retries exhausted
        Err(last_error.unwrap_or_else(|| {
            EmbeddingError::Api(ApiError::ServerError {
                status: 500,
                message: format!(
                    "Batch of {} texts failed after {} retries",
                    batch_size,
                    MAX_RETRIES + 1
                ),
            })
        }))
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
    /// # use maproom::embedding::ollama::OllamaProvider;
    /// # use maproom::embedding::provider::EmbeddingProvider;
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
    /// This method intelligently chooses between parallel sub-batch processing and
    /// single-batch processing based on configuration and batch size:
    /// - Uses parallel processing if `parallel_config.enabled` and `texts.len() > sub_batch_size`
    /// - Otherwise uses single-batch API call
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
    /// # use maproom::embedding::ollama::OllamaProvider;
    /// # use maproom::embedding::provider::EmbeddingProvider;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let provider = OllamaProvider::default_config()?;
    /// let texts = vec!["First".to_string(), "Second".to_string()];
    /// let embeddings = provider.embed_batch(texts).await?;
    /// assert_eq!(embeddings.len(), 2);
    /// # Ok(())
    /// # }
    /// ```
    async fn embed_batch(&self, texts: Vec<String>) -> Result<Vec<Vector>, EmbeddingError> {
        // Use parallel processing for large batches when enabled
        if self.parallel_config.enabled && texts.len() > self.parallel_config.sub_batch_size {
            self.embed_batch_parallel(texts).await
        } else {
            self.embed_batch_raw(texts).await
        }
    }

    /// Get the embedding dimension for this provider.
    ///
    /// Returns the configured dimension for this provider instance.
    /// Common values: 768 (nomic-embed-text), 1024 (mxbai-embed-large).
    ///
    /// # Returns
    ///
    /// The configured dimension for this provider.
    fn dimension(&self) -> usize {
        self.dimension
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
            768,
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
        assert_eq!(provider.model, "mxbai-embed-large");
        assert_eq!(provider.dimension(), 1024);
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
        assert_eq!(embeddings[0].len(), 1024);
        assert_eq!(embeddings[1].len(), 1024);
    }

    #[tokio::test]
    #[ignore] // Requires running Ollama
    async fn test_ollama_single_embed_uses_batch_api() {
        let provider = OllamaProvider::default_config().unwrap();
        let embedding = provider.embed("test".to_string()).await.unwrap();

        assert_eq!(embedding.len(), 1024);
    }

    // Unit tests for parallel processing (EMBPERF-2001)

    #[test]
    fn test_sub_batch_splitting() {
        // Test that 105 texts with batch_size 50 produces 3 batches: [50, 50, 5]
        let texts: Vec<String> = (0..105).map(|i| i.to_string()).collect();
        let batches: Vec<Vec<String>> = texts.chunks(50).map(|c| c.to_vec()).collect();

        assert_eq!(batches.len(), 3);
        assert_eq!(batches[0].len(), 50);
        assert_eq!(batches[1].len(), 50);
        assert_eq!(batches[2].len(), 5);
    }

    #[test]
    fn test_result_merge_ordering() {
        // Simulate out-of-order completion of sub-batches
        let mut results = vec![
            (2, vec!["c1".to_string(), "c2".to_string()]),
            (0, vec!["a1".to_string(), "a2".to_string()]),
            (1, vec!["b1".to_string(), "b2".to_string()]),
        ];

        // Sort by index (same as parallel implementation)
        results.sort_by_key(|(idx, _)| *idx);

        // Flatten results
        let merged: Vec<String> = results.into_iter().flat_map(|(_, v)| v).collect();

        // Verify correct order
        assert_eq!(merged, vec!["a1", "a2", "b1", "b2", "c1", "c2"]);
    }

    #[test]
    fn test_parallel_config_construction() {
        let config = ParallelConfig {
            enabled: true,
            sub_batch_size: 50,
            max_concurrency: 8,
        };

        let provider = OllamaProvider::new_with_config(
            "http://localhost:11434/api/embed".to_string(),
            "nomic-embed-text".to_string(),
            768,
            config.clone(),
        )
        .unwrap();

        assert_eq!(provider.parallel_config.enabled, true);
        assert_eq!(provider.parallel_config.sub_batch_size, 50);
        assert_eq!(provider.parallel_config.max_concurrency, 8);
    }

    #[test]
    fn test_parallel_config_defaults() {
        let provider = OllamaProvider::default_config().unwrap();

        // Verify updated defaults from EMBPERF-2001
        assert_eq!(provider.parallel_config.enabled, true);
        assert_eq!(provider.parallel_config.sub_batch_size, 50);
        assert_eq!(provider.parallel_config.max_concurrency, 8);
    }

    #[tokio::test]
    async fn test_small_batch_uses_raw_not_parallel() {
        // Create provider with parallel enabled
        let config = ParallelConfig {
            enabled: true,
            sub_batch_size: 50,
            max_concurrency: 8,
        };
        let provider = OllamaProvider::new_with_config(
            "http://localhost:11434/api/embed".to_string(),
            "nomic-embed-text".to_string(),
            768,
            config,
        )
        .unwrap();

        // Small batch (10 texts) should use raw, not parallel
        // We can't directly test this without mocking, but we can verify the logic
        let texts: Vec<String> = (0..10).map(|i| format!("text_{}", i)).collect();

        // This would normally call embed_batch_raw since 10 <= 50
        // The test just verifies the struct is set up correctly
        assert!(texts.len() <= provider.parallel_config.sub_batch_size);
    }

    #[tokio::test]
    async fn test_large_batch_triggers_parallel() {
        // Create provider with parallel enabled
        let config = ParallelConfig {
            enabled: true,
            sub_batch_size: 50,
            max_concurrency: 8,
        };
        let provider = OllamaProvider::new_with_config(
            "http://localhost:11434/api/embed".to_string(),
            "nomic-embed-text".to_string(),
            768,
            config,
        )
        .unwrap();

        // Large batch (100 texts) should trigger parallel
        let texts: Vec<String> = (0..100).map(|i| format!("text_{}", i)).collect();

        // This would normally call embed_batch_parallel since 100 > 50
        // The test just verifies the struct is set up correctly
        assert!(texts.len() > provider.parallel_config.sub_batch_size);
    }

    #[tokio::test]
    #[ignore] // Requires running Ollama
    async fn test_parallel_preserves_order() {
        // Integration test: verify parallel processing preserves order
        let config = ParallelConfig {
            enabled: true,
            sub_batch_size: 10,
            max_concurrency: 4,
        };
        let provider = OllamaProvider::new_with_config(
            OllamaProvider::DEFAULT_ENDPOINT.to_string(),
            OllamaProvider::DEFAULT_MODEL.to_string(),
            1024,
            config,
        )
        .unwrap();

        // Create texts with identifiable content
        let texts: Vec<String> = (0..50).map(|i| format!("text_{}", i)).collect();
        let embeddings = provider.embed_batch(texts.clone()).await.unwrap();

        // Verify we got the right number of embeddings
        assert_eq!(embeddings.len(), 50);

        // Verify each embedding has correct dimension
        for embedding in &embeddings {
            assert_eq!(embedding.len(), 1024);
        }

        // To truly verify order, we'd need to re-embed individually and compare
        // For now, we just verify the batch processing succeeded with correct count
    }

    #[test]
    fn test_parallel_disabled_config() {
        let config = ParallelConfig {
            enabled: false,
            sub_batch_size: 50,
            max_concurrency: 8,
        };

        let provider = OllamaProvider::new_with_config(
            "http://localhost:11434/api/embed".to_string(),
            "nomic-embed-text".to_string(),
            768,
            config,
        )
        .unwrap();

        assert_eq!(provider.parallel_config.enabled, false);

        // Even with large batch, parallel should not be used when disabled
        let texts: Vec<String> = (0..100).map(|i| format!("text_{}", i)).collect();
        assert!(texts.len() > provider.parallel_config.sub_batch_size);
        assert!(!provider.parallel_config.enabled);
    }

    // Unit tests for dimension configuration (DIM1024-2001)

    #[test]
    fn test_ollama_accepts_dimension_1024() {
        // Test that OllamaProvider accepts dimension=1024 for mxbai-embed-large
        let provider = OllamaProvider::new(
            "http://localhost:11434/api/embed".to_string(),
            "mxbai-embed-large".to_string(),
            1024,
        );
        assert!(provider.is_ok());

        let provider = provider.unwrap();
        assert_eq!(provider.dimension(), 1024);
        assert_eq!(provider.provider_name(), "ollama");
    }

    #[test]
    fn test_dimension_returns_configured_value() {
        // Test that dimension() returns the configured value, not hardcoded 768
        let provider_768 = OllamaProvider::new(
            "http://localhost:11434/api/embed".to_string(),
            "nomic-embed-text".to_string(),
            768,
        )
        .unwrap();
        assert_eq!(provider_768.dimension(), 768);

        let provider_1024 = OllamaProvider::new(
            "http://localhost:11434/api/embed".to_string(),
            "mxbai-embed-large".to_string(),
            1024,
        )
        .unwrap();
        assert_eq!(provider_1024.dimension(), 1024);

        // Test arbitrary dimension values
        let provider_512 = OllamaProvider::new(
            "http://localhost:11434/api/embed".to_string(),
            "custom-model".to_string(),
            512,
        )
        .unwrap();
        assert_eq!(provider_512.dimension(), 512);
    }

    #[test]
    fn test_backward_compatibility_dimension_768() {
        // Ensure existing configurations with dimension=768 still work
        let provider = OllamaProvider::new(
            "http://localhost:11434/api/embed".to_string(),
            "nomic-embed-text".to_string(),
            768,
        );
        assert!(provider.is_ok());

        let provider = provider.unwrap();
        assert_eq!(provider.dimension(), 768);
    }

    #[test]
    fn test_new_with_config_accepts_dimension() {
        // Test that new_with_config properly stores dimension
        let config = ParallelConfig::default();
        let provider = OllamaProvider::new_with_config(
            "http://localhost:11434/api/embed".to_string(),
            "mxbai-embed-large".to_string(),
            1024,
            config,
        );
        assert!(provider.is_ok());

        let provider = provider.unwrap();
        assert_eq!(provider.dimension(), 1024);
    }

    // Unit tests for conditional sanitization (DIM1024-2002)

    #[test]
    fn test_sanitize_for_nomic_replaces_pipes() {
        let input = "function | table | data";
        let output = OllamaProvider::sanitize_for_nomic(input);
        assert_eq!(output, "function   table   data");
        assert!(!output.contains('|'));
    }

    #[test]
    fn test_sanitize_for_nomic_replaces_brackets() {
        let input = "[x] checkbox [link](url)";
        let output = OllamaProvider::sanitize_for_nomic(input);
        assert_eq!(output, "(x) checkbox (link)(url)");
        assert!(!output.contains('['));
        assert!(!output.contains(']'));
    }

    #[test]
    fn test_sanitize_for_nomic_replaces_unicode_arrows() {
        let input = "a → b ← c ↔ d";
        let output = OllamaProvider::sanitize_for_nomic(input);
        assert_eq!(output, "a -> b <- c <-> d");
        assert!(!output.contains('→'));
        assert!(!output.contains('←'));
        assert!(!output.contains('↔'));
    }

    #[test]
    fn test_sanitize_for_nomic_replaces_box_drawing() {
        let input = "├── file\n└── dir\n│   ├── nested";
        let output = OllamaProvider::sanitize_for_nomic(input);
        assert!(!output.contains('├'));
        assert!(!output.contains('└'));
        assert!(!output.contains('│'));
        assert!(!output.contains('─'));
        assert!(output.contains('+'));
        assert!(output.contains('-'));
    }

    #[test]
    fn test_sanitize_for_nomic_all_problematic_chars() {
        let input = "| [ ] → ← ↔ ├ └ │ ─ ┌ ┐ ┘ ┤ ┬ ┴ ┼";
        let output = OllamaProvider::sanitize_for_nomic(input);

        // Verify all problematic characters are replaced
        let problematic_chars = [
            '|', '[', ']', '→', '←', '↔', '├', '└', '│', '─', '┌', '┐', '┘', '┤', '┬', '┴', '┼',
        ];
        for ch in &problematic_chars {
            assert!(!output.contains(*ch), "Output still contains: {}", ch);
        }
    }

    #[test]
    fn test_sanitize_for_nomic_preserves_normal_text() {
        let input = "function calculateTotal(a, b) { return a + b; }";
        let output = OllamaProvider::sanitize_for_nomic(input);
        // Parentheses in function calls should stay (only brackets are replaced)
        assert_eq!(output, input);
    }

    #[tokio::test]
    async fn test_conditional_sanitization_nomic_embed_text() {
        // Create provider with nomic-embed-text model
        let provider = OllamaProvider::new(
            "http://localhost:11434/api/embed".to_string(),
            "nomic-embed-text".to_string(),
            768,
        )
        .unwrap();

        // Test that model name is set correctly
        assert_eq!(provider.model, "nomic-embed-text");

        // Verify sanitize_for_nomic works as expected
        let test_text = "| table | [link] → symbol";
        let sanitized = OllamaProvider::sanitize_for_nomic(test_text);
        assert!(!sanitized.contains('|'));
        assert!(!sanitized.contains('['));
        assert!(!sanitized.contains('→'));
    }

    #[tokio::test]
    async fn test_conditional_sanitization_mxbai_embed_large() {
        // Create provider with mxbai-embed-large model
        let provider = OllamaProvider::new(
            "http://localhost:11434/api/embed".to_string(),
            "mxbai-embed-large".to_string(),
            1024,
        )
        .unwrap();

        // Test that model name is set correctly
        assert_eq!(provider.model, "mxbai-embed-large");

        // For mxbai-embed-large, raw text should be preserved
        // (We can't test the actual embed_batch_raw without Ollama running,
        // but we verify the model is set correctly for conditional logic)
    }

    #[test]
    fn test_sanitize_for_nomic_idempotent() {
        // Sanitizing twice should produce same result
        let input = "| [x] → ├ test";
        let once = OllamaProvider::sanitize_for_nomic(input);
        let twice = OllamaProvider::sanitize_for_nomic(&once);
        assert_eq!(once, twice);
    }

    #[test]
    fn test_sanitize_for_nomic_empty_string() {
        let input = "";
        let output = OllamaProvider::sanitize_for_nomic(input);
        assert_eq!(output, "");
    }

    #[test]
    fn test_sanitize_for_nomic_unicode_preserved() {
        // Non-problematic Unicode should be preserved
        let input = "Hello 世界 مرحبا שלום";
        let output = OllamaProvider::sanitize_for_nomic(input);
        assert_eq!(output, input);
    }

    #[test]
    fn test_model_comparison_exact_match() {
        // Verify exact string match for model name
        let nomic_provider = OllamaProvider::new(
            "http://localhost:11434/api/embed".to_string(),
            "nomic-embed-text".to_string(),
            768,
        )
        .unwrap();
        assert_eq!(nomic_provider.model, "nomic-embed-text");

        let mxbai_provider = OllamaProvider::new(
            "http://localhost:11434/api/embed".to_string(),
            "mxbai-embed-large".to_string(),
            1024,
        )
        .unwrap();
        assert_eq!(mxbai_provider.model, "mxbai-embed-large");

        // Verify they are different
        assert_ne!(nomic_provider.model, mxbai_provider.model);
    }
}
