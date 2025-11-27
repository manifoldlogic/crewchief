# Ticket: SQLIMPL-2003: Wire Graph Executor to SqliteStore

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - tests executed and passing (or N/A if no tests)
- [ ] **Verified** - by the verify-ticket agent

## Agents
- rust-indexer-engineer
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary
Wire the Graph executor to the existing `SqliteStore::find_callers()` and `SqliteStore::find_callees()` methods. This is a DELEGATION task - do NOT write new SQL; call existing methods and convert return types.

## Background
The Graph executor at `src/search/graph.rs:76,105` has two placeholder implementations. However, `SqliteStore` already has working recursive CTE implementations at `src/db/sqlite/mod.rs:1783-1817`.

**Key Insight:** This is wiring, not reimplementation. The graph traversal SQL exists and works.

This ticket implements Plan Phase 2, Ticket 2003: "Wire Graph Executor to SqliteStore".

## Acceptance Criteria
- [ ] `GraphExecutor` calls `SqliteStore::find_callers()` for upstream relationships
- [ ] `GraphExecutor` calls `SqliteStore::find_callees()` for downstream relationships
- [ ] Graph results converted to `RankedResult` with depth-based scoring
- [ ] TODO comments and placeholders removed from `graph.rs:76,105`
- [ ] Graph executor returns related chunks when chunk_edges exist
- [ ] Graph tests (from Phase 1) now pass

## Technical Requirements
- **DELEGATE, don't reimplement:** Call `store.find_callers()` and `store.find_callees()`
- Score based on graph depth: `score = 1.0 / (depth as f64 + 1.0)`
- Convert `GraphResult` → `RankedResult`
- May also leverage `find_imports()` and `find_extensions()` if needed

## Implementation Notes

### Current Code (to replace)
```rust
// src/search/graph.rs:76
// TODO(IDXABS-2003): This is a placeholder implementation.
// (find_callers stub)

// src/search/graph.rs:105
// TODO(IDXABS-2003): This is a placeholder implementation.
// (find_callees stub)
```

### Target Implementation Pattern
```rust
pub async fn execute(&self, seed_chunks: &[i64]) -> Result<RankedResults> {
    let mut results = Vec::new();

    for chunk_id in seed_chunks {
        // Find callers (who calls this chunk)
        let callers = self.store.find_callers(*chunk_id, Some(2)).await?;

        // Find callees (what this chunk calls)
        let callees = self.store.find_callees(*chunk_id, Some(2)).await?;

        // Convert to RankedResults with depth-based scoring
        for result in callers.into_iter().chain(callees) {
            let score = 1.0 / (result.depth as f64 + 1.0);
            results.push(RankedResult {
                chunk_id: result.chunk_id,
                score,
                source: SearchSource::Graph,
                // ... other fields
            });
        }
    }

    Ok(RankedResults::new(results, SearchSource::Graph))
}
```

### Existing SqliteStore Methods
```rust
// src/db/sqlite/mod.rs:1783-1817
pub async fn find_callers(&self, chunk_id: i64, max_depth: Option<i32>) -> Result<Vec<GraphResult>>
pub async fn find_callees(&self, chunk_id: i64, max_depth: Option<i32>) -> Result<Vec<GraphResult>>
```

### Score Calculation
- Depth 0 (direct): 1.0 / 1.0 = 1.0
- Depth 1: 1.0 / 2.0 = 0.5
- Depth 2: 1.0 / 3.0 = 0.33

## Dependencies
- Phase 1 Complete (tests compile)
- chunk_edges table must be populated for results

## Risk Assessment
- **Risk**: Empty results if no edges in database
  - **Mitigation**: Expected behavior; edges are populated during scan
- **Risk**: Recursive CTE may be slow for deep graphs
  - **Mitigation**: Max depth parameter limits traversal

## Files/Packages Affected
- `crates/maproom/src/search/graph.rs` (primary)
