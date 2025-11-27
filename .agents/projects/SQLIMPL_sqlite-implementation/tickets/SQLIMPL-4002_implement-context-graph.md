# Ticket: SQLIMPL-4002: Implement Context Graph

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - tests executed and passing (or N/A if no tests)
- [ ] **Verified** - by the verify-ticket agent

**Phase 4 - OPTIONAL ENHANCEMENT:** This ticket is part of the optional context assembly phase. Defer if timeline pressure.

## Agents
- rust-indexer-engineer
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary
Implement the context graph for building relationship maps used in context expansion. This DELEGATES to existing SqliteStore graph methods.

## Background
The context graph at `src/context/graph.rs` has 3 stubbed methods that should delegate to existing `find_callers()`, `find_callees()`, and `find_imports()` methods in SqliteStore.

**Key Insight:** This is delegation, not reimplementation. The graph traversal SQL already exists.

This ticket implements Plan Phase 4, Ticket 4002: "Implement Context Graph".

## Acceptance Criteria
- [ ] Graph methods delegate to existing SqliteStore methods
- [ ] `get_callers()` returns chunks that call the target
- [ ] `get_callees()` returns chunks called by the target
- [ ] `get_imports()` returns import relationships
- [ ] Relationship maps built correctly for context expansion
- [ ] Context graph tests pass

## Technical Requirements
- **DELEGATE, don't reimplement:** Use existing `SqliteStore::find_callers()`, `find_callees()`, `find_imports()`
- Build relationship maps for context assembly
- Handle circular references gracefully

## Implementation Notes

### Current Stubs (to implement)
```rust
// src/context/graph.rs:95
// get_callers() - stub

// src/context/graph.rs:121
// get_callees() - stub

// src/context/graph.rs:150
// get_related() - stub (may use imports/extensions)
```

### Existing SqliteStore Methods to Use
```rust
// src/db/sqlite/mod.rs
pub async fn find_callers(&self, chunk_id: i64, max_depth: Option<i32>) -> Result<Vec<GraphResult>>
pub async fn find_callees(&self, chunk_id: i64, max_depth: Option<i32>) -> Result<Vec<GraphResult>>
pub async fn find_imports(&self, chunk_id: i64, max_depth: Option<i32>) -> Result<Vec<GraphResult>>
pub async fn find_extensions(&self, chunk_id: i64, max_depth: Option<i32>) -> Result<Vec<GraphResult>>
```

### Target Implementation Pattern
```rust
pub async fn get_callers(&self, chunk_id: i64, depth: i32) -> Result<Vec<RelatedChunk>> {
    // Delegate to existing SqliteStore method
    let results = self.store.find_callers(chunk_id, Some(depth)).await?;

    // Convert GraphResult to RelatedChunk
    let related: Vec<RelatedChunk> = results.into_iter().map(|r| {
        RelatedChunk {
            chunk_id: r.chunk_id,
            relationship: Relationship::Caller,
            depth: r.depth,
        }
    }).collect();

    Ok(related)
}

pub async fn get_callees(&self, chunk_id: i64, depth: i32) -> Result<Vec<RelatedChunk>> {
    let results = self.store.find_callees(chunk_id, Some(depth)).await?;
    // Similar conversion...
}

pub async fn get_related(&self, chunk_id: i64, depth: i32) -> Result<RelationshipMap> {
    // Combine callers, callees, imports, extensions
    let callers = self.get_callers(chunk_id, depth).await?;
    let callees = self.get_callees(chunk_id, depth).await?;
    let imports = self.store.find_imports(chunk_id, Some(depth)).await?;

    Ok(RelationshipMap {
        callers,
        callees,
        imports: imports.into_iter().map(/* convert */).collect(),
    })
}
```

## Dependencies
- Phase 1 Complete (tests compile)

## Risk Assessment
- **Risk**: Deep graphs may be slow
  - **Mitigation**: Limit depth; existing methods already have max_depth parameter
- **Risk**: Circular references
  - **Mitigation**: Track visited chunks; limit total results

## Files/Packages Affected
- `crates/maproom/src/context/graph.rs` (primary)
