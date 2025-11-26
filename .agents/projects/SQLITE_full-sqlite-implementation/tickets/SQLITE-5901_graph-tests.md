# Ticket: SQLITE-5901: Graph Tests

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - tests executed and passing
- [ ] **Verified** - by the verify-ticket agent

## Agents
- rust-indexer-engineer
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary
Create comprehensive tests for graph traversal functionality including cycle handling, depth limiting, and edge cases.

## Background
Graph traversal is complex and must handle cycles correctly, respect depth limits, and return accurate paths. Tests ensure the recursive CTEs work correctly.

Implements: Plan Phase 5 - Graph Traversal

## Acceptance Criteria
- [ ] Test direct caller/callee relationships (depth 1)
- [ ] Test transitive relationships (depth > 1)
- [ ] Test cycle handling doesn't infinite loop
- [ ] Test depth limiting works correctly
- [ ] Test empty graph returns empty results
- [ ] Test with 100+ node graph (performance sanity)
- [ ] Test import relationships (incoming/outgoing)
- [ ] All tests pass: `cargo test --features sqlite graph`

## Technical Requirements
Add tests to `crates/maproom/src/db/sqlite/graph.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    /// Helper to create test graph: A -> B -> C
    async fn setup_linear_graph(store: &SqliteStore) -> (i64, i64, i64) {
        // Create chunks A, B, C
        let a = store.insert_chunk(...).await.unwrap();
        let b = store.insert_chunk(...).await.unwrap();
        let c = store.insert_chunk(...).await.unwrap();

        // A calls B, B calls C
        store.insert_edge(a, b, "calls").await.unwrap();
        store.insert_edge(b, c, "calls").await.unwrap();

        (a, b, c)
    }

    /// Direct relationships work
    #[tokio::test]
    async fn test_find_callers_direct() {
        let store = setup_test_store().await;
        let (a, b, c) = setup_linear_graph(&store).await;

        let callers = store.find_callers(b, Some(1)).await.unwrap();

        assert_eq!(callers.len(), 1);
        assert_eq!(callers[0].chunk_id, a);
        assert_eq!(callers[0].depth, 1);
    }

    /// Transitive relationships work
    #[tokio::test]
    async fn test_find_callers_transitive() {
        let store = setup_test_store().await;
        let (a, b, c) = setup_linear_graph(&store).await;

        let callers = store.find_callers(c, Some(3)).await.unwrap();

        // Should find both B (depth 1) and A (depth 2)
        assert_eq!(callers.len(), 2);
        assert!(callers.iter().any(|r| r.chunk_id == b && r.depth == 1));
        assert!(callers.iter().any(|r| r.chunk_id == a && r.depth == 2));
    }

    /// Cycles don't cause infinite loop
    #[tokio::test]
    async fn test_graph_cycle_handling() {
        let store = setup_test_store().await;

        // Create cycle: A -> B -> C -> A
        let a = store.insert_chunk(...).await.unwrap();
        let b = store.insert_chunk(...).await.unwrap();
        let c = store.insert_chunk(...).await.unwrap();
        store.insert_edge(a, b, "calls").await.unwrap();
        store.insert_edge(b, c, "calls").await.unwrap();
        store.insert_edge(c, a, "calls").await.unwrap();  // Cycle!

        // Should not hang, should not include duplicates
        let callers = store.find_callers(a, Some(10)).await.unwrap();

        // Each chunk should appear at most once
        let unique_chunks: HashSet<i64> = callers.iter().map(|r| r.chunk_id).collect();
        assert_eq!(unique_chunks.len(), callers.len());
    }

    /// Depth limiting works
    #[tokio::test]
    async fn test_depth_limiting() {
        let store = setup_test_store().await;
        let (a, b, c) = setup_linear_graph(&store).await;

        // With depth 1, should only find direct caller
        let callers = store.find_callers(c, Some(1)).await.unwrap();
        assert_eq!(callers.len(), 1);
        assert_eq!(callers[0].chunk_id, b);

        // Should not find A (depth 2)
        assert!(!callers.iter().any(|r| r.chunk_id == a));
    }

    /// Empty graph returns empty results
    #[tokio::test]
    async fn test_empty_graph() {
        let store = setup_test_store().await;
        let chunk = store.insert_chunk(...).await.unwrap();

        let callers = store.find_callers(chunk, None).await.unwrap();
        assert!(callers.is_empty());
    }

    /// Performance sanity check with 100+ nodes
    #[tokio::test]
    async fn test_graph_traversal_100_nodes() {
        let store = setup_test_store().await;

        // Create chain of 100 nodes
        let mut chunks = Vec::new();
        for i in 0..100 {
            chunks.push(store.insert_chunk(...).await.unwrap());
        }
        for i in 0..99 {
            store.insert_edge(chunks[i], chunks[i+1], "calls").await.unwrap();
        }

        let start = std::time::Instant::now();
        let callers = store.find_callers(chunks[99], Some(10)).await.unwrap();
        let elapsed = start.elapsed();

        // Should complete in reasonable time (<1s)
        assert!(elapsed.as_secs() < 1, "Graph traversal took {:?}", elapsed);

        // Should find 10 callers (limited by depth)
        assert_eq!(callers.len(), 10);
    }
}
```

## Implementation Notes
- Use `:memory:` database for tests
- Create helper functions for setting up test graphs
- Cycle test is critical - must not hang
- Performance test with 100 nodes ensures queries don't explode

## Dependencies
- SQLITE-5001 (Graph Module) - functionality to test

## Risk Assessment
- **Risk**: Cycle test could hang if cycle detection broken
  - **Mitigation**: Add test timeout; cycle detection in CTE verified
- **Risk**: 100-node test slow in CI
  - **Mitigation**: Test with timeout; can be marked #[ignore] if problematic

## Files/Packages Affected
- `crates/maproom/src/db/sqlite/graph.rs` (add test module)
