# Ticket: SQLIMPL-3003: Implement Edge Updater

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
Implement the edge updater for computing and storing chunk relationships. This maintains the code graph (call/import/extends relationships) as files change.

## Background
The edge updater at `src/incremental/edge_updater.rs` has 4 stubbed methods for relationship management. These use tree-sitter to analyze code and populate the `chunk_edges` table.

This ticket implements Plan Phase 3, Ticket 3003: "Implement Edge Updater".

## Acceptance Criteria
- [ ] `compute_edges()` extracts call/import relationships using tree-sitter
- [ ] `insert_edges()` stores edges in `chunk_edges` table
- [ ] `update_edges()` recomputes edges for changed files
- [ ] `find_test_targets()` identifies test → implementation relationships
- [ ] Edge computation and storage work correctly
- [ ] Graph traversal (from Phase 2) returns results after edges are computed

## Technical Requirements
- Use existing tree-sitter queries for relationship extraction
- Edge types: `calls`, `imports`, `extends`, `tests`
- Store in `chunk_edges` table: `(src_chunk_id, dst_chunk_id, edge_type)`
- Handle self-references and circular dependencies gracefully

## Implementation Notes

### Current Stubs (to implement)
```rust
// src/incremental/edge_updater.rs:114
// compute_edges() - stub

// src/incremental/edge_updater.rs:184
// insert_edges() - stub

// src/incremental/edge_updater.rs:247
// update_edges() - stub

// src/incremental/edge_updater.rs:261
// find_test_targets() - stub
```

### Schema Reference
```sql
CREATE TABLE chunk_edges (
    id INTEGER PRIMARY KEY,
    src_chunk_id INTEGER REFERENCES chunks(id),
    dst_chunk_id INTEGER REFERENCES chunks(id),
    edge_type TEXT  -- 'calls', 'imports', 'extends', 'tests'
);
```

### Target Implementation Patterns

#### Compute Edges
```rust
pub fn compute_edges(&self, chunks: &[Chunk]) -> Vec<Edge> {
    let mut edges = Vec::new();

    for chunk in chunks {
        // Use tree-sitter to find relationships
        if let Some(tree) = &chunk.tree {
            // Extract function calls
            let calls = self.extract_calls(tree, &chunk.content);
            for target in calls {
                if let Some(dst_chunk) = self.resolve_target(&target, chunks) {
                    edges.push(Edge {
                        src_chunk_id: chunk.id,
                        dst_chunk_id: dst_chunk.id,
                        edge_type: EdgeType::Calls,
                    });
                }
            }

            // Extract imports
            let imports = self.extract_imports(tree, &chunk.content);
            // ... similar pattern
        }
    }

    edges
}
```

#### Insert Edges
```rust
pub async fn insert_edges(&self, edges: &[Edge]) -> Result<()> {
    self.store.run(move |conn| {
        let mut stmt = conn.prepare(
            "INSERT INTO chunk_edges (src_chunk_id, dst_chunk_id, edge_type) VALUES (?, ?, ?)"
        )?;

        for edge in edges {
            stmt.execute(params![
                edge.src_chunk_id,
                edge.dst_chunk_id,
                edge.edge_type.as_str()
            ])?;
        }

        Ok(())
    }).await
}
```

#### Update Edges
```rust
pub async fn update_edges(&self, chunk_ids: &[i64]) -> Result<()> {
    // 1. Delete existing edges for these chunks
    self.store.run(move |conn| {
        conn.execute(
            "DELETE FROM chunk_edges WHERE src_chunk_id IN (?)",
            // ... parameter handling for IN clause
        )?;
        Ok(())
    }).await?;

    // 2. Recompute edges for affected chunks
    let chunks = self.store.get_chunks_by_ids(chunk_ids).await?;
    let new_edges = self.compute_edges(&chunks);

    // 3. Insert new edges
    self.insert_edges(&new_edges).await
}
```

#### Find Test Targets
```rust
pub fn find_test_targets(&self, test_chunk: &Chunk) -> Vec<i64> {
    // Analyze test file to find what it's testing
    // Patterns: import statements, function names matching test_*
    // Returns chunk IDs of likely implementation targets
}
```

## Dependencies
- SQLIMPL-3001 (Change Detector)
- Phase 1 Complete (tests compile)

## Risk Assessment
- **Risk**: Edge resolution may fail for external dependencies
  - **Mitigation**: Only create edges to known chunks; ignore unresolved
- **Risk**: Tree-sitter queries may not cover all languages
  - **Mitigation**: Start with Rust/TypeScript/Python; document limitations

## Files/Packages Affected
- `crates/maproom/src/incremental/edge_updater.rs` (primary)
