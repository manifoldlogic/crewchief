//! Abstract embedding provider interface.
//!
//! This module defines the `EmbeddingProvider` trait, which abstracts over different
//! embedding API providers (OpenAI, Ollama, Google, etc.). The trait is object-safe
//! and designed for use with dynamic dispatch (`Box<dyn EmbeddingProvider>`).
//!
//! # Design Goals
//!
//! - **Provider Flexibility**: Support multiple embedding providers with different models
//!   and dimensions (768-dim for Ollama/Google, 1536-dim for OpenAI)
//! - **Object Safety**: Enable runtime provider selection via trait objects
//! - **Async Support**: All embedding operations are async for non-blocking I/O
//! - **Thread Safety**: Providers must be Send + Sync for use in async contexts
//! - **Batch Optimization**: Providers can override default batching with native batch APIs
//!
//! # Examples
//!
//! ```no_run
//! use crewchief_maproom::embedding::provider::{EmbeddingProvider, Vector};
//! use crewchief_maproom::embedding::error::EmbeddingError;
//! use async_trait::async_trait;
//!
//! // Define a custom provider
//! struct MyProvider {
//!     dimension: usize,
//! }
//!
//! #[async_trait]
//! impl EmbeddingProvider for MyProvider {
//!     async fn embed(&self, text: String) -> Result<Vector, EmbeddingError> {
//!         // Implementation here
//!         Ok(vec![0.0; self.dimension])
//!     }
//!
//!     fn dimension(&self) -> usize {
//!         self.dimension
//!     }
//!
//!     fn provider_name(&self) -> &'static str {
//!         "my-provider"
//!     }
//! }
//!
//! // Use with dynamic dispatch
//! async fn process_with_provider(provider: Box<dyn EmbeddingProvider>) -> Result<(), EmbeddingError> {
//!     let embedding = provider.embed("Hello, world!".to_string()).await?;
//!     assert_eq!(embedding.len(), provider.dimension());
//!     Ok(())
//! }
//! ```

use async_trait::async_trait;

use crate::embedding::error::EmbeddingError;

/// Vector type representing an embedding.
///
/// Embeddings are represented as Vec<f32> with dimension determined by the provider.
/// - Ollama models: typically 768 dimensions
/// - Google models: typically 768 dimensions
/// - OpenAI models: 1536 dimensions (text-embedding-3-small)
pub type Vector = Vec<f32>;

/// Abstract embedding provider interface.
///
/// This trait defines the contract for embedding providers that can generate
/// vector embeddings from text. Implementations must be thread-safe and support
/// async operations.
///
/// # Object Safety
///
/// This trait is object-safe and can be used with `Box<dyn EmbeddingProvider>`
/// for runtime provider selection. All methods use `&self` (not `&mut self`)
/// and return concrete types (not associated types).
///
/// # Thread Safety
///
/// All implementations must be `Send + Sync` for use in async contexts.
/// This allows providers to be shared across async tasks and threads safely.
///
/// # Invariants
///
/// Implementations must uphold these invariants:
///
/// - **Consistent Dimension**: `dimension()` must return the same value for the
///   lifetime of the provider instance
/// - **Output Length Match**: `embed()` must return a vector with length exactly
///   equal to `dimension()`
/// - **Batch Length Match**: `embed_batch()` must return a Vec with length equal
///   to the input texts length
/// - **No Mutation**: All methods take `&self`, providers should use interior
///   mutability (Arc<Mutex<_>>, etc.) if state updates are needed
///
/// # Error Handling
///
/// Methods return `Result<_, EmbeddingError>` to handle:
/// - Network failures
/// - API authentication errors
/// - Rate limiting
/// - Invalid input (text too long, empty input, etc.)
/// - Model unavailability
///
/// See [`EmbeddingError`] for detailed error types.
#[async_trait]
pub trait EmbeddingProvider: Send + Sync {
    /// Generate embedding vector for a single text.
    ///
    /// This method sends the text to the embedding API and returns the resulting
    /// vector representation. The vector length will equal `dimension()`.
    ///
    /// # Arguments
    ///
    /// * `text` - The text to embed. Length limits depend on the provider's model
    ///
    /// # Returns
    ///
    /// - `Ok(Vector)` - The embedding vector with length = `dimension()`
    /// - `Err(EmbeddingError)` - If the API call fails or input is invalid
    ///
    /// # Errors
    ///
    /// This method returns an error if:
    /// - Network request fails
    /// - API authentication fails
    /// - Text exceeds model's context window
    /// - Rate limit is exceeded
    /// - Provider service is unavailable
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use crewchief_maproom::embedding::provider::EmbeddingProvider;
    /// # async fn example(provider: Box<dyn EmbeddingProvider>) -> Result<(), Box<dyn std::error::Error>> {
    /// let embedding = provider.embed("Hello, world!".to_string()).await?;
    /// assert_eq!(embedding.len(), provider.dimension());
    /// # Ok(())
    /// # }
    /// ```
    async fn embed(&self, text: String) -> Result<Vector, EmbeddingError>;

    /// Generate embeddings for a batch of texts.
    ///
    /// This method provides efficient batch processing for multiple texts.
    /// The default implementation calls `embed()` sequentially, but providers
    /// with native batch APIs should override this for better performance.
    ///
    /// # Arguments
    ///
    /// * `texts` - Vector of texts to embed
    ///
    /// # Returns
    ///
    /// - `Ok(Vec<Vector>)` - Vector of embeddings, same length as input
    /// - `Err(EmbeddingError)` - If any embedding fails
    ///
    /// # Errors
    ///
    /// This method returns an error if any single embedding fails. For partial
    /// failure handling, use `embed()` on individual texts.
    ///
    /// # Implementation Note
    ///
    /// The default implementation processes texts sequentially:
    ///
    /// ```rust,ignore
    /// async fn embed_batch(&self, texts: Vec<String>) -> Result<Vec<Vector>, EmbeddingError> {
    ///     let mut results = Vec::with_capacity(texts.len());
    ///     for text in texts {
    ///         results.push(self.embed(text).await?);
    ///     }
    ///     Ok(results)
    /// }
    /// ```
    ///
    /// Providers with native batching (e.g., OpenAI's batch endpoint) should override
    /// this to send all texts in a single API call for better performance and lower cost.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use crewchief_maproom::embedding::provider::EmbeddingProvider;
    /// # async fn example(provider: Box<dyn EmbeddingProvider>) -> Result<(), Box<dyn std::error::Error>> {
    /// let texts = vec!["First".to_string(), "Second".to_string()];
    /// let embeddings = provider.embed_batch(texts.clone()).await?;
    /// assert_eq!(embeddings.len(), texts.len());
    /// # Ok(())
    /// # }
    /// ```
    async fn embed_batch(&self, texts: Vec<String>) -> Result<Vec<Vector>, EmbeddingError> {
        let mut results = Vec::with_capacity(texts.len());
        for text in texts {
            results.push(self.embed(text).await?);
        }
        Ok(results)
    }

    /// Get the embedding dimension for this provider.
    ///
    /// This value is constant for the lifetime of the provider instance.
    /// Common values:
    /// - 768: Ollama models, Google Vertex AI text-embedding-gecko
    /// - 1536: OpenAI text-embedding-3-small
    ///
    /// # Returns
    ///
    /// The number of dimensions in embedding vectors produced by this provider.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use crewchief_maproom::embedding::provider::EmbeddingProvider;
    /// # fn example(provider: Box<dyn EmbeddingProvider>) {
    /// match provider.dimension() {
    ///     768 => println!("Using 768-dim model (Ollama/Google)"),
    ///     1536 => println!("Using 1536-dim model (OpenAI)"),
    ///     dim => println!("Using {}-dim model", dim),
    /// }
    /// # }
    /// ```
    fn dimension(&self) -> usize;

    /// Get the provider name identifier.
    ///
    /// This returns a static string identifying the provider type.
    /// Standard values:
    /// - "ollama": Ollama local models
    /// - "google": Google Vertex AI
    /// - "openai": OpenAI API
    ///
    /// # Returns
    ///
    /// A static string identifying the provider. This value is used for:
    /// - Logging and debugging
    /// - Metrics and monitoring
    /// - Configuration validation
    /// - Database column selection (in multi-provider setups)
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use crewchief_maproom::embedding::provider::EmbeddingProvider;
    /// # fn example(provider: Box<dyn EmbeddingProvider>) {
    /// println!("Using provider: {}", provider.provider_name());
    /// # }
    /// ```
    fn provider_name(&self) -> &'static str;

    /// Get provider-specific metrics (optional).
    ///
    /// This method returns operational metrics about the provider's performance,
    /// cost, and usage. The default implementation returns `None`.
    ///
    /// Providers that track metrics should override this to return their statistics.
    ///
    /// # Returns
    ///
    /// - `Some(ProviderMetrics)` - If the provider tracks metrics
    /// - `None` - If metrics are not available or not implemented
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use crewchief_maproom::embedding::provider::EmbeddingProvider;
    /// # fn example(provider: Box<dyn EmbeddingProvider>) {
    /// if let Some(metrics) = provider.metrics() {
    ///     println!("Total requests: {}", metrics.total_requests);
    ///     println!("Failed requests: {}", metrics.failed_requests);
    ///     println!("Estimated cost: ${:.4}", metrics.estimated_cost_usd);
    /// }
    /// # }
    /// ```
    fn metrics(&self) -> Option<ProviderMetrics> {
        None
    }
}

/// Metrics about provider performance and cost.
///
/// This struct tracks operational statistics for embedding providers, including
/// request counts, token usage, failure rates, and cost estimates.
///
/// # Examples
///
/// ```
/// use crewchief_maproom::embedding::provider::ProviderMetrics;
///
/// let metrics = ProviderMetrics {
///     total_requests: 1000,
///     total_tokens: 50000,
///     failed_requests: 5,
///     estimated_cost_usd: 0.025,
/// };
///
/// let failure_rate = metrics.failed_requests as f64 / metrics.total_requests as f64;
/// println!("Failure rate: {:.2}%", failure_rate * 100.0);
/// ```
#[derive(Debug, Clone, Default)]
pub struct ProviderMetrics {
    /// Total number of embedding requests made to the provider.
    ///
    /// Includes both successful and failed requests.
    pub total_requests: u64,

    /// Total number of tokens processed.
    ///
    /// For providers that charge per token, this is used for cost calculation.
    /// Ollama providers may not track this (free local models).
    pub total_tokens: u64,

    /// Number of requests that failed.
    ///
    /// Includes network errors, API errors, rate limits, etc.
    /// Does not include requests that succeeded after retries.
    pub failed_requests: u64,

    /// Estimated total cost in USD.
    ///
    /// Based on provider pricing and token usage. For providers with free tiers
    /// or local models (Ollama), this may be 0.0.
    ///
    /// OpenAI pricing (as of 2024):
    /// - text-embedding-3-small: $0.00002 per 1K tokens
    pub estimated_cost_usd: f64,
}

impl ProviderMetrics {
    /// Calculate the success rate as a percentage.
    ///
    /// # Returns
    ///
    /// Success rate from 0.0 to 1.0, or 1.0 if no requests have been made.
    ///
    /// # Examples
    ///
    /// ```
    /// use crewchief_maproom::embedding::provider::ProviderMetrics;
    ///
    /// let metrics = ProviderMetrics {
    ///     total_requests: 100,
    ///     failed_requests: 5,
    ///     ..Default::default()
    /// };
    ///
    /// assert_eq!(metrics.success_rate(), 0.95);
    /// ```
    pub fn success_rate(&self) -> f64 {
        if self.total_requests == 0 {
            return 1.0;
        }
        let successful = self.total_requests - self.failed_requests;
        successful as f64 / self.total_requests as f64
    }

    /// Calculate the failure rate as a percentage.
    ///
    /// # Returns
    ///
    /// Failure rate from 0.0 to 1.0, or 0.0 if no requests have been made.
    ///
    /// # Examples
    ///
    /// ```
    /// use crewchief_maproom::embedding::provider::ProviderMetrics;
    ///
    /// let metrics = ProviderMetrics {
    ///     total_requests: 100,
    ///     failed_requests: 5,
    ///     ..Default::default()
    /// };
    ///
    /// assert_eq!(metrics.failure_rate(), 0.05);
    /// ```
    pub fn failure_rate(&self) -> f64 {
        if self.total_requests == 0 {
            return 0.0;
        }
        self.failed_requests as f64 / self.total_requests as f64
    }

    /// Calculate the average cost per request in USD.
    ///
    /// # Returns
    ///
    /// Average cost per request, or 0.0 if no requests have been made.
    ///
    /// # Examples
    ///
    /// ```
    /// use crewchief_maproom::embedding::provider::ProviderMetrics;
    ///
    /// let metrics = ProviderMetrics {
    ///     total_requests: 1000,
    ///     estimated_cost_usd: 0.50,
    ///     ..Default::default()
    /// };
    ///
    /// assert_eq!(metrics.avg_cost_per_request(), 0.0005);
    /// ```
    pub fn avg_cost_per_request(&self) -> f64 {
        if self.total_requests == 0 {
            return 0.0;
        }
        self.estimated_cost_usd / self.total_requests as f64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provider_metrics_default() {
        let metrics = ProviderMetrics::default();
        assert_eq!(metrics.total_requests, 0);
        assert_eq!(metrics.total_tokens, 0);
        assert_eq!(metrics.failed_requests, 0);
        assert_eq!(metrics.estimated_cost_usd, 0.0);
    }

    #[test]
    fn test_provider_metrics_success_rate() {
        let metrics = ProviderMetrics {
            total_requests: 100,
            failed_requests: 5,
            ..Default::default()
        };
        assert_eq!(metrics.success_rate(), 0.95);
        assert_eq!(metrics.failure_rate(), 0.05);

        // Test zero requests
        let empty_metrics = ProviderMetrics::default();
        assert_eq!(empty_metrics.success_rate(), 1.0);
        assert_eq!(empty_metrics.failure_rate(), 0.0);
    }

    #[test]
    fn test_provider_metrics_avg_cost() {
        let metrics = ProviderMetrics {
            total_requests: 1000,
            estimated_cost_usd: 0.50,
            ..Default::default()
        };
        assert_eq!(metrics.avg_cost_per_request(), 0.0005);

        // Test zero requests
        let empty_metrics = ProviderMetrics::default();
        assert_eq!(empty_metrics.avg_cost_per_request(), 0.0);
    }

    // Mock provider for testing trait object usage
    struct MockProvider {
        dimension: usize,
        name: &'static str,
    }

    #[async_trait]
    impl EmbeddingProvider for MockProvider {
        async fn embed(&self, _text: String) -> Result<Vector, EmbeddingError> {
            Ok(vec![0.0; self.dimension])
        }

        fn dimension(&self) -> usize {
            self.dimension
        }

        fn provider_name(&self) -> &'static str {
            self.name
        }
    }

    #[tokio::test]
    async fn test_provider_trait_object() {
        let provider: Box<dyn EmbeddingProvider> = Box::new(MockProvider {
            dimension: 768,
            name: "mock",
        });

        assert_eq!(provider.dimension(), 768);
        assert_eq!(provider.provider_name(), "mock");

        let embedding = provider.embed("test".to_string()).await.unwrap();
        assert_eq!(embedding.len(), 768);
    }

    #[tokio::test]
    async fn test_default_batch_implementation() {
        let provider: Box<dyn EmbeddingProvider> = Box::new(MockProvider {
            dimension: 768,
            name: "mock",
        });

        let texts = vec!["first".to_string(), "second".to_string(), "third".to_string()];
        let embeddings = provider.embed_batch(texts.clone()).await.unwrap();

        assert_eq!(embeddings.len(), texts.len());
        for embedding in embeddings {
            assert_eq!(embedding.len(), 768);
        }
    }

    #[test]
    fn test_metrics_optional() {
        let provider = MockProvider {
            dimension: 768,
            name: "mock",
        };

        // Default implementation returns None
        assert!(provider.metrics().is_none());
    }
}
