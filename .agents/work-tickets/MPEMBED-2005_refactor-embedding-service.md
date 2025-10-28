# Ticket: MPEMBED-2005: Refactor EmbeddingService to use provider abstraction

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - related tests pass
- [ ] **Verified** - by the verify-ticket agent

## Agents
- embeddings-engineer
- rust-indexer-engineer
- test-runner
- verify-ticket
- commit-ticket

## Summary
Update EmbeddingService to use `Box<dyn EmbeddingProvider>` instead of hardcoded OpenAIClient, preserving all existing API and behavior.

## Background
The current EmbeddingService has a `client: OpenAIClient` field. We need to change it to `provider: Box<dyn EmbeddingProvider>` to support multiple embedding providers.

The service layer should be provider-agnostic (doesn't know if Ollama or OpenAI). The caching layer remains provider-agnostic as it caches by content hash.

This ticket completes Phase 2: Provider Abstraction from the MPEMBED multi-provider embedding support plan.

## Acceptance Criteria
- [ ] `EmbeddingService` struct has `provider: Box<dyn EmbeddingProvider>` field
- [ ] `EmbeddingService::new()` accepts `Box<dyn EmbeddingProvider>`
- [ ] `EmbeddingService::from_env()` uses factory to create provider
- [ ] `embed_text()` method delegates to `provider.embed()`
- [ ] `embed_batch()` method delegates to `provider.embed_batch()`
- [ ] `dimension()` method added (returns `provider.dimension()`)
- [ ] `provider_name()` method added (returns `provider.provider_name()`)
- [ ] Caching layer remains unchanged (provider-agnostic)
- [ ] Existing tests updated to use trait objects

## Technical Requirements
- File location: `crates/maproom/src/embedding/service.rs` (MODIFY)
- Replace `client: OpenAIClient` with `provider: Box<dyn EmbeddingProvider>`
- Update `from_env()` to call `create_provider_from_env()` factory
- Add convenience methods: `dimension()`, `provider_name()`
- Update existing tests to use mock providers or test doubles
- Preserve all existing error handling and retry logic

## Implementation Notes

```rust
// crates/maproom/src/embedding/service.rs (MODIFY)

use std::sync::Arc;
use crate::embedding::provider::EmbeddingProvider;
use crate::embedding::cache::EmbeddingCache;
use crate::embedding::factory::create_provider_from_env;
use crate::embedding::error::EmbeddingError;

pub struct EmbeddingService {
    provider: Box<dyn EmbeddingProvider>,
    cache: Arc<EmbeddingCache>,
}

impl EmbeddingService {
    /// Create new service with given provider and cache.
    pub fn new(provider: Box<dyn EmbeddingProvider>, cache: Arc<EmbeddingCache>) -> Self {
        Self { provider, cache }
    }

    /// Create service from environment configuration.
    pub async fn from_env() -> Result<Self, EmbeddingError> {
        let provider = create_provider_from_env().await?;
        let cache_config = CacheConfig::from_env()?;
        let cache = EmbeddingCache::new(cache_config)?;
        Ok(Self::new(provider, Arc::new(cache)))
    }

    /// Get embedding dimension for current provider.
    pub fn dimension(&self) -> usize {
        self.provider.dimension()
    }

    /// Get provider name.
    pub fn provider_name(&self) -> &str {
        self.provider.provider_name()
    }

    /// Generate embedding for a single text.
    pub async fn embed_text(&self, text: String) -> Result<Vec<f32>, EmbeddingError> {
        // Check cache first (provider-agnostic)
        let hash = self.cache.hash(&text);
        if let Some(cached) = self.cache.get(&hash) {
            return Ok(cached);
        }

        // Generate embedding via provider
        let embedding = self.provider.embed(text).await?;

        // Store in cache
        self.cache.set(hash, embedding.clone());

        Ok(embedding)
    }

    /// Generate embeddings for a batch of texts.
    pub async fn embed_batch(&self, texts: Vec<String>) -> Result<Vec<Vec<f32>>, EmbeddingError> {
        // Similar caching logic for batch
        // TODO: Implement per-text caching in batch operations
        self.provider.embed_batch(texts).await
    }
}
```

**Key Changes**:
- Replace concrete type with trait object
- Delegate to provider methods
- Add convenience methods for provider metadata
- Preserve caching behavior (provider-agnostic)

**Testing Strategy**:
- Update existing tests to use mock providers
- Test with both Ollama and OpenAI providers
- Verify caching works across provider swaps
- Verify dimension/name methods work correctly

## Dependencies
- MPEMBED-2004 (factory implementation)

## Risk Assessment
- **Risk**: Changing field type breaks existing code using EmbeddingService
  - **Mitigation**: Preserve public API, only internals change
- **Risk**: Dynamic dispatch adds performance overhead
  - **Mitigation**: Embedding API calls dominate runtime, dispatch overhead negligible
- **Risk**: Tests may need significant refactoring
  - **Mitigation**: Use mock providers implementing trait for easier testing

## Files/Packages Affected
- crates/maproom/src/embedding/service.rs (modify)
- crates/maproom/tests/embedding_service_test.rs (modify)
