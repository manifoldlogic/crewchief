# Ticket: CONTEXT_ASM-3002: Caching System

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - related tests pass
- [ ] **Verified** - by the verify-ticket agent

## Agents
- performance-engineer
- caching-engineer
- test-runner
- verify-ticket
- commit-ticket

## Summary
Implement a comprehensive caching system for the Context Assembly Engine to improve performance by caching assembled bundles, graph traversals, and token counts. This will reduce redundant computation and database queries for frequently requested context bundles.

## Background
As outlined in Phase 3, Week 5, Task 2 of the CONTEXT_ASM plan, the context assembly process involves expensive operations:
- Recursive graph traversals across chunk relationships
- Token counting for content budgeting
- Bundle assembly with multiple database queries

These operations are often repeated for the same chunks with identical options. A caching layer will significantly improve response times and reduce database load, especially for frequently accessed code sections and in iterative development workflows.

The architecture document specifies a cache table design and configuration parameters (TTL: 3600s, max entries: 1000) to balance performance gains against memory usage and cache staleness.

## Acceptance Criteria
- [ ] Bundle cache implemented with (chunk_id, options_hash) as key
- [ ] Graph traversal results are cached
- [ ] Token counts are cached
- [ ] Cache invalidation works correctly on chunk updates
- [ ] Cache hit rate exceeds 60% in typical usage
- [ ] Cache statistics and monitoring are available
- [ ] TTL and max entries are configurable
- [ ] All cache operations are covered by unit tests

## Technical Requirements
- Implement `context_cache` table per architecture schema (lines 199-206)
- Cache key: combination of `chunk_id` and `options_hash` (hash of ExpandOptions)
- Store assembled bundles as JSONB
- Implement cache eviction based on:
  - TTL (default: 3600 seconds)
  - LRU when max entries (default: 1000) exceeded
- Invalidate cache entries when:
  - Source chunk is updated
  - Related chunks in the bundle are updated
  - Manual cache clear requested
- Provide cache statistics:
  - Hit rate percentage
  - Total hits/misses
  - Current cache size
  - Eviction counts
- Make caching configurable via `context.cache` configuration section

## Implementation Notes

### Database Schema
Create migration for the `context_cache` table as specified in the architecture:
```sql
CREATE TABLE maproom.context_cache (
  chunk_id BIGINT REFERENCES maproom.chunks(id),
  options_hash TEXT,
  bundle JSONB,
  created_at TIMESTAMPTZ DEFAULT NOW(),
  PRIMARY KEY (chunk_id, options_hash)
);
```

### Cache Implementation (`cache.rs`)
- Create `CacheKey` struct combining chunk_id and options_hash
- Implement `hash_options()` to create consistent hash from ExpandOptions
- Implement `get()`, `put()`, `invalidate()` operations
- Use PostgreSQL JSONB for flexible bundle storage
- Implement TTL checks on retrieval
- Track cache statistics (hits, misses, evictions)

### Integration Points
- Modify `assembler.rs` to check cache before assembly
- Store successfully assembled bundles in cache
- Hook into chunk update pipeline for invalidation

### Monitoring (`cache_stats.rs`)
- Track hit/miss ratios
- Monitor cache size and eviction frequency
- Provide statistics query endpoint
- Log cache effectiveness metrics

### Configuration
Reference architecture configuration (lines 226-230):
```yaml
cache:
  enabled: true
  ttl_seconds: 3600
  max_entries: 1000
```

### Performance Considerations
- Cache lookups should be faster than assembly (< 10ms)
- Batch invalidations for related chunks
- Consider read-through cache pattern
- Monitor memory usage of JSONB storage

## Dependencies
- **CONTEXT_ASM-1001** (Basic Assembly Pipeline) - Required for assembler integration
- PostgreSQL database with JSONB support
- Chunk update detection mechanism

## Risk Assessment
- **Risk**: Cache invalidation complexity with related chunks
  - **Mitigation**: Start with conservative invalidation (invalidate on any related chunk update), optimize later if needed

- **Risk**: Memory consumption with large bundles in cache
  - **Mitigation**: Enforce max_entries limit, monitor JSONB storage size, implement size-based eviction if needed

- **Risk**: Cache staleness affecting development workflow
  - **Mitigation**: Implement proper invalidation on chunk updates, provide manual cache clear command, keep TTL reasonable (1 hour default)

- **Risk**: Hash collisions in options_hash
  - **Mitigation**: Use cryptographic hash (SHA-256) for ExpandOptions, include all relevant fields

## Files/Packages Affected
- `crates/maproom/migrations/XXX_create_context_cache.sql` - New cache table migration
- `crates/maproom/src/context/cache.rs` - New cache implementation module
- `crates/maproom/src/context/cache_stats.rs` - New cache statistics module
- `crates/maproom/src/context/assembler.rs` - Modified to use cache
- `crates/maproom/src/context/mod.rs` - Export new cache modules
- `crates/maproom/tests/context/cache_test.rs` - New unit tests for cache
- `crates/maproom/tests/context/assembler_test.rs` - Updated tests for cached assembly
- Configuration schema files - Add cache configuration options
