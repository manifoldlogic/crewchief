# Ticket: SQLITE-3901: Vector Search Tests

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - tests executed and passing
- [x] **Verified** - by the verify-ticket agent

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
- [x] Test basic similarity search returns results
- [x] Test results are ordered by similarity (best first)
- [x] Test empty index returns empty results (not error)
- [x] Test worktree filtering works correctly
- [x] Test extension missing returns empty results gracefully
- [x] Test dimension validation catches mismatches
- [x] All tests pass: `cargo test --features sqlite vector`

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

---

## Implementation Notes (rust-indexer-engineer)

### Summary
Comprehensive vector search tests implemented covering all acceptance criteria. Tests validate correct behavior for normal operation, edge cases, and graceful degradation.

### Tests Implemented

**In `mod.rs` (integration tests):**

1. **`test_vector_search_integration`** (existing from SQLITE-3001)
   - Basic similarity search returns results
   - Results sorted by distance (ascending)
   - Worktree filtering works (both main and feature worktrees)
   - Non-existent repo returns empty results
   - Non-existent worktree returns empty results

2. **`test_vector_search_no_embeddings`** (existing from SQLITE-3001)
   - Empty index returns empty results (not error)

3. **`test_vector_search_dimension_validation`** (existing from SQLITE-3001)
   - Dimension mismatch (3 dims instead of 1536) returns clear error

4. **`test_vector_search_extension_not_available`** (NEW)
   - Simulates missing sqlite-vec extension by setting `vec_available` to false
   - Verifies `search_vector` returns `Ok(vec![])` not an error
   - Verifies `has_vec_extension()` returns false

5. **`test_vector_search_similarity_ordering`** (NEW)
   - Creates 3 chunks with embeddings at different distances from query
   - Verifies results come back in distance order (ascending)
   - First result has high similarity (> 0.9)
   - Last result has lower similarity than first

**In `vector.rs` (unit tests):**

5 existing unit tests for `distance_to_similarity` function:
- `test_distance_to_similarity_identical` - Distance 0 = similarity 1.0
- `test_distance_to_similarity_different` - Distance 1.0 = similarity 0.5
- `test_distance_to_similarity_far` - Large distance = low similarity
- `test_distance_to_similarity_monotonic` - Similarity decreases as distance increases
- `test_distance_to_similarity_range` - All similarities in (0, 1] range

### Test Results
```
running 28 tests
test db::sqlite::tests::test_vector_search_dimension_validation ... ok
test db::sqlite::tests::test_vector_search_extension_not_available ... ok
test db::sqlite::tests::test_vector_search_no_embeddings ... ok
test db::sqlite::tests::test_vector_search_similarity_ordering ... ok
test db::sqlite::tests::test_vector_search_integration ... ok
... (28 total SQLite tests passing)
```

### Acceptance Criteria Verification

| Criterion | Test |
|-----------|------|
| Basic similarity search returns results | `test_vector_search_integration` |
| Results ordered by similarity (best first) | `test_vector_search_similarity_ordering` |
| Empty index returns empty results | `test_vector_search_no_embeddings` |
| Worktree filtering works | `test_vector_search_integration` |
| Extension missing returns empty results | `test_vector_search_extension_not_available` |
| Dimension validation catches mismatches | `test_vector_search_dimension_validation` |
