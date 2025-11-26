# Ticket: SQLITE-3901: Vector Search Tests

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
Create comprehensive tests for vector search functionality including edge cases and graceful degradation.

## Background
Vector search is a critical path that must work correctly. Tests should cover normal operation, edge cases (empty index, missing extension), and verify similarity ordering is correct.

Implements: Plan Phase 3 - Vector Search

## Acceptance Criteria
- [ ] Test basic similarity search returns results
- [ ] Test results are ordered by similarity (best first)
- [ ] Test empty index returns empty results (not error)
- [ ] Test worktree filtering works correctly
- [ ] Test extension missing returns empty results gracefully
- [ ] Test dimension validation catches mismatches
- [ ] All tests pass: `cargo test --features sqlite vector`

## Technical Requirements
Add tests to `crates/maproom/src/db/sqlite/vector.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    /// Basic similarity search returns results
    #[tokio::test]
    async fn test_vector_search_basic() {
        let store = setup_test_store().await;

        // Insert test embeddings
        let embedding1 = vec![0.1f32; 1536];
        let embedding2 = vec![0.2f32; 1536];
        store.upsert_embedding("sha1", &embedding1, "test").await.unwrap();
        store.upsert_embedding("sha2", &embedding2, "test").await.unwrap();

        // Insert chunks referencing those embeddings
        // ... setup chunks with blob_sha = "sha1", "sha2" ...

        // Query with embedding similar to embedding1
        let query = vec![0.1f32; 1536];
        let results = store.search_vector("repo", None, &query, 10).await.unwrap();

        assert!(!results.is_empty());
        // First result should be most similar (embedding1)
    }

    /// Results are ordered by similarity
    #[tokio::test]
    async fn test_vector_search_similarity_ordering() {
        // Insert 3 embeddings at different distances from query
        // Verify results come back in distance order
    }

    /// Empty index returns empty results
    #[tokio::test]
    async fn test_vector_search_empty_index() {
        let store = setup_test_store().await;

        let query = vec![0.1f32; 1536];
        let results = store.search_vector("repo", None, &query, 10).await.unwrap();

        assert!(results.is_empty());  // Empty, not error
    }

    /// Worktree filtering works
    #[tokio::test]
    async fn test_vector_search_worktree_filter() {
        // Insert chunks in different worktrees
        // Verify filtering by worktree returns only matching chunks
    }

    /// Extension missing returns empty results gracefully
    #[tokio::test]
    async fn test_extension_missing_graceful_degradation() {
        // This test requires mocking extension availability
        // Or testing on a build without sqlite-vec
        // Verify search returns Ok(vec![]) not an error
    }
}
```

## Implementation Notes
- Use `:memory:` database for unit tests (fast, isolated)
- Create a `setup_test_store()` helper that creates store with test data
- For extension missing test, may need to create a mock or conditional compilation
- Test floating point comparisons with appropriate tolerance

## Dependencies
- SQLITE-3001 (Vector Search Module) - functionality to test

## Risk Assessment
- **Risk**: Tests flaky due to floating point comparisons
  - **Mitigation**: Use appropriate tolerance for float comparison
- **Risk**: Extension missing test hard to write
  - **Mitigation**: Test via mock or skip with #[ignore] if extension always present

## Files/Packages Affected
- `crates/maproom/src/db/sqlite/vector.rs` (add test module)
