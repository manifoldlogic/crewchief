# Ticket: BRANCHX-1010: Test incremental update logic and correctness

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - related tests pass
- [ ] **Verified** - by the verify-ticket agent

## Agents
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary
Create comprehensive test suite verifying that incremental updates produce identical results to full scans, and test all edge cases including no changes, deletions, and multi-worktree scenarios.

## Background
This is Phase 3, Step 3.4 of BRANCHX—the most critical testing phase. The incremental update algorithm must produce **exactly the same results** as a full scan, or the entire project fails. We need to verify the core optimization (tree SHA skip) works correctly and test edge cases thoroughly.

This ticket implements Phase 3.4 "Incremental Update Tests" from `.agents/projects/BRANCHX_branch-aware-indexing/planning/plan.md`.

## Acceptance Criteria
- [ ] **CRITICAL**: `test_incremental_equals_full_scan` passes - verifies incremental produces identical results to full scan
- [ ] **CRITICAL**: `test_tree_sha_skip_unchanged` passes - verifies tree SHA optimization works and completes <100ms
- [ ] `test_incremental_only_scans_changed_files` passes - verifies efficiency of selective scanning
- [ ] `test_deleted_file_removes_worktree` passes - verifies deletion handling updates worktree_ids correctly
- [ ] `test_same_content_multiple_worktrees` passes - verifies multi-branch tracking shares chunks
- [ ] All tests pass in CI environment
- [ ] Performance benchmarks documented showing tree SHA check <100ms

## Technical Requirements
- Create test git repositories with controlled changes and commits
- Compare incremental vs full scan: verify identical blob_sha sets
- Test tree SHA comparison on unchanged repo (should skip all processing)
- Test file deletion properly updates worktree_ids in database
- Test same content in multiple worktrees correctly shares chunks
- Use real database connection (not mocked)
- Measure and assert tree SHA check time <100ms
- Use deterministic git commits with controlled timestamps
- Test with realistic file variety (not toy examples)

## Implementation Notes

Create test file: `crates/maproom/tests/incremental_update.rs`

### CRITICAL TEST 1: Correctness Guarantee
```rust
#[tokio::test]
async fn test_incremental_equals_full_scan() {
    let pool = get_test_pool().await;
    let repo = create_test_repo();

    // Full scan (baseline)
    let worktree1 = create_worktree(&pool, "full").await.unwrap();
    full_scan(&pool, worktree1, &repo).await.unwrap();
    let full_chunks = get_all_chunks(&pool, worktree1).await.unwrap();

    // Incremental scan (same content)
    let worktree2 = create_worktree(&pool, "incremental").await.unwrap();
    incremental_update(&pool, worktree2, &repo).await.unwrap();
    let incr_chunks = get_all_chunks(&pool, worktree2).await.unwrap();

    // Should index identical chunks
    assert_eq!(full_chunks.len(), incr_chunks.len());
    assert_eq!(
        full_chunks.into_iter().map(|c| c.blob_sha).collect::<HashSet<_>>(),
        incr_chunks.into_iter().map(|c| c.blob_sha).collect::<HashSet<_>>()
    );
}
```

### CRITICAL TEST 2: Optimization Works
```rust
#[tokio::test]
async fn test_tree_sha_skip_unchanged() {
    let pool = get_test_pool().await;
    let repo = create_test_repo();
    let worktree_id = create_worktree(&pool, "main").await.unwrap();

    // Initial scan
    let stats1 = incremental_update(&pool, worktree_id, &repo).await.unwrap();
    assert!(stats1.chunks_processed > 0);

    // Second scan (no changes)
    let start = Instant::now();
    let stats2 = incremental_update(&pool, worktree_id, &repo).await.unwrap();
    let duration = start.elapsed();

    assert_eq!(stats2.chunks_processed, 0, "Should skip unchanged tree");
    assert!(duration < Duration::from_millis(100), "Should be <100ms");
}
```

### Additional Tests
```rust
#[tokio::test]
async fn test_incremental_only_scans_changed_files() {
    // Create repo with 100 files
    // Modify 1 file
    // Verify incremental processes only 1 file
}

#[tokio::test]
async fn test_deleted_file_removes_worktree() {
    // Index file
    // Delete file, commit
    // Incremental update
    // Verify worktree removed from chunks
}

#[tokio::test]
async fn test_same_content_multiple_worktrees() {
    // Index same repo in two worktrees
    // Verify chunks have both worktree_ids
}
```

Reference the complete test suite specification in `quality-strategy.md` section "Incremental Update Tests" (lines 154-257).

### Test Utilities Required
May need to add to `crates/maproom/tests/helpers.rs`:
- `create_test_repo()` - Create git repo with controlled content
- `get_all_chunks(pool, worktree_id)` - Query all chunks for comparison
- `get_test_pool()` - Database connection for tests

## Dependencies
- BRANCHX-1007 complete (incremental update algorithm implementation)
- BRANCHX-1008 complete (upsert with worktree tracking)
- BRANCHX-1009 complete (deletion handling logic)

## Risk Assessment
- **Risk**: Test passes but production fails (test repo too simple, doesn't catch edge cases)
  - **Mitigation**: Use realistic test repos with varied file types, sizes, and structures
- **Risk**: Tests flaky due to timing issues or race conditions
  - **Mitigation**: Use deterministic git commits with controlled timestamps, avoid time-based assertions except for performance benchmarks
- **Risk**: Performance test fails in CI due to slower environment
  - **Mitigation**: Document baseline performance, allow for CI overhead in assertions

## Files/Packages Affected
- `crates/maproom/tests/incremental_update.rs` (new - main test file)
- `crates/maproom/tests/helpers.rs` (potentially extended with test utilities)
