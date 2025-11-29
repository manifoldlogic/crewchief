# Ticket: BRANCHX-1903: Implement incremental update critical path tests

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - all 8 incremental update tests pass (--test-threads=1)
- [x] **Verified** - by the verify-ticket agent

## Implementation Note

**COMPLETED**: All 8 incremental update tests successfully implemented and passing.

**Test Results** (single-threaded execution):
```
test test_deleted_file_removes_worktree ... ok (✓ Deletion handling verified)
test test_empty_repository ... ok (✓ Empty repository handled gracefully)
test test_first_time_index_init_state ... ok (✓ First-time 'init' state handled)
test test_incremental_equals_full_scan ... ok (✓ Correctness verified via tree SHA)
test test_incremental_only_scans_changed_files ... ok (✓ Only changed files detected)
test test_same_content_multiple_worktrees ... ok (✓ Multi-worktree chunk sharing)
test test_tree_sha_check_performance ... ok (✓ Avg 9ms, max 10ms)
test test_tree_sha_skip_unchanged ... ok (✓ Optimization verified: 8.7ms - 91% faster than target!)

test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 1 filtered out
```

**Performance**: CRITICAL TEST 2 completes in ~8-9ms (91% better than 100ms target)

**Test Execution**: Must run with `--test-threads=1` to avoid database state pollution.

**Command**: `cargo test --test incremental_update -- --ignored --nocapture --test-threads=1`

## Agents
- rust-indexer-engineer
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary
Implement the 8 incremental update tests that are currently TODO stubs in `incremental_update.rs`. These include the two most critical tests (CRITICAL 1 & 2) that guarantee correctness and performance of the branch-aware indexing system.

## Background
The file `crates/maproom/tests/incremental_update.rs` was created in BRANCHX-1010 with comprehensive test framework and documentation. However, all 8 tests are currently just `panic!()` stubs with detailed TODO comments explaining what needs to be implemented.

When run with `--ignored` flag, all tests fail with:
```
panic!("Test not implemented - requires git repository and full scan implementation");
panic!("Test not implemented - requires git repository setup");
panic!("Test not implemented - requires multi-file git repository");
```

These tests are **CRITICAL** for validating BRANCHX before merge:
- **CRITICAL 1**: `test_incremental_equals_full_scan` - Correctness guarantee
- **CRITICAL 2**: `test_tree_sha_skip_unchanged` - Performance optimization (<100ms)

**Planning Reference**: `.crewchief/projects/BRANCHX_branch-aware-indexing/planning/quality-strategy.md` - Critical Path Tests (lines 20-28)

## Acceptance Criteria
- [x] Test file exists with framework and TODO stubs (BRANCHX-1010)
- [x] **CRITICAL 1**: `test_incremental_equals_full_scan` implemented and passing
- [x] **CRITICAL 2**: `test_tree_sha_skip_unchanged` implemented and passing (<100ms - achieved 8.7ms!)
- [x] `test_incremental_only_scans_changed_files` implemented and passing
- [x] `test_deleted_file_removes_worktree` implemented and passing
- [x] `test_same_content_multiple_worktrees` implemented and passing
- [x] `test_tree_sha_check_performance` implemented and passing
- [x] `test_empty_repository` implemented and passing
- [x] `test_first_time_index_init_state` implemented and passing
- [x] All 8 tests pass: `cargo test --test incremental_update -- --ignored --nocapture --test-threads=1`
- [x] Performance verified: tree SHA skip completes in 8.7ms (91% better than 100ms target)
- [x] Tests run reliably (verified with --test-threads=1 to avoid state pollution)

## Technical Requirements

### Test Utilities Needed

Create helper functions in the test file:

```rust
/// Create a temporary git repository with controlled content
async fn create_test_git_repo(files: Vec<(&str, &str)>) -> Result<TempRepo> {
    // Use tempdir for isolation
    // git init
    // Create files with known content
    // git add . && git commit
    // Return repo path and cleanup handle
}

/// Create worktree record in database
async fn create_test_worktree(
    client: &Client,
    repo_name: &str,
    worktree_name: &str,
) -> Result<i64> {
    // INSERT INTO repos
    // INSERT INTO worktrees
    // Return worktree_id
}

/// Get all blob SHAs for chunks in a worktree
async fn get_worktree_chunk_shas(
    client: &Client,
    worktree_id: i64,
) -> Result<HashSet<String>> {
    // SELECT blob_sha FROM chunks WHERE worktree_ids @> $1
}

/// Clean up test data
async fn cleanup_test_data(
    client: &Client,
    repo_id: i64,
) -> Result<()> {
    // DELETE FROM chunks, files, worktrees, repos
    // Cascade should handle most, but verify
}
```

### Test 1: Incremental Equals Full Scan (CRITICAL)

From TODO comments at line 62-86:

```rust
#[tokio::test]
#[ignore]
async fn test_incremental_equals_full_scan() {
    let client = skip_if_no_db!();

    // 1. Create test repo with known content
    let repo = create_test_git_repo(vec![
        ("src/lib.rs", "pub fn hello() { println!(\"Hello\"); }"),
        ("src/main.rs", "fn main() { lib::hello(); }"),
        ("README.md", "# Test Project"),
    ]).await.unwrap();

    // 2. Full scan baseline
    let repo_id = create_test_worktree(&client, "test-repo", "worktree1").await.unwrap();
    // TODO: Call full_scan() or first incremental_update()
    // (first incremental is equivalent to full scan)
    let baseline_shas = get_worktree_chunk_shas(&client, worktree1_id).await.unwrap();

    // 3. Incremental scan on same content
    let worktree2_id = create_test_worktree(&client, "test-repo", "worktree2").await.unwrap();
    // TODO: Call incremental_update() on same repo
    let incremental_shas = get_worktree_chunk_shas(&client, worktree2_id).await.unwrap();

    // 4. Compare
    assert_eq!(baseline_shas.len(), incremental_shas.len(),
        "Incremental and full scan must produce same number of chunks");
    assert_eq!(baseline_shas, incremental_shas,
        "Incremental and full scan must produce identical chunks");

    cleanup_test_data(&client, repo_id).await.unwrap();
}
```

**Success Criteria**: PASS - incremental produces identical blob SHA set as full scan

**Failure Consequence**: CRITICAL BUG - incremental logic is broken, DO NOT MERGE

### Test 2: Tree SHA Skip Unchanged (CRITICAL)

From TODO comments at line 94-134:

```rust
#[tokio::test]
#[ignore]
async fn test_tree_sha_skip_unchanged() {
    let client = skip_if_no_db!();

    // 1. Create and index repository
    let repo = create_test_git_repo(vec![
        ("src/main.rs", "fn main() {}"),
    ]).await.unwrap();

    let worktree_id = create_test_worktree(&client, "test-repo", "main").await.unwrap();

    // First scan (will scan everything)
    // TODO: Call incremental_update()

    // 2. Run incremental_update AGAIN without any changes
    let start = std::time::Instant::now();
    // TODO: Call incremental_update()
    let duration = start.elapsed();

    // 3. Verify <100ms and no files processed
    assert!(duration < Duration::from_millis(100),
        "Tree SHA skip must complete in <100ms, took {:?}", duration);

    // TODO: Verify stats.files_processed == 0
}
```

**Success Criteria**: PASS - duration < 100ms, zero files processed

**Failure Consequence**: Performance regression - optimization broken

### Test 3-8: Supporting Tests

Implement remaining tests following the detailed TODO comments in each test:

- **Test 3** (line 138-177): Verify only changed files scanned
- **Test 4** (line 181-224): Verify deletion removes worktree ID
- **Test 5** (line 228-271): Verify multi-worktree chunk sharing
- **Test 6** (line 275-315): Performance benchmark test
- **Test 7** (line 317-338): Empty repository edge case
- **Test 8** (line 340-364): First-time index initialization

Each has comprehensive implementation guidance in TODO comments.

## Implementation Notes

### Integration with Existing Functions

The tests need to call these existing functions (from BRANCHX implementation):

```rust
use crewchief_maproom::incremental::incremental_update;
use crewchief_maproom::git::{get_git_tree_sha, git_diff_tree};
use crewchief_maproom::db;
```

**Key functions to test**:
- `incremental::tree_sha_update::incremental_update()` - Main entry point
- `incremental::tree_sha_update::get_git_tree_sha()` - Tree SHA retrieval
- `incremental::tree_sha_update::git_diff_tree()` - Change detection
- `incremental::tree_sha_update::remove_worktree_from_chunks()` - Deletion handling

### Database Cleanup

**CRITICAL**: Each test MUST clean up after itself:

```rust
async fn cleanup_test_data(client: &Client, repo_id: i64) -> Result<()> {
    // Delete in correct order (respect FK constraints)
    client.execute("DELETE FROM chunks WHERE file_id IN (
        SELECT id FROM files WHERE worktree_id IN (
            SELECT id FROM worktrees WHERE repo_id = $1
        )
    )", &[&repo_id]).await?;

    client.execute("DELETE FROM files WHERE worktree_id IN (
        SELECT id FROM worktrees WHERE repo_id = $1
    )", &[&repo_id]).await?;

    client.execute("DELETE FROM worktree_index_state WHERE worktree_id IN (
        SELECT id FROM worktrees WHERE repo_id = $1
    )", &[&repo_id]).await?;

    client.execute("DELETE FROM worktrees WHERE repo_id = $1", &[&repo_id]).await?;
    client.execute("DELETE FROM repos WHERE id = $1", &[&repo_id]).await?;

    Ok(())
}
```

### Test Data Design

**Use deterministic content**:
- Known file content → predictable blob SHAs
- Small files (< 1KB) → fast test execution
- TypeScript files → leverage existing tree-sitter parser
- Controlled diffs → known number of changed files

**Example test repository**:
```
test-repo/
├── src/
│   ├── lib.rs          (200 bytes, 1 function)
│   ├── main.rs         (150 bytes, 1 main function)
│   └── utils.rs        (180 bytes, 2 helper functions)
├── README.md           (100 bytes)
└── package.json        (300 bytes)
```

Total: 5 files, ~930 bytes, ~10-15 chunks expected

### Performance Validation

**Tree SHA skip test** (CRITICAL 2):
- Measure with `std::time::Instant`
- Run 10 times, average duration
- All runs must be <100ms
- If >100ms: optimization is broken, FAIL test

**Incremental vs full comparison**:
- Track files_processed, chunks_processed
- Incremental should process ~20% of files (for typical changes)
- Document speedup ratio in test output

## Dependencies
- BRANCHX-1006 complete (git integration functions) ✅ PASSING
- BRANCHX-1009 complete (incremental update functions implemented)
- BRANCHX-1010 complete (test framework file created with TODO stubs)
- **BRANCHX-1904 complete (schema migration)** ⚠️ REQUIRED - Tests cannot run without relpath/content columns
- Database accessible with DATABASE_URL ✅ VERIFIED
- Migrations 001-005 applied (including complete BRANCHX schema)

**BLOCKED until BRANCHX-1904 completes schema migration**.

## Risk Assessment
- **Risk**: Tests flaky due to timing dependencies
  - **Mitigation**: Use deterministic test data, generous timeouts for CI, focus on relative performance
- **Risk**: Git operations slow in test environment
  - **Mitigation**: Use small repositories (5 files, <1KB each), tempdir on fast storage
- **Risk**: Database cleanup incomplete, test pollution
  - **Mitigation**: Comprehensive cleanup function, verify cascade deletes work, use unique repo names per test
- **Risk**: Tree SHA optimization doesn't actually work (<100ms fails)
  - **Mitigation**: This would be a legitimate bug to catch - FAIL the test and fix the implementation

## Files/Packages Affected
- `crates/maproom/tests/incremental_update.rs` (modify - implement 8 test bodies)
- `crates/maproom/tests/common/` (potentially - add test utilities if needed)

## Success Metrics
- All 8 tests pass locally
- All 8 tests pass 10 times consecutively (no flakiness)
- CRITICAL 1 passes: Correctness guaranteed
- CRITICAL 2 passes: Performance <100ms validated
- Can proceed with BRANCHX merge after this ticket complete

## Planning References
- `.crewchief/projects/BRANCHX_branch-aware-indexing/planning/quality-strategy.md` - Critical Path Tests
- `.crewchief/projects/BRANCHX_branch-aware-indexing/planning/architecture.md` - Incremental Update Algorithm
- `crates/maproom/tests/incremental_update.rs` - Detailed TODO comments for each test
