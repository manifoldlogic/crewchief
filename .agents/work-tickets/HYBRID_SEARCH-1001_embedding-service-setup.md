# Ticket: HYBRID_SEARCH-1001: Embedding Service Setup

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - related tests pass
- [ ] **Verified** - by the verify-ticket agent

## Agents
- embeddings-engineer
- test-runner
- verify-ticket
- commit-ticket

## Summary
Set up the embedding service infrastructure for the hybrid search system, including OpenAI client integration, LRU caching, retry logic, and batch processing pipeline to support efficient embedding generation for code and text chunks.

## Background
The hybrid search system requires semantic embeddings to power vector similarity search alongside full-text search. This foundational ticket establishes the embedding generation infrastructure that will be used throughout the project to create embeddings for existing chunks and new content.

Key requirements:
- Use text-embedding-3-small model (1536 dimensions) for cost efficiency
- Implement robust caching to minimize API costs (target <$100/month)
- Support batch processing for efficient bulk operations
- Provide extensible architecture to support future providers (local models, Cohere, etc.)

This is Phase 1, Week 1, Task 1 from the HYBRID_SEARCH plan and is a prerequisite for all subsequent phases.

## Acceptance Criteria
- [ ] Embedding service successfully generates embeddings using text-embedding-3-small model
- [ ] Retry logic handles API failures gracefully with exponential backoff
- [ ] LRU cache implemented with target >80% hit rate on typical workloads
- [ ] Batch processing pipeline supports efficient bulk embedding generation
- [ ] Configuration system allows switching between providers (OpenAI, local, future)
- [ ] Cost tracking metrics implemented to monitor API usage
- [ ] Unit tests cover cache behavior, retry logic, and error handling
- [ ] Integration test validates end-to-end embedding generation

## Technical Requirements

### Core Service Implementation
- Create `EmbeddingService` struct with async methods
- Implement `embed_text(&self, text: &str) -> Result<Vector>` method
- Use `Arc<RwLock<LruCache<String, Vector>>>` for thread-safe caching
- Support vector dimension of 1536 (text-embedding-3-small)

### OpenAI Client Integration
- Configure OpenAI client with API key from environment
- Use `text-embedding-3-small` model for all embeddings
- Implement request batching (max 100 texts per batch recommended)
- Handle rate limits and quota errors

### Retry Logic
- Implement exponential backoff for transient failures
- Max 3 retries with delays: 1s, 2s, 4s
- Distinguish between retryable (500, 429) and non-retryable errors (401, 400)
- Log retry attempts for monitoring

### Caching Strategy
- LRU cache with configurable size (default: 10,000 entries)
- Cache key: hash of input text
- TTL: 3600 seconds (configurable)
- Thread-safe read/write access
- Cache hit/miss metrics

### Configuration Schema
```yaml
embedding:
  provider: openai  # openai|cohere|local
  model: text-embedding-3-small
  dimension: 1536
  cache_size: 10000
  cache_ttl: 3600
  batch_size: 100
  retry_max_attempts: 3
```

### Error Handling
- Return `Result<Vector, EmbeddingError>` with detailed error types
- Custom error types: `ApiError`, `CacheError`, `ConfigError`
- Graceful degradation: log errors but don't crash service

## Implementation Notes

### File Structure
```
crates/maproom/src/embedding/
├── mod.rs           # Module exports
├── service.rs       # Main EmbeddingService implementation
├── client.rs        # OpenAI client wrapper
├── cache.rs         # LRU cache implementation
├── config.rs        # Configuration structures
└── error.rs         # Error types and handling
```

### Architecture Considerations
- Reference architecture document lines 83-117 for service design
- Reference configuration schema lines 263-294 for config structure
- Use tokio for async runtime
- Consider future provider extensibility via trait abstraction

### Performance Targets
- Single embedding generation: <100ms p95
- Batch processing: <500ms for 100 embeddings
- Cache lookup: <1ms
- Memory usage: ~500MB for 10k cached embeddings

### Dependencies
Add to `Cargo.toml`:
```toml
tokio = { version = "1", features = ["full"] }
reqwest = { version = "0.11", features = ["json"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
lru = "0.12"
anyhow = "1"
thiserror = "1"
```

## Dependencies
- None (foundational ticket - Phase 1, Week 1, Task 1)

## Risk Assessment
- **Risk**: OpenAI API rate limits or quota exhaustion
  - **Mitigation**: Implement robust retry logic, caching, and monitoring. Document fallback to local embedding models.

- **Risk**: Embedding quality issues with text-embedding-3-small
  - **Mitigation**: Track embedding quality metrics. Architecture supports switching to text-embedding-3-large if needed.

- **Risk**: Cost overrun from API usage
  - **Mitigation**: Aggressive caching (>80% hit rate target), cost tracking metrics, alerts at $80/month threshold.

- **Risk**: Cache memory usage exceeds available resources
  - **Mitigation**: Configurable cache size with sensible defaults. LRU eviction prevents unbounded growth.

## Files/Packages Affected
- `crates/maproom/src/embedding/mod.rs` (new)
- `crates/maproom/src/embedding/service.rs` (new)
- `crates/maproom/src/embedding/client.rs` (new)
- `crates/maproom/src/embedding/cache.rs` (new)
- `crates/maproom/src/embedding/config.rs` (new)
- `crates/maproom/src/embedding/error.rs` (new)
- `crates/maproom/Cargo.toml` (dependencies)
- `crates/maproom/tests/embedding_integration.rs` (new)

## Planning References
- Architecture: `/workspace/crewchief_context/maproom/HYBRID_SEARCH/HYBRID_SEARCH_ARCHITECTURE.md`
  - Embedding Service section (lines 83-117)
  - Configuration Schema section (lines 263-294)
- Plan: `/workspace/crewchief_context/maproom/HYBRID_SEARCH/HYBRID_SEARCH_PLAN.md`
  - Phase 1: Embedding Infrastructure (lines 6-34)
