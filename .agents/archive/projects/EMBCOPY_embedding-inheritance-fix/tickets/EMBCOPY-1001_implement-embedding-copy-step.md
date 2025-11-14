# Ticket: EMBCOPY-1001: Implement Embedding Copy Step in Pipeline

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - tests executed and passing (or N/A if no tests)
- [x] **Verified** - by the verify-ticket agent

**Note on "Tests pass"**:
- If tests were created/modified, you MUST run them and show output
- "Tests pass" means tests were EXECUTED and all passed
- "Tests pass - N/A" is only valid for documentation-only tickets
- Test file existence alone does NOT satisfy this requirement

## Agents
- rust-indexer-engineer
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary
Add `copy_existing_embeddings()` method to the embedding pipeline that copies embeddings from `code_embeddings` table to chunks with NULL embeddings before attempting API generation. This is the critical missing step from BLOBSHA deduplication.

## Background
The BLOBSHA project created the `code_embeddings` deduplication table and `blob_sha` column, but the embedding pipeline never uses this cache. It only checks for NULL embeddings and generates new ones. This causes variant worktree scans to take hours (42K API calls) when they should take seconds (SQL copy operation).

Database evidence shows 670K chunks but only 58K unique blob SHAs - 88% duplication. With 42K chunks having NULL embeddings, we're regenerating embeddings that already exist in the cache.

This ticket implements Phase 1 of the Embedding Inheritance Fix project, specifically the "Embedding Copy Step" detailed in the architecture document.

**Planning References:**
- Architecture: `.agents/projects/EMBCOPY_embedding-inheritance-fix/planning/architecture.md` (lines 15-85)
- Plan: `.agents/projects/EMBCOPY_embedding-inheritance-fix/planning/plan.md` (lines 11-49)

## Acceptance Criteria
- [x] `copy_existing_embeddings()` method added to `EmbeddingPipeline` struct
- [x] Method executes SQL UPDATE query with JOIN to copy embeddings by blob_sha
- [x] Method returns count of chunks updated
- [x] `PipelineStats` struct extended with `copied_from_cache` and `cost_saved_usd` fields
- [x] `run()` method calls copy step before generation step
- [x] Code compiles without errors
- [x] Stats summary includes copy metrics (copied count and cost saved)

## Technical Requirements

### 1. New Method Signature
```rust
async fn copy_existing_embeddings(&self, client: &Client) -> Result<usize>
```

### 2. SQL UPDATE Query
```sql
UPDATE maproom.chunks c
SET
    code_embedding = ce.code_embedding,
    text_embedding = ce.text_embedding,
    updated_at = NOW()
FROM maproom.code_embeddings ce
WHERE c.blob_sha = ce.blob_sha
  AND (c.code_embedding IS NULL OR c.text_embedding IS NULL)
RETURNING c.id
```

### 3. Stats Struct Extension
```rust
pub struct PipelineStats {
    pub total_chunks: usize,
    pub copied_from_cache: usize,  // NEW
    pub generated_new: usize,
    pub cost_saved_usd: f64,       // NEW: copied * $0.00013
    // ... existing fields
}
```

### 4. Integration into run() Method
- Call `copy_existing_embeddings()` after finding NULL chunks
- Pass remaining NULL chunks to generation step
- Update stats with both copy and generation counts
- Display copy metrics in stats summary

### 5. Error Handling
- Use tokio_postgres::Client for query execution
- Handle edge cases: partial embeddings, no cache entry, concurrent updates
- Query must be idempotent - safe to run multiple times
- If copy fails, fall back to generation (fail-safe design)
- Use RETURNING clause to count updated rows efficiently

## Implementation Notes

**Pipeline Flow:**
1. Find chunks with NULL embeddings
2. **NEW: Copy embeddings from cache by blob_sha**
3. Re-query to find remaining NULL chunks
4. Generate new embeddings for remaining chunks
5. Report stats (copy + generation)

**Cost Calculation:**
- OpenAI text-embedding-3-small: $0.00013 per 1K tokens
- Average chunk ~1K tokens
- `cost_saved_usd = copied_from_cache * 0.00013`

**Idempotency:**
- Query only updates chunks with NULL embeddings
- WHERE clause: `(c.code_embedding IS NULL OR c.text_embedding IS NULL)`
- Safe to run multiple times without duplicating work

**Performance:**
- Single UPDATE with JOIN is fast (milliseconds vs hours)
- WHERE clause limits scope to NULL chunks only
- RETURNING clause avoids separate COUNT query

## Dependencies
None - this is the first implementation ticket for Phase 1.

## Risk Assessment

**Risk**: SQL query syntax errors in UPDATE with JOIN
- **Mitigation**: Carefully review PostgreSQL JOIN syntax for UPDATE statements; test query manually in psql

**Risk**: Stats tracking incorrect (double-counting or missing counts)
- **Mitigation**: Add logging at each step; verify stats sum correctly (copied + generated = total NULL chunks)

**Risk**: Performance regression if query scans entire table
- **Mitigation**: WHERE clause limits to NULL chunks only; ensure blob_sha has index (already exists)

**Risk**: Concurrent updates causing race conditions
- **Mitigation**: Query is idempotent; PostgreSQL transaction isolation handles concurrent updates safely

## Files/Packages Affected
- `crates/maproom/src/embedding/pipeline.rs` (modify)
  - Add `copy_existing_embeddings()` method
  - Extend `PipelineStats` struct
  - Update `run()` method to call copy step
  - Update stats display to show copy metrics
