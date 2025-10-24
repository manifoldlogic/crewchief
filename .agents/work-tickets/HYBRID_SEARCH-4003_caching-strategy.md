# Ticket: HYBRID_SEARCH-4003: Multi-Layer Caching Strategy

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - related tests pass
- [ ] **Verified** - by the verify-ticket agent

## Agents
- performance-engineer
- caching-engineer
- test-runner (e.g. unit-test-runner)
- verify-ticket
- commit-ticket

## Summary
Implement a comprehensive multi-layer caching strategy for the hybrid search system to improve query performance and reduce computational overhead. The cache system will include query result caching, embedding caching, cache warming on startup, intelligent invalidation logic, and comprehensive monitoring to achieve >60% cache hit rates.

## Background
The hybrid search system performs computationally expensive operations including embedding generation, vector similarity searches, and full-text searches. Without caching, every query incurs these costs. A multi-layer caching strategy is critical for:

1. **Performance**: Reduce query latency from ~100ms to <20ms for cached queries
2. **Resource Efficiency**: Minimize embedding API calls and database queries
3. **Scalability**: Handle higher query volumes without proportional infrastructure costs
4. **User Experience**: Provide near-instant results for common queries

This is Phase 4, Week 4, Task 3 of the HYBRID_SEARCH project plan, building upon the embedding infrastructure (HYBRID_SEARCH-1001) and query result caching foundation (HYBRID_SEARCH-4001).

## Acceptance Criteria
- [ ] Multi-layer cache implemented with query_cache and embedding_cache
- [ ] Cache warming on startup preloads popular queries
- [ ] Cache invalidation logic triggers on database updates (incremental indexing)
- [ ] Cache hit rate exceeds 60% under normal query load
- [ ] Total memory usage for all caches remains under 500MB
- [ ] LRU eviction policy implemented with configurable size (10,000 entries default)
- [ ] TTL support implemented (3600s default)
- [ ] CacheStats monitoring tracks hits, misses, evictions, and hit rate
- [ ] Cache warming completes within 30 seconds on startup
- [ ] Cache invalidation properly handles partial updates without full cache clear

## Technical Requirements

### Multi-Layer Cache Architecture
- Implement `SearchCache` struct with:
  - `query_cache: Arc<RwLock<LruCache<String, SearchResults>>>` - Full query results
  - `embedding_cache: Arc<RwLock<LruCache<String, Vector>>>` - Generated embeddings
  - `stats: Arc<CacheStats>` - Cache performance metrics
- Use async RwLock for concurrent read access
- Support configurable cache sizes and TTLs from `hybrid_search.yaml` config

### Cache Key Design
- Query cache key: Hash of (query text, filters, limit, offset, fusion method)
- Embedding cache key: Hash of normalized query text
- Keys must be deterministic and collision-resistant (use SHA-256 or SipHash)

### LRU Eviction Policy
- Use `lru` crate for LRU cache implementation
- Configurable max size (default: 10,000 entries per cache layer)
- Evict least recently used entries when size limit reached
- Track eviction count in CacheStats

### TTL Support
- Store timestamp with each cache entry
- Check TTL on cache read operations
- Default TTL: 3600s (1 hour)
- Configurable per cache layer via config file

### Cache Warming
- On startup, load top N popular queries from query log or analytics
- Pre-compute and cache embeddings for common query patterns
- Support warmup query list in config file
- Log warming progress and completion time
- Target: Complete warming within 30 seconds

### Cache Invalidation
- Trigger invalidation on incremental indexing (file updates)
- Selective invalidation: Only clear entries affected by updated files
- Track file-to-query mapping for targeted invalidation
- Option for full cache clear on major schema changes
- Emit invalidation events for monitoring

### Statistics & Monitoring
- Implement `CacheStats` with:
  - `hits: AtomicU64` - Cache hit count
  - `misses: AtomicU64` - Cache miss count
  - `evictions: AtomicU64` - Eviction count
  - `invalidations: AtomicU64` - Invalidation event count
- Calculate hit rate: `hits / (hits + misses)`
- Expose metrics via `/metrics` endpoint
- Log cache performance on regular intervals (every 1000 queries)

### Memory Management
- Monitor total cache memory usage
- Implement memory-based eviction if size approaches 500MB limit
- Use `sysinfo` crate to track process memory
- Alert if memory usage exceeds threshold

## Implementation Notes

### Architecture Reference
See `/workspace/crewchief_context/maproom/HYBRID_SEARCH/HYBRID_SEARCH_ARCHITECTURE.md`:
- Caching Strategy section (lines 343-379)
- Caching Layers section (lines 435-439)
- Configuration settings (lines 270-272)

### Implementation Approach

1. **Core Cache Module** (`src/cache/search_cache.rs`):
   - Implement `SearchCache` struct with dual-layer caching
   - Provide `get_or_compute` pattern for transparent cache usage
   - Handle concurrent access with RwLock
   - Support async operations throughout

2. **Cache Warming** (`src/cache/warming.rs`):
   - Load popular queries from config or query log
   - Pre-generate embeddings for common patterns
   - Run warming async on startup (don't block server start)
   - Report warming statistics

3. **Cache Invalidation** (`src/cache/invalidation.rs`):
   - Subscribe to file update events from indexer
   - Implement selective invalidation based on file paths
   - Maintain file-to-cache-key mapping
   - Support manual cache clear via CLI/API

4. **Statistics Tracking** (`src/cache/stats.rs`):
   - Implement atomic counters for thread-safe stats
   - Provide hit rate calculation
   - Export metrics in Prometheus format
   - Log periodic summaries

### Key Technical Considerations

- **Thread Safety**: Use Arc<RwLock<>> for shared cache access across async tasks
- **Serialization**: Cache entries must be Clone or use Arc for efficient storage
- **Key Normalization**: Normalize query text (lowercase, trim) before hashing
- **Memory Estimation**: Track approximate memory per entry for accurate limits
- **Graceful Degradation**: If cache is full or slow, fall back to direct computation
- **Cold Start**: Handle cache misses gracefully during initial warmup period

### Configuration Example
```yaml
hybrid_search:
  cache:
    query_cache:
      enabled: true
      max_size: 10000
      ttl_seconds: 3600
    embedding_cache:
      enabled: true
      max_size: 10000
      ttl_seconds: 7200
    warming:
      enabled: true
      timeout_seconds: 30
      queries_file: "config/popular_queries.txt"
    memory_limit_mb: 500
```

### Testing Strategy
- Unit tests: LRU eviction, TTL expiration, key generation
- Integration tests: Cache warming, invalidation triggers, concurrent access
- Performance tests: Hit rate measurement, memory usage tracking
- Benchmark: Compare cached vs uncached query performance

## Dependencies
- **HYBRID_SEARCH-1001**: Embedding service setup - Extends existing embedding cache
- **HYBRID_SEARCH-4001**: Query result caching foundation - Builds on basic caching
- **External Crates**:
  - `lru = "0.12"` - LRU cache implementation
  - `sysinfo = "0.30"` - Memory monitoring
  - `serde` - Cache entry serialization
  - `tokio` - Async runtime for cache operations

## Risk Assessment

- **Risk**: Cache invalidation too aggressive, causing low hit rates
  - **Mitigation**: Implement selective invalidation based on affected files only. Monitor hit rate and adjust invalidation granularity. Add metrics to track invalidation frequency.

- **Risk**: Memory usage exceeds 500MB limit under high load
  - **Mitigation**: Implement memory-based eviction alongside LRU. Monitor memory usage actively. Add circuit breaker to disable caching if memory critical.

- **Risk**: Cache warming delays server startup
  - **Mitigation**: Run warming async, don't block server start. Set timeout (30s). Cache warming failures should log warning but not crash server.

- **Risk**: Stale cache entries after database updates
  - **Mitigation**: TTL ensures eventual consistency. Invalidation events provide immediate freshness. Add cache version number for schema changes.

- **Risk**: Cache key collisions causing incorrect results
  - **Mitigation**: Use cryptographic hash (SHA-256) for cache keys. Include all query parameters in key generation. Add unit tests for key uniqueness.

- **Risk**: Lock contention on cache access under high concurrency
  - **Mitigation**: Use RwLock for multiple concurrent readers. Consider sharding cache by key prefix. Monitor lock wait times in metrics.

## Files/Packages Affected

### New Files to Create
- `crates/maproom/src/cache/search_cache.rs` - Multi-layer cache implementation
- `crates/maproom/src/cache/warming.rs` - Cache warming on startup
- `crates/maproom/src/cache/invalidation.rs` - Cache invalidation logic
- `crates/maproom/src/cache/stats.rs` - Cache statistics monitoring
- `crates/maproom/src/cache/mod.rs` - Cache module exports

### Existing Files to Modify
- `crates/maproom/src/search/mod.rs` - Integrate SearchCache into search pipeline
- `crates/maproom/src/search/hybrid.rs` - Use cache in hybrid search execution
- `crates/maproom/src/config.rs` - Add cache configuration schema
- `crates/maproom/src/indexer/incremental.rs` - Trigger cache invalidation on updates
- `crates/maproom/Cargo.toml` - Add lru and sysinfo dependencies

### Test Files to Create
- `crates/maproom/tests/cache/lru_test.rs` - LRU eviction tests
- `crates/maproom/tests/cache/ttl_test.rs` - TTL expiration tests
- `crates/maproom/tests/cache/warming_test.rs` - Cache warming tests
- `crates/maproom/tests/cache/invalidation_test.rs` - Invalidation tests
- `crates/maproom/tests/cache/concurrent_test.rs` - Concurrency tests
- `crates/maproom/tests/cache/memory_test.rs` - Memory usage tests

### Configuration Files
- `config/hybrid_search.yaml` - Add cache configuration section
- `config/popular_queries.txt` - Popular queries for cache warming (optional)
