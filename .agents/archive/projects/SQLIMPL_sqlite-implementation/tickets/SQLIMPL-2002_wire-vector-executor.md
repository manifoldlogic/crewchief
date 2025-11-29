# Ticket: SQLIMPL-2002: Wire Vector Executor to SqliteStore

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - all vector tests passing (20+ tests)
- [x] **Verified** - by the verify-ticket agent

## Agents
- rust-indexer-engineer
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary
Wire the Vector executor to the existing `SqliteStore::search_chunks_vector()` method. This is a DELEGATION task - do NOT write new SQL; call the existing method and convert return types.

## Background
The Vector executor at `src/search/vector.rs:112` currently returns empty results with a TODO comment. However, `SqliteStore` already has a fully working vector similarity implementation using sqlite-vec at `src/db/sqlite/mod.rs:750-856`.

**Key Insight:** This is wiring, not reimplementation. The sqlite-vec SQL exists and works.

This ticket implements Plan Phase 2, Ticket 2002: "Wire Vector Executor to SqliteStore".

## Acceptance Criteria
- [x] `VectorExecutor::execute()` calls `SqliteStore::search_vector_by_id()` (new method added)
- [x] `SearchHit` results are converted to `RankedResult` format
- [x] Uses existing `distance_to_similarity()` for score normalization
- [x] TODO comment and placeholder return removed from `vector.rs:112`
- [x] Vector search returns results when embeddings exist in database
- [x] Vector search tests (from Phase 1) now pass (20+ tests)

## Technical Requirements
- **DELEGATE, don't reimplement:** Call `store.search_chunks_vector()`, not raw SQL
- Use existing helper: `distance_to_similarity()` from `src/db/sqlite/vector.rs:30-34`
- Convert `SearchHit` → `RankedResult` with proper score handling
- Vector search requires embeddings to be generated first (via `generate-embeddings` command)

## Implementation Notes

### Current Code (to replace)
```rust
// src/search/vector.rs:112
// TODO(IDXABS-2003): This is a placeholder implementation.
Ok(RankedResults::empty())
```

### Target Implementation Pattern
```rust
pub async fn execute(&self, query: &SearchQuery) -> Result<RankedResults> {
    // Get query embedding first (if not already provided)
    let query_embedding = self.get_or_compute_embedding(&query.text).await?;

    // Delegate to existing SqliteStore method
    let hits = self.store.search_chunks_vector(
        &query.repo,
        query.worktree.as_deref(),
        &query_embedding,
        query.limit as i64,
        false,  // debug
    ).await?;

    // Convert SearchHit to RankedResult
    // Score is already similarity (0-1) from SqliteStore
    let results: Vec<RankedResult> = hits.into_iter().map(|hit| {
        RankedResult {
            chunk_id: hit.chunk_id,
            score: hit.score,  // Already converted via distance_to_similarity
            source: SearchSource::Vector,
            // ... other fields
        }
    }).collect();

    Ok(RankedResults::new(results, SearchSource::Vector))
}
```

### Score Conversion (already in SqliteStore)
```rust
// src/db/sqlite/vector.rs:30-34
fn distance_to_similarity(distance: f64) -> f64 {
    1.0 / (1.0 + distance)
}
```

### Verification
After implementation:
```bash
# First ensure embeddings exist
cargo run -p crewchief-maproom -- generate-embeddings --repo test

# Then search should return semantic results
cargo run -p crewchief-maproom -- search "authentication flow"
```

## Dependencies
- Phase 1 Complete (tests compile)
- Embeddings must be generated for vector search to return results

## Risk Assessment
- **Risk**: Query embedding generation may fail
  - **Mitigation**: Handle embedding service errors gracefully
- **Risk**: No embeddings in database returns empty results
  - **Mitigation**: This is expected behavior; document in acceptance criteria

## Files/Packages Affected
- `crates/maproom/src/search/vector.rs` (primary)
