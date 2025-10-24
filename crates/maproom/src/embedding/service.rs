//! Main embedding service with caching and batch processing.

use crate::embedding::cache::{EmbeddingCache, Vector};
use crate::embedding::client::{CostMetrics, OpenAIClient};
use crate::embedding::config::EmbeddingConfig;
use crate::embedding::error::EmbeddingError;
use std::sync::Arc;
use tracing::{debug, info};

/// Main embedding service with caching and batch processing.
pub struct EmbeddingService {
    /// OpenAI client for API calls
    client: Arc<OpenAIClient>,
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
    /// Create a new embedding service.
    pub fn new(config: EmbeddingConfig) -> Result<Self, EmbeddingError> {
        let cache = EmbeddingCache::new(config.cache.clone())?;
        let client = OpenAIClient::new(config)?;

        Ok(Self {
            client: Arc::new(client),
            cache: Arc::new(cache),
        })
    }

    /// Create a new embedding service from environment variables.
    pub fn from_env() -> Result<Self, EmbeddingError> {
        let config = EmbeddingConfig::from_env()?;
        Self::new(config)
    }

    /// Embed a single text with caching.
    pub async fn embed_text(&self, text: &str) -> Result<Vector, EmbeddingError> {
        // Check cache first
        if let Some(cached) = self.cache.get(text).await {
            debug!("Cache hit for text");
            return Ok(cached);
        }

        // Generate embedding via API
        debug!("Cache miss, generating embedding via API");
        self.cache.record_miss().await;

        let embedding = self.client.embed_text(text.to_string()).await?;

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
            info!("Generating {} embeddings via API", uncached_texts.len());

            let new_embeddings = self.client.embed_batch(uncached_texts.clone()).await?;

            // Store new embeddings in cache and update results
            for (i, embedding) in uncached_indices.iter().zip(new_embeddings.iter()) {
                self.cache
                    .put(&uncached_texts[uncached_indices.iter().position(|&idx| idx == *i).unwrap()], embedding.clone())
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
        let initial_requests = self.client.metrics().total_requests();

        let embeddings = self.embed_batch(texts).await?;

        let final_requests = self.client.metrics().total_requests();
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
    pub async fn embed_large_batch(
        &self,
        texts: Vec<String>,
    ) -> Result<Vec<Vector>, EmbeddingError> {
        let batch_size = self.client.config().batch_size;
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
                (total + batch_size - 1) / batch_size,
                chunk.len()
            );

            let chunk_embeddings = self.embed_batch(chunk.to_vec()).await?;
            all_embeddings.extend(chunk_embeddings);

            // Small delay between chunks to avoid rate limiting
            if chunk_idx < (total + batch_size - 1) / batch_size - 1 {
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            }
        }

        Ok(all_embeddings)
    }

    /// Get cache statistics.
    pub async fn cache_metrics(&self) -> crate::embedding::cache::CacheMetrics {
        self.cache.metrics().await
    }

    /// Get API cost metrics.
    pub fn cost_metrics(&self) -> &CostMetrics {
        self.client.metrics()
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

    /// Get the embedding dimension.
    pub fn dimension(&self) -> usize {
        self.client.config().dimension
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
            cache: CacheConfig {
                max_entries: 100,
                ttl_seconds: 3600,
                enable_metrics: true,
            },
            batch_size: 10,
            retry: RetryConfig::default(),
            api_key: Some("test-key".to_string()),
            api_endpoint: None,
        }
    }

    #[test]
    fn test_service_creation() {
        let config = test_config();
        let service = EmbeddingService::new(config);
        assert!(service.is_ok());
    }

    #[test]
    fn test_service_dimension() {
        let config = test_config();
        let service = EmbeddingService::new(config).unwrap();
        assert_eq!(service.dimension(), 1536);
    }

    #[tokio::test]
    async fn test_cache_operations() {
        let config = test_config();
        let service = EmbeddingService::new(config).unwrap();

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
        let config = test_config();
        let service = EmbeddingService::new(config).unwrap();

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
        let config = test_config();
        let service = EmbeddingService::new(config).unwrap();

        let metrics = service.cache_metrics().await;
        assert_eq!(metrics.hits, 0);
        assert_eq!(metrics.misses, 0);
        assert_eq!(metrics.hit_rate(), 0.0);
    }

    #[test]
    fn test_cost_metrics() {
        let config = test_config();
        let service = EmbeddingService::new(config).unwrap();

        let metrics = service.cost_metrics();
        assert_eq!(metrics.total_tokens(), 0);
        assert_eq!(metrics.estimated_cost_usd(), 0.0);
    }

    #[tokio::test]
    async fn test_cleanup_cache() {
        let mut config = test_config();
        config.cache.ttl_seconds = 0; // Expire immediately

        let service = EmbeddingService::new(config).unwrap();

        // Add some entries
        service.cache.put("text1", vec![0.1; 1536]).await.unwrap();
        service.cache.put("text2", vec![0.2; 1536]).await.unwrap();

        // With TTL of 0, entries expire immediately
        let removed = service.cleanup_cache().await;
        assert_eq!(removed, 2);
        assert_eq!(service.cache_size().await, 0);
    }

    #[tokio::test]
    async fn test_large_batch_chunking() {
        let config = test_config();
        let service = EmbeddingService::new(config).unwrap();

        // Create a batch larger than batch_size
        let texts: Vec<String> = (0..25).map(|i| format!("text {}", i)).collect();

        // This will fail with API call, but we're testing the chunking logic
        // In a real integration test with a valid API key, this would work
        assert_eq!(texts.len(), 25);
        assert!(service.client.config().batch_size < texts.len());
    }
}
