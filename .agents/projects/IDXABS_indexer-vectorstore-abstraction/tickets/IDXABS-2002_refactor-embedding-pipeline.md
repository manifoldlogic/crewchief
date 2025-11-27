# Ticket: IDXABS-2002: Refactor Embedding Pipeline

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - tests executed and passing (or N/A if no tests)
- [ ] **Verified** - by the verify-ticket agent

**Note on "Tests pass"**:
- Run `cargo check` to verify embedding module compiles
- Test embedding pipeline methods work with SqliteStore
- Verify new methods are implemented correctly

## Agents
- rust-indexer-engineer
- verify-ticket
- commit-ticket

## Summary
Update `embedding/pipeline.rs` to use `&SqliteStore` instead of `&Client`, implementing three missing SqliteStore methods required by the embedding pipeline.

## Background
The embedding pipeline uses raw PostgreSQL queries for operations like counting chunks needing embeddings and fetching chunk data. SqliteStore needs three new methods to support these operations.

**Reference**: Phase 2, Ticket 2002 of `planning/plan.md` - "Refactor Embedding Pipeline"
**Architecture**: See `planning/architecture.md` - Section 4.2 "embedding/pipeline.rs"
**Risk Analysis**: See `planning/analysis.md` - "Embedding pipeline gaps" risk

## Acceptance Criteria
- [ ] `SqliteStore::get_chunks_needing_embeddings_count()` implemented
- [ ] `SqliteStore::copy_existing_embeddings_from_cache()` implemented
- [ ] `SqliteStore::fetch_chunks_needing_embeddings(incremental, sample_size)` implemented
- [ ] `EmbeddingPipeline::run()` works with `&SqliteStore`
- [ ] No raw SQL queries in `embedding/pipeline.rs` (use store methods)
- [ ] No `tokio_postgres` imports in `embedding/` directory
- [ ] `cargo check` passes for embedding module

## Technical Requirements

### New SqliteStore Methods to Implement

1. **`get_chunks_needing_embeddings_count() -> Result<i64>`**
   ```rust
   // Returns count of chunks where code_embedding IS NULL
   SELECT COUNT(*) FROM chunks WHERE code_embedding IS NULL
   ```

2. **`copy_existing_embeddings_from_cache() -> Result<i64>`**
   ```rust
   // Bulk copy embeddings from code_embeddings cache table to chunks
   // Returns number of rows updated
   UPDATE chunks SET code_embedding = (
       SELECT embedding FROM code_embeddings
       WHERE code_embeddings.content_sha = chunks.content_sha
   ) WHERE code_embedding IS NULL
     AND content_sha IN (SELECT content_sha FROM code_embeddings)
   ```

3. **`fetch_chunks_needing_embeddings(incremental: bool, sample_size: Option<usize>) -> Result<Vec<ChunkForEmbedding>>`**
   ```rust
   // Returns chunks that need embeddings
   // If incremental=true and sample_size=Some(n), return random sample
   // ChunkForEmbedding: { id, content_sha, preview_text }
   SELECT id, content_sha, preview FROM chunks
   WHERE code_embedding IS NULL
   [ORDER BY RANDOM() LIMIT ?] -- if sample_size
   ```

### Pipeline Method Changes
```rust
// Before
impl EmbeddingPipeline {
    pub async fn run(&self, client: &Client) -> Result<PipelineStats> {
        let count: i64 = client.query_one(
            "SELECT COUNT(*) FROM chunks WHERE code_embedding IS NULL", &[]
        ).await?.get(0);
    }
}

// After
impl EmbeddingPipeline {
    pub async fn run(&self, store: &SqliteStore) -> Result<PipelineStats> {
        let count = store.get_chunks_needing_embeddings_count().await?;
    }
}
```

## Implementation Notes

### Files to Modify
- `crates/maproom/src/embedding/pipeline.rs` - Main pipeline code
- `crates/maproom/src/db/sqlite/mod.rs` - Add new methods to SqliteStore
- `crates/maproom/src/db/sqlite/embeddings.rs` - May need updates

### Method Implementation Location
Add new methods to `SqliteStore` impl block in `db/sqlite/mod.rs` or `db/sqlite/embeddings.rs`.

### Data Types
```rust
pub struct ChunkForEmbedding {
    pub id: i64,
    pub content_sha: String,
    pub preview: String,
}
```

### Verification
```bash
# Check embedding module compiles
cargo check -p crewchief-maproom --lib 2>&1 | grep -E "embedding|pipeline"

# Verify no raw SQL in pipeline
grep -n "query\|execute" crates/maproom/src/embedding/pipeline.rs
# Should show only store method calls, not raw queries

# Verify no Client references
grep -r "tokio_postgres\|&Client" crates/maproom/src/embedding/
# Should return nothing
```

## Dependencies
- IDXABS-1001 (Delete PostgreSQL Database Files)
- IDXABS-1002 (Simplify db/mod.rs)
- IDXABS-1003 (Update Cargo.toml)
- IDXABS-2001 (Refactor Indexer Module) - conceptual dependency

## Risk Assessment
- **Risk**: New SqliteStore methods have incorrect SQL
  - **Mitigation**: Test with actual data after implementation
  - **Mitigation**: Compare behavior to original PostgreSQL queries
- **Risk**: Transaction handling differs between PostgreSQL and SQLite
  - **Mitigation**: Use SqliteStore's existing transaction patterns
- **Risk**: Performance regression on large chunk counts
  - **Mitigation**: SQLite adequate for typical repository sizes
  - **Mitigation**: Add LIMIT to fetch query for incremental processing

## Files/Packages Affected
Files to MODIFY:
- `crates/maproom/src/embedding/pipeline.rs` - Change `&Client` to `&SqliteStore`
- `crates/maproom/src/db/sqlite/mod.rs` - Add new methods
- `crates/maproom/src/db/sqlite/embeddings.rs` - May need supporting methods
