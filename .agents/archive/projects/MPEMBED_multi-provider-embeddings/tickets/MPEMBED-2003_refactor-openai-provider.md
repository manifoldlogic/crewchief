# Ticket: MPEMBED-2003: Refactor OpenAIClient to implement EmbeddingProvider trait

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - related tests pass
- [x] **Verified** - by the verify-ticket agent

## Agents
- embeddings-engineer
- rust-indexer-engineer
- test-runner
- verify-ticket
- commit-ticket

## Summary
Wrap existing OpenAIClient to implement the new EmbeddingProvider trait while preserving all existing behavior (caching, retry logic, cost tracking).

## Background
The existing `OpenAIClient` works well and has mature error handling and metrics. We need to make it implement `EmbeddingProvider` trait without breaking changes.

Preserving existing cost tracking and metrics is important for monitoring. We must ensure backward compatibility so existing code still works.

This ticket implements the OpenAI provider adapter as part of Phase 2: Provider Abstraction from the MPEMBED multi-provider embedding support plan.

## Acceptance Criteria
- [x] `OpenAIClient` implements `EmbeddingProvider` trait via `#[async_trait]`
- [x] `embed()` method delegates to existing `embed_text()` implementation
- [x] `embed_batch()` method delegates to existing batch implementation
- [x] `dimension()` returns 1536 (text-embedding-3-small)
- [x] `provider_name()` returns "openai"
- [x] `metrics()` returns existing cost tracking data (token count, estimated cost)
- [x] Existing OpenAIClient API unchanged (backward compatible)
- [x] All existing tests still pass

## Technical Requirements
- File location: `crates/maproom/src/embedding/openai.rs` (MODIFY)
- Don't change existing methods, just add trait implementation
- Preserve existing error types (map to EmbeddingError in trait methods)
- Keep existing retry logic, rate limiting, caching
- Ensure trait implementation doesn't break existing direct usage of OpenAIClient

## Implementation Notes

```rust
// crates/maproom/src/embedding/openai.rs (MODIFY)

use async_trait::async_trait;
use crate::embedding::provider::{EmbeddingProvider, Vector, ProviderMetrics};
use crate::embedding::error::EmbeddingError;

// Existing OpenAIClient struct unchanged
pub struct OpenAIClient {
    // ... existing fields ...
}

// Existing implementation methods unchanged
impl OpenAIClient {
    pub async fn embed_text(&self, text: String) -> Result<Vector, OpenAIError> {
        // ... existing implementation ...
    }

    pub async fn embed_batch(&self, texts: Vec<String>) -> Result<Vec<Vector>, OpenAIError> {
        // ... existing implementation ...
    }

    // ... other existing methods ...
}

// NEW: Implement trait by delegating to existing methods
#[async_trait]
impl EmbeddingProvider for OpenAIClient {
    async fn embed(&self, text: String) -> Result<Vector, EmbeddingError> {
        self.embed_text(text)
            .await
            .map_err(|e| EmbeddingError::ProviderError(e.to_string()))
    }

    async fn embed_batch(&self, texts: Vec<String>) -> Result<Vec<Vector>, EmbeddingError> {
        OpenAIClient::embed_batch(self, texts)
            .await
            .map_err(|e| EmbeddingError::ProviderError(e.to_string()))
    }

    fn dimension(&self) -> usize {
        1536 // text-embedding-3-small
    }

    fn provider_name(&self) -> &'static str {
        "openai"
    }

    fn metrics(&self) -> Option<ProviderMetrics> {
        Some(ProviderMetrics {
            total_requests: self.metrics.total_requests(),
            total_tokens: self.metrics.total_tokens(),
            failed_requests: self.metrics.failed_requests(),
            estimated_cost_usd: self.metrics.estimated_cost_usd(),
        })
    }
}
```

**Key Points**:
- Only add trait implementation, don't modify existing API
- Delegate to existing methods to preserve behavior
- Map existing error types to EmbeddingError in trait methods
- Preserve all existing tests

## Dependencies
- MPEMBED-2001 (trait definition)

## Risk Assessment
- **Risk**: Trait implementation breaks existing code using OpenAIClient directly
  - **Mitigation**: Only add trait implementation, don't modify existing API surface
- **Risk**: Error type conversion loses important error context
  - **Mitigation**: Include full error message in EmbeddingError::ProviderError

## Files/Packages Affected
- crates/maproom/src/embedding/client.rs (modified - note: file is client.rs not openai.rs)

## Implementation Notes

### Completed Changes
- ✅ Added `async_trait` and `EmbeddingProvider` imports to client.rs
- ✅ Implemented `EmbeddingProvider` trait for `OpenAIClient` (lines 466-516)
- ✅ All trait methods delegate to existing implementations:
  - `embed()` → `embed_text()`
  - `embed_batch()` → `embed_batch()`
  - `dimension()` → returns `self.config.dimension` (1536 for text-embedding-3-small)
  - `provider_name()` → returns `"openai"`
  - `metrics()` → converts `CostMetrics` to `ProviderMetrics`
- ✅ Added three new tests to verify trait implementation:
  - `test_embedding_provider_trait_implementation()` - tests trait methods directly
  - `test_embedding_provider_trait_object()` - tests dynamic dispatch with `Box<dyn EmbeddingProvider>`
  - `test_provider_metrics_conversion()` - tests metrics conversion from CostMetrics to ProviderMetrics
- ✅ All 86 embedding tests pass (11 client tests + 75 other embedding tests)
- ✅ Backward compatibility maintained - existing API unchanged

### Key Design Decisions
1. **File Location**: The OpenAI client is in `client.rs` not `openai.rs` as mentioned in ticket
2. **Dimension**: Uses `self.config.dimension` which defaults to 1536 but can be configured
3. **Error Handling**: No error conversion needed - `embed_text()` and `embed_batch()` already return `EmbeddingError`
4. **Metrics Conversion**: Trait's `metrics()` creates new `ProviderMetrics` struct from existing `CostMetrics` atomic counters

### Test Coverage
- Existing 8 client tests all pass (backward compatibility verified)
- New 3 trait implementation tests pass
- Total: 86 embedding module tests pass
