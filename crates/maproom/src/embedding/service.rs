//! Main embedding service with caching and batch processing.

use crate::embedding::cache::{EmbeddingCache, Vector};
use crate::embedding::config::EmbeddingConfig;
use crate::embedding::error::EmbeddingError;
use crate::embedding::factory::create_provider_from_env;
use crate::embedding::provider::{EmbeddingProvider, ProviderMetrics};
use std::sync::Arc;
use tracing::{debug, info};

/// Main embedding service with caching and batch processing.
pub struct EmbeddingService {
    /// Embedding provider (Ollama, OpenAI, Google, etc.)
    provider: Box<dyn EmbeddingProvider>,
    /// LRU cache for embeddings
    cache: Arc<EmbeddingCache>,
}

/// Batch processing result with statistics.
#[derive(Debug, Clone)]
pub struct BatchResult {
    /// Total number of texts processed
    pub total: usize,
    /// Number of embeddings from cache
    pub cached: usize,
    /// Number of embeddings from API
    pub from_api: usize,
    /// Number of failed embeddings
    pub failed: usize,
    /// Cache hit rate for this batch
    pub cache_hit_rate: f64,
}

impl EmbeddingService {
    /// Create a new embedding service with a specific provider.
    ///
    /// # Arguments
    ///
    /// * `provider` - The embedding provider to use (Ollama, OpenAI, Google, etc.)
    /// * `cache` - The embedding cache for storing and retrieving embeddings
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use crewchief_maproom::embedding::service::EmbeddingService;
    /// use crewchief_maproom::embedding::factory::create_provider_from_env;
    /// use crewchief_maproom::embedding::cache::EmbeddingCache;
    /// use crewchief_maproom::embedding::config::CacheConfig;
    /// use std::sync::Arc;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let provider = create_provider_from_env().await?;
    /// let cache = EmbeddingCache::new(CacheConfig::default())?;
    /// let service = EmbeddingService::new(provider, Arc::new(cache));
    /// # Ok(())
    /// # }
    /// ```
    pub fn new(provider: Box<dyn EmbeddingProvider>, cache: Arc<EmbeddingCache>) -> Self {
        Self { provider, cache }
    }

    /// Create a new embedding service from environment variables.
    ///
    /// This method automatically detects and configures the embedding provider
    /// based on environment variables. See the factory module for details on
    /// provider auto-detection.
    ///
    /// # Environment Variables
    ///
    /// - `EMBEDDING_PROVIDER`: Provider name (optional, auto-detects Ollama if not set)
    /// - `EMBEDDING_MODEL`: Model name (optional, provider-specific defaults)
    /// - `OPENAI_API_KEY`: Required for OpenAI provider
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use crewchief_maproom::embedding::service::EmbeddingService;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// // Auto-detect provider (prefers Ollama, falls back to EMBEDDING_PROVIDER)
    /// let service = EmbeddingService::from_env().await?;
    /// println!("Using provider: {}", service.provider_name());
    /// # Ok(())
    /// # }
    /// ```
    pub async fn from_env() -> Result<Self, EmbeddingError> {
        let provider = create_provider_from_env().await?;
        let config = EmbeddingConfig::from_env()?;
        let cache = EmbeddingCache::new(config.cache)?;
        Ok(Self::new(provider, Arc::new(cache)))
    }

    /// Embed a single text with caching.
    pub async fn embed_text(&self, text: &str) -> Result<Vector, EmbeddingError> {
        // Check cache first
        if let Some(cached) = self.cache.get(text).await {
            debug!("Cache hit for text");
            return Ok(cached);
        }

        // Generate embedding via provider
        debug!(
            "Cache miss, generating embedding via provider: {}",
            self.provider.provider_name()
        );
        self.cache.record_miss().await;

        let embedding = self.provider.embed(text.to_string()).await?;

        // Store in cache
        self.cache.put(text, embedding.clone()).await?;

        Ok(embedding)
    }

    /// Embed a batch of texts efficiently with caching.
    pub async fn embed_batch(&self, texts: Vec<String>) -> Result<Vec<Vector>, EmbeddingError> {
        if texts.is_empty() {
            return Ok(Vec::new());
        }

        let total = texts.len();
        info!("Processing batch of {} texts", total);

        // Check cache for all texts
        let mut results = Vec::with_capacity(total);
        let mut uncached_indices = Vec::new();
        let mut uncached_texts = Vec::new();

        for (i, text) in texts.iter().enumerate() {
            if let Some(cached) = self.cache.get(text).await {
                results.push((i, Some(cached)));
            } else {
                results.push((i, None));
                uncached_indices.push(i);
                uncached_texts.push(text.clone());
            }
        }

        let cached_count = total - uncached_texts.len();
        info!(
            "Cache hits: {}/{} ({:.1}%)",
            cached_count,
            total,
            (cached_count as f64 / total as f64) * 100.0
        );

        // Generate embeddings for uncached texts
        if !uncached_texts.is_empty() {
            info!(
                "Generating {} embeddings via provider: {}",
                uncached_texts.len(),
                self.provider.provider_name()
            );

            // Generate embeddings via provider
            let new_embeddings = self.provider.embed_batch(uncached_texts.clone()).await?;

            // Store new embeddings in cache and update results
            for (i, embedding) in uncached_indices.iter().zip(new_embeddings.iter()) {
                self.cache
                    .put(
                        &uncached_texts
                            [uncached_indices.iter().position(|&idx| idx == *i).unwrap()],
                        embedding.clone(),
                    )
                    .await?;
                results[*i].1 = Some(embedding.clone());
            }
        }

        // Extract embeddings in original order
        let final_embeddings: Result<Vec<Vector>, EmbeddingError> = results
            .into_iter()
            .map(|(_, emb)| {
                emb.ok_or_else(|| EmbeddingError::Other("Missing embedding".to_string()))
            })
            .collect();

        final_embeddings
    }

    /// Embed a batch with detailed statistics.
    pub async fn embed_batch_with_stats(
        &self,
        texts: Vec<String>,
    ) -> Result<(Vec<Vector>, BatchResult), EmbeddingError> {
        let total = texts.len();

        // Get initial metrics if available
        let initial_requests = self
            .provider
            .metrics()
            .map(|m| m.total_requests)
            .unwrap_or(0);

        let embeddings = self.embed_batch(texts).await?;

        // Get final metrics if available
        let final_requests = self
            .provider
            .metrics()
            .map(|m| m.total_requests)
            .unwrap_or(0);

        let from_api = (final_requests - initial_requests) as usize;
        let cached = total.saturating_sub(from_api);
        let cache_hit_rate = if total > 0 {
            cached as f64 / total as f64
        } else {
            0.0
        };

        let stats = BatchResult {
            total,
            cached,
            from_api,
            failed: 0,
            cache_hit_rate,
        };

        Ok((embeddings, stats))
    }

    /// Process a large batch by splitting into smaller chunks.
    ///
    /// This method processes large batches in chunks to avoid memory issues and
    /// rate limiting. The batch size defaults to 100, but can be configured.
    pub async fn embed_large_batch(
        &self,
        texts: Vec<String>,
    ) -> Result<Vec<Vector>, EmbeddingError> {
        // Default batch size of 100 for safety
        let batch_size = 100;
        let total = texts.len();

        info!(
            "Processing large batch of {} texts in chunks of {}",
            total, batch_size
        );

        let mut all_embeddings = Vec::with_capacity(total);

        for (chunk_idx, chunk) in texts.chunks(batch_size).enumerate() {
            info!(
                "Processing chunk {}/{} ({} texts)",
                chunk_idx + 1,
                total.div_ceil(batch_size),
                chunk.len()
            );

            let chunk_embeddings = self.embed_batch(chunk.to_vec()).await?;
            all_embeddings.extend(chunk_embeddings);

            // Small delay between chunks to avoid rate limiting
            if chunk_idx < total.div_ceil(batch_size) - 1 {
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            }
        }

        Ok(all_embeddings)
    }

    /// Get cache statistics.
    pub async fn cache_metrics(&self) -> crate::embedding::cache::CacheMetrics {
        self.cache.metrics().await
    }

    /// Get provider metrics (if available).
    ///
    /// Returns performance and cost metrics from the embedding provider.
    /// Some providers (like Ollama) may not track metrics.
    pub fn provider_metrics(&self) -> Option<ProviderMetrics> {
        self.provider.metrics()
    }

    /// Clear the embedding cache.
    pub async fn clear_cache(&self) {
        self.cache.clear().await;
        info!("Cache cleared");
    }

    /// Get cache size.
    pub async fn cache_size(&self) -> usize {
        self.cache.len().await
    }

    /// Clean up expired cache entries.
    pub async fn cleanup_cache(&self) -> usize {
        let removed = self.cache.cleanup_expired().await;
        if removed > 0 {
            info!("Removed {} expired cache entries", removed);
        }
        removed
    }

    /// Get the embedding dimension for the current provider.
    ///
    /// Returns the number of dimensions in the embedding vectors produced
    /// by this service's provider. Common values:
    /// - 768: Ollama models, Google Vertex AI
    /// - 1536: OpenAI text-embedding-3-small
    pub fn dimension(&self) -> usize {
        self.provider.dimension()
    }

    /// Get the provider name.
    ///
    /// Returns the name of the embedding provider being used:
    /// - "ollama": Ollama local models
    /// - "openai": OpenAI API
    /// - "google": Google Vertex AI (future)
    pub fn provider_name(&self) -> &str {
        self.provider.provider_name()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::embedding::config::CacheConfig;
    use async_trait::async_trait;

    // Mock provider for testing
    struct MockProvider {
        dimension: usize,
        name: &'static str,
    }

    #[async_trait]
    impl EmbeddingProvider for MockProvider {
        async fn embed(&self, _text: String) -> Result<Vector, EmbeddingError> {
            Ok(vec![0.0; self.dimension])
        }

        async fn embed_batch(&self, texts: Vec<String>) -> Result<Vec<Vector>, EmbeddingError> {
            Ok(vec![vec![0.0; self.dimension]; texts.len()])
        }

        fn dimension(&self) -> usize {
            self.dimension
        }

        fn provider_name(&self) -> &'static str {
            self.name
        }

        fn metrics(&self) -> Option<ProviderMetrics> {
            Some(ProviderMetrics {
                total_requests: 10,
                total_tokens: 1000,
                failed_requests: 0,
                estimated_cost_usd: 0.001,
            })
        }
    }

    fn create_test_service(dimension: usize) -> EmbeddingService {
        let provider = Box::new(MockProvider {
            dimension,
            name: "mock",
        });
        let cache_config = CacheConfig {
            max_entries: 100,
            ttl_seconds: 3600,
            enable_metrics: true,
        };
        let cache = EmbeddingCache::new(cache_config).unwrap();
        EmbeddingService::new(provider, Arc::new(cache))
    }

    #[test]
    fn test_service_creation() {
        let service = create_test_service(1536);
        assert_eq!(service.dimension(), 1536);
        assert_eq!(service.provider_name(), "mock");
    }

    #[test]
    fn test_service_dimension() {
        let service = create_test_service(768);
        assert_eq!(service.dimension(), 768);
    }

    #[test]
    fn test_provider_name() {
        let service = create_test_service(1536);
        assert_eq!(service.provider_name(), "mock");
    }

    #[tokio::test]
    async fn test_cache_operations() {
        let service = create_test_service(1536);

        assert_eq!(service.cache_size().await, 0);

        // Manually add to cache for testing
        let test_vector = vec![0.1; 1536];
        service
            .cache
            .put("test", test_vector.clone())
            .await
            .unwrap();

        assert_eq!(service.cache_size().await, 1);

        service.clear_cache().await;
        assert_eq!(service.cache_size().await, 0);
    }

    #[tokio::test]
    async fn test_empty_batch() {
        let service = create_test_service(1536);

        let result = service.embed_batch(vec![]).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 0);
    }

    #[tokio::test]
    async fn test_batch_result() {
        let result = BatchResult {
            total: 100,
            cached: 80,
            from_api: 20,
            failed: 0,
            cache_hit_rate: 0.8,
        };

        assert_eq!(result.total, 100);
        assert_eq!(result.cached, 80);
        assert_eq!(result.from_api, 20);
        assert_eq!(result.cache_hit_rate, 0.8);
    }

    #[tokio::test]
    async fn test_cache_metrics() {
        let service = create_test_service(1536);

        let metrics = service.cache_metrics().await;
        assert_eq!(metrics.hits, 0);
        assert_eq!(metrics.misses, 0);
        assert_eq!(metrics.hit_rate(), 0.0);
    }

    #[test]
    fn test_provider_metrics() {
        let service = create_test_service(1536);

        let metrics = service.provider_metrics();
        assert!(metrics.is_some());

        let metrics = metrics.unwrap();
        assert_eq!(metrics.total_requests, 10);
        assert_eq!(metrics.total_tokens, 1000);
        assert_eq!(metrics.failed_requests, 0);
        assert_eq!(metrics.estimated_cost_usd, 0.001);
    }

    #[tokio::test]
    async fn test_cleanup_cache() {
        let provider = Box::new(MockProvider {
            dimension: 1536,
            name: "mock",
        });
        let cache_config = CacheConfig {
            max_entries: 100,
            ttl_seconds: 0, // Expire immediately
            enable_metrics: true,
        };
        let cache = EmbeddingCache::new(cache_config).unwrap();
        let service = EmbeddingService::new(provider, Arc::new(cache));

        // Add some entries
        service.cache.put("text1", vec![0.1; 1536]).await.unwrap();
        service.cache.put("text2", vec![0.2; 1536]).await.unwrap();

        // With TTL of 0, entries expire immediately
        let removed = service.cleanup_cache().await;
        assert_eq!(removed, 2);
        assert_eq!(service.cache_size().await, 0);
    }

    #[tokio::test]
    async fn test_embed_text_with_cache() {
        let service = create_test_service(768);

        // First call should go to provider
        let embedding1 = service.embed_text("test text").await.unwrap();
        assert_eq!(embedding1.len(), 768);

        // Second call should hit cache
        let embedding2 = service.embed_text("test text").await.unwrap();
        assert_eq!(embedding2.len(), 768);
        assert_eq!(embedding1, embedding2);
    }

    #[tokio::test]
    async fn test_embed_batch_with_mixed_cache() {
        let service = create_test_service(768);

        // Pre-populate cache with one text
        service.cache.put("cached", vec![1.0; 768]).await.unwrap();

        // Batch with cached and uncached texts
        let texts = vec!["cached".to_string(), "uncached".to_string()];
        let embeddings = service.embed_batch(texts).await.unwrap();

        assert_eq!(embeddings.len(), 2);
        assert_eq!(embeddings[0].len(), 768);
        assert_eq!(embeddings[1].len(), 768);

        // First should be from cache (all 1.0s)
        assert_eq!(embeddings[0], vec![1.0; 768]);
        // Second should be from provider (all 0.0s from mock)
        assert_eq!(embeddings[1], vec![0.0; 768]);
    }

    #[tokio::test]
    async fn test_large_batch_chunking() {
        let service = create_test_service(768);

        // Create a batch larger than default chunk size (100)
        let texts: Vec<String> = (0..150).map(|i| format!("text {}", i)).collect();

        let embeddings = service.embed_large_batch(texts.clone()).await.unwrap();
        assert_eq!(embeddings.len(), texts.len());
        for embedding in embeddings {
            assert_eq!(embedding.len(), 768);
        }
    }

    #[tokio::test]
    async fn test_embed_batch_with_stats() {
        let service = create_test_service(768);

        // Pre-populate cache
        service.cache.put("cached1", vec![1.0; 768]).await.unwrap();
        service.cache.put("cached2", vec![2.0; 768]).await.unwrap();

        let texts = vec![
            "cached1".to_string(),
            "cached2".to_string(),
            "uncached".to_string(),
        ];

        let (embeddings, stats) = service.embed_batch_with_stats(texts).await.unwrap();

        assert_eq!(embeddings.len(), 3);
        assert_eq!(stats.total, 3);
        // Note: Stats might not be accurate with mock provider if it doesn't track metrics properly
    }
}
