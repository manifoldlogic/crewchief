# Ticket: SQLIMPL-2001: Wire FTS Executor to SqliteStore

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - all FTS and search tests passing
- [x] **Verified** - by the verify-ticket agent

## Agents
- rust-indexer-engineer
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary
Wire the FTS (Full-Text Search) executor to the existing `SqliteStore::search_chunks_fts()` method. This is a DELEGATION task - do NOT write new SQL; call the existing method and convert return types.

## Background
The FTS executor at `src/search/fts.rs:159` currently returns `RankedResults::empty()` with a TODO comment. However, `SqliteStore` already has a fully working FTS5 implementation at `src/db/sqlite/mod.rs:611-748`.

**Key Insight:** This is wiring, not reimplementation. The SQL exists and works.

This ticket implements Plan Phase 2, Ticket 2001: "Wire FTS Executor to SqliteStore".

## Acceptance Criteria
- [x] `FtsExecutor::execute()` calls `SqliteStore::search_fts_by_id()` (new method added)
- [x] `SearchHit` results are converted to `RankedResult` format
- [x] TODO comment and placeholder return removed from `fts.rs:159`
- [x] FTS search returns non-empty results for queries matching indexed content
- [x] Search tests (from Phase 1) that use FTS now pass (40 FTS tests + 1 FTS search test)

## Technical Requirements
- **DELEGATE, don't reimplement:** Call `store.search_chunks_fts()`, not raw SQL
- Convert `SearchHit` → `RankedResult`:
  ```rust
  RankedResult {
      chunk_id: hit.chunk_id,
      score: hit.score,  // Already normalized by SqliteStore
      source: SearchSource::FTS,
      // other fields as needed
  }
  ```
- Use existing `normalize_fts_rank()` if additional normalization needed (it's in `src/db/sqlite/fts.rs:28-30`)

## Implementation Notes

### Current Code (to replace)
```rust
// src/search/fts.rs:159
// TODO(IDXABS-2003): This is a placeholder implementation.
Ok(RankedResults::empty())
```

### Target Implementation Pattern
```rust
pub async fn execute(&self, query: &SearchQuery) -> Result<RankedResults> {
    // Delegate to existing SqliteStore method
    let hits = self.store.search_chunks_fts(
        &query.repo,
        query.worktree.as_deref(),
        &query.text,
        query.limit as i64,
        false,  // debug
    ).await?;

    // Convert SearchHit to RankedResult
    let results: Vec<RankedResult> = hits.into_iter().map(|hit| {
        RankedResult {
            chunk_id: hit.chunk_id,
            score: hit.score,
            source: SearchSource::FTS,
            // ... other fields
        }
    }).collect();

    Ok(RankedResults::new(results, SearchSource::FTS))
}
```

### Verification
After implementation:
```bash
# Should return non-empty results
cargo run -p crewchief-maproom -- search "function"
```

## Dependencies
- Phase 1 Complete (tests compile)

## Risk Assessment
- **Risk**: SearchHit → RankedResult conversion may miss fields
  - **Mitigation**: Review both structs carefully; map all relevant fields
- **Risk**: Score normalization may differ from expectations
  - **Mitigation**: SqliteStore already normalizes; verify with tests

## Files/Packages Affected
- `crates/maproom/src/search/fts.rs` (primary)
