# Ticket: SRCHDUP-2004: Integration tests for pipeline dedup

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - tests executed and passing (14 tests pass)
- [ ] **Verified** - by the verify-ticket agent

## Agents
- integration-tester
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary

Create integration tests that verify deduplication works correctly within the full search pipeline. Tests should use a test database with duplicate chunks across worktrees and verify the deduplication behavior end-to-end.

## Background

Unit tests verify the dedup module in isolation. Integration tests verify the module works correctly when integrated into the pipeline with real database queries, fusion, and result assembly.

**Reference:** plan.md Phase 2, quality-strategy.md "Level 2: Integration Tests"

## Acceptance Criteria

- [ ] New test file `crates/maproom/tests/search_dedup_integration.rs` exists
- [ ] Test fixture creates duplicate chunks across multiple worktrees
- [ ] Test verifies deduplication reduces duplicate count correctly
- [ ] Test verifies highest-scoring duplicate is selected
- [ ] Test verifies `deduplicate=false` returns all results
- [ ] Test verifies default behavior enables deduplication
- [ ] All tests pass: `cargo test --test search_dedup_integration`

## Technical Requirements

### Test Fixture Setup
```rust
async fn setup_duplicate_chunks(db: &Pool) -> Result<TestData> {
    // Create test repo
    let repo_id = insert_repo(db, "test-repo").await?;

    // Create two worktrees
    let wt_main = insert_worktree(db, repo_id, "main").await?;
    let wt_feature = insert_worktree(db, repo_id, "feature-x").await?;

    // Insert the same file in both worktrees
    let file_main = insert_file(db, wt_main, "src/auth.rs").await?;
    let file_feature = insert_file(db, wt_feature, "src/auth.rs").await?;

    // Insert identical chunks
    let chunk_content = "fn validate(token: &str) -> bool { ... }";
    insert_chunk(db, file_main, "validate", 10, 25, chunk_content).await?;
    insert_chunk(db, file_feature, "validate", 10, 25, chunk_content).await?;

    Ok(TestData { repo_id, ... })
}
```

### Required Tests

1. **test_search_with_deduplication_enabled**
```rust
#[tokio::test]
async fn test_search_with_deduplication_enabled() {
    let db = setup_test_db().await;
    let data = setup_duplicate_chunks(&db).await.unwrap();

    let options = SearchOptions::new(data.repo_id, None, 10);  // dedup=true by default
    let results = pipeline.search("validate", &options).await.unwrap();

    // Should return only one result (duplicates merged)
    assert_eq!(results.len(), 1);
}
```

2. **test_search_with_deduplication_disabled**
```rust
#[tokio::test]
async fn test_search_with_deduplication_disabled() {
    let db = setup_test_db().await;
    let data = setup_duplicate_chunks(&db).await.unwrap();

    let options = SearchOptions::new(data.repo_id, None, 10).without_dedup();
    let results = pipeline.search("validate", &options).await.unwrap();

    // Should return both results
    assert!(results.len() >= 2);
}
```

3. **test_search_selects_highest_score**
```rust
#[tokio::test]
async fn test_search_selects_highest_score() {
    // Insert duplicates with known different scores
    // Verify the higher-scoring one is returned
}
```

4. **test_search_mode_compatibility**
```rust
#[tokio::test]
async fn test_search_mode_compatibility() {
    // Verify dedup works with FTS, vector, and hybrid modes
}
```

## Implementation Notes

1. **Use existing test infrastructure** - Check how other integration tests set up the database
2. **Clean up after tests** - Ensure test data doesn't persist
3. **Parallel test safety** - Use unique identifiers if tests run in parallel
4. **Check embedding requirements** - Vector search may need embeddings populated

### Test Organization
```
crates/maproom/tests/
├── search_dedup_integration.rs  # NEW
└── ... existing test files ...
```

## Dependencies

- SRCHDUP-2002 (pipeline integration complete)

## Risk Assessment

- **Risk**: Test database setup complexity
  - **Mitigation**: Reuse existing test fixtures if available
- **Risk**: Tests may be slow due to database operations
  - **Mitigation**: Minimize fixture data, use transactions for rollback
- **Risk**: Embedding generation required for vector search tests
  - **Mitigation**: May skip vector-specific tests or use pre-computed embeddings

## Files/Packages Affected

- `crates/maproom/tests/search_dedup_integration.rs` (NEW)
- Possibly `crates/maproom/tests/common/mod.rs` or similar for shared fixtures
