# Ticket: BLOBSHA-3002: Implement Cache-Aware Upsert with Metrics

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - related tests pass
- [ ] **Verified** - by the verify-ticket agent

## Agents
- rust-indexer-engineer
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary
Implement cache-aware chunk upsert logic that checks code_embeddings for existing blob SHA before generating embeddings, and add CacheMetrics tracking for hits/misses/cost.

## Background
This ticket implements Steps 3.2-3.3 from the BLOBSHA project plan (planning/plan.md, lines 331-439). This is the core deduplication logic - before generating an expensive embedding ($0.00002), check if blob SHA already exists in code_embeddings table. If yes (cache hit), reuse existing embedding. If no (cache miss), generate and store. Metrics track cache effectiveness, which should be 70-90% for typical branch overlaps.

This is the heart of the content-addressed storage optimization. By checking for existing embeddings before generation, we avoid redundant API calls and significantly reduce costs for repositories with multiple branches containing similar code.

## Acceptance Criteria
- [ ] Function `upsert_chunk_with_cache()` implemented in `crates/maproom/src/upsert.rs`
- [ ] Cache check implemented: query code_embeddings for blob_sha before generating embedding
- [ ] Cache hit path: log hit, skip embedding generation, insert/update chunk with existing blob_sha
- [ ] Cache miss path: log miss, generate embedding, insert into code_embeddings, insert/update chunk
- [ ] Use `ON CONFLICT (blob_sha) DO NOTHING` for code_embeddings insert (handles concurrent inserts)
- [ ] CacheMetrics struct implemented in `crates/maproom/src/metrics.rs` with:
  - `cache_hits: AtomicU64`
  - `cache_misses: AtomicU64`
  - `record_hit()`, `record_miss()`, `hit_rate()`, `report()` methods
- [ ] Metrics logged at end of scan operations with format from planning/architecture.md lines 457-465
- [ ] No duplicate embeddings generated for same content

## Technical Requirements

### Upsert Logic (from planning/architecture.md lines 365-421)
The cache-aware upsert follows this sequence:
1. Compute blob_sha from chunk content
2. Check if embedding exists in code_embeddings
3. If not exists, generate embedding and insert
4. Insert/update chunk with blob_sha reference

### Database Operations
- Use parameterized queries (sqlx::query_scalar! macro)
- Cache check query:
  ```rust
  let embedding_exists = sqlx::query_scalar!(
      "SELECT EXISTS(SELECT 1 FROM code_embeddings WHERE blob_sha = $1)",
      blob_sha
  ).fetch_one(pool).await?.unwrap_or(false);
  ```
- Use `ON CONFLICT (blob_sha) DO NOTHING` for code_embeddings insert to handle concurrent writes

### Metrics Implementation (from planning/architecture.md lines 428-454)
- Use AtomicU64 for thread-safe counters
- Use `Ordering::Relaxed` for performance (exact ordering not critical for metrics)
- Track cache hits, misses, and derive hit rate
- Calculate estimated cost: `embeddings_generated * 0.00002` (OpenAI text-embedding-3-small pricing)

### Log Format
```
[INFO] Indexing complete:
  - Chunks processed: 10,000
  - Cache hits: 8,000 (80%)
  - Cache misses: 2,000 (20%)
  - Embeddings generated: 2,000
  - Estimated cost: $0.04
```

## Implementation Notes

### Cache Check Strategy
The cache check is a simple EXISTS query that returns a boolean. This is more efficient than fetching the actual embedding data since we only need to know if it exists.

### Thread Safety
Metrics must be thread-safe since multiple chunks may be processed concurrently. AtomicU64 provides lock-free thread-safe counters with minimal overhead. Using `Ordering::Relaxed` is acceptable because:
- Metrics don't need strict ordering guarantees
- We only read final values after all processing completes
- Relaxed ordering has better performance than SeqCst

### Cost Calculation
Cost is based on OpenAI's text-embedding-3-small pricing at $0.00002 per embedding. This is used for reporting only and should accurately reflect actual API costs incurred during the indexing run.

### Integration Points
- Integrate with existing scan/upsert commands in `crates/maproom/src/cli.rs`
- Metrics should be displayed after scan completion
- Replace existing direct embedding generation with cache-aware upsert function

### Error Handling
- Database errors should propagate normally
- Embedding generation failures should be logged and propagated
- Concurrent insert conflicts are handled by `ON CONFLICT DO NOTHING` clause

## Dependencies
- BLOBSHA-3001 (search queries updated to join code_embeddings table)
- BLOBSHA-2001 (code_embeddings table exists)
- BLOBSHA-2002 (Phase 2 test suite passed, ensuring table structure is validated)
- Existing `generate_embedding()` function for cache misses

## Risk Assessment
- **Risk**: Race condition - two concurrent processes generate embedding for same blob SHA
  - **Mitigation**: `ON CONFLICT DO NOTHING` handles duplicates gracefully, at worst generates one extra embedding but won't corrupt data. Last write wins for chunks table.

- **Risk**: Cache metrics inaccurate due to concurrency
  - **Mitigation**: AtomicU64 provides lock-free thread-safe counters, ensuring accurate counts even with concurrent updates

- **Risk**: Logging overhead slows down processing
  - **Mitigation**: Log only summary at end of scan, not per-chunk. Individual hit/miss logging should use debug level only.

- **Risk**: EXISTS query adds latency to upsert path
  - **Mitigation**: EXISTS is optimized by PostgreSQL and uses index on blob_sha (primary key). Overhead is minimal compared to embedding generation cost.

## Files/Packages Affected
- **MODIFY**: `crates/maproom/src/upsert.rs` (implement cache-aware upsert logic)
- **NEW**: `crates/maproom/src/metrics.rs` (CacheMetrics struct and implementation)
- **MODIFY**: `crates/maproom/src/lib.rs` (export metrics module)
- **MODIFY**: `crates/maproom/src/cli.rs` (integrate metrics display after scan completion)
