# Ticket: MPEMBED-2001: Define EmbeddingProvider trait with object-safe methods

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - related tests pass
- [ ] **Verified** - by the verify-ticket agent

## Agents
- provider-abstraction-architect
- rust-indexer-engineer
- test-runner
- verify-ticket
- commit-ticket

## Summary
Create the core abstraction for embedding providers with async methods for single/batch embedding, dimension reporting, and optional metrics.

## Background
The current system uses hardcoded `OpenAIClient` directly. To support multiple providers (Ollama with 768-dim, Google with 768-dim, OpenAI with 1536-dim), we need a trait-based abstraction.

The trait must be object-safe for `Box<dyn EmbeddingProvider>` dynamic dispatch and support async/await via `async-trait` crate.

This ticket implements the foundation for Phase 2: Provider Abstraction from the MPEMBED multi-provider embedding support plan.

## Acceptance Criteria
- [ ] `EmbeddingProvider` trait defined with object-safe methods
- [ ] `embed(&self, text: String) -> Result<Vector, EmbeddingError>` method declared
- [ ] `embed_batch(&self, texts: Vec<String>) -> Result<Vec<Vector>, EmbeddingError>` method declared
- [ ] `dimension(&self) -> usize` method declared (returns 768 or 1536)
- [ ] `provider_name(&self) -> &'static str` method declared (returns "ollama", "google", "openai")
- [ ] `metrics(&self) -> Option<ProviderMetrics>` optional method declared
- [ ] Trait requires `Send + Sync` bounds for async safety
- [ ] Documentation includes trait contract (invariants, thread safety)

## Technical Requirements
- File location: `crates/maproom/src/embedding/provider.rs` (NEW FILE)
- Use `#[async_trait]` from async-trait crate
- All methods use `&self` (not `&mut self`) for object safety
- Return types must be concrete (not associated types) for object safety
- Vector type: `Vec<f32>` (consistent with existing code)
- Default implementation for `embed_batch()` that calls `embed()` sequentially
- Providers with native batching should override `embed_batch()`

## Implementation Notes

```rust
// crates/maproom/src/embedding/provider.rs (NEW FILE)

use async_trait::async_trait;
use crate::embedding::error::EmbeddingError;

pub type Vector = Vec<f32>;

/// Abstract embedding provider interface.
///
/// # Object Safety
/// This trait is object-safe and can be used with `Box<dyn EmbeddingProvider>`.
///
/// # Thread Safety
/// All implementations must be Send + Sync for use in async contexts.
///
/// # Invariants
/// - `dimension()` must return consistent value for provider lifetime
/// - `embed()` output length must equal `dimension()`
/// - `embed_batch()` output length must equal input length
#[async_trait]
pub trait EmbeddingProvider: Send + Sync {
    /// Generate embedding for a single text.
    ///
    /// # Errors
    /// Returns error if API call fails or text is too long for model context.
    async fn embed(&self, text: String) -> Result<Vector, EmbeddingError>;

    /// Generate embeddings for a batch of texts.
    ///
    /// # Implementation Note
    /// Default implementation calls `embed()` sequentially.
    /// Providers with native batching should override this.
    async fn embed_batch(&self, texts: Vec<String>) -> Result<Vec<Vector>, EmbeddingError> {
        let mut results = Vec::with_capacity(texts.len());
        for text in texts {
            results.push(self.embed(text).await?);
        }
        Ok(results)
    }

    /// Get the embedding dimension for this provider.
    fn dimension(&self) -> usize;

    /// Get the provider name ("ollama", "google", "openai").
    fn provider_name(&self) -> &'static str;

    /// Get provider-specific metrics (optional).
    fn metrics(&self) -> Option<ProviderMetrics> {
        None
    }
}

/// Metrics about provider performance and cost.
#[derive(Debug, Clone, Default)]
pub struct ProviderMetrics {
    pub total_requests: u64,
    pub total_tokens: u64,
    pub failed_requests: u64,
    pub estimated_cost_usd: f64,
}
```

## Dependencies
- MPEMBED-0003 (async-trait dependency added)

## Risk Assessment
- **Risk**: Trait not object-safe due to async methods
  - **Mitigation**: async-trait crate handles this via proc macro transformation
- **Risk**: Generic trait design may not fit future provider needs
  - **Mitigation**: Trait methods are extensible via default implementations

## Files/Packages Affected
- crates/maproom/src/embedding/provider.rs (create)
- crates/maproom/src/embedding/mod.rs (modify - add `pub mod provider;`)
- crates/maproom/src/embedding/error.rs (create if doesn't exist)
