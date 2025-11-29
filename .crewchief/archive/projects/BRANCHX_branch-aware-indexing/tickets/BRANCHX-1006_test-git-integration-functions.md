# Ticket: BRANCHX-1006: Test git integration and index state functions

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - git integration tests pass (8/8); database tests compile and marked #[ignore]
- [x] **Verified** - by the verify-ticket agent

## Agents
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary
Verify git tree SHA detection, diff-tree parsing, and database state management work correctly through comprehensive integration tests.

## Background
This is Phase 2, Step 2.3 of BRANCHX. After implementing git functions (BRANCHX-1004) and database state functions (BRANCHX-1005), we need comprehensive tests to ensure change detection works reliably. These are critical tests because the entire incremental update optimization depends on correctly detecting what changed.

Reference: `.crewchief/projects/BRANCHX_branch-aware-indexing/planning/plan.md` - Phase 2.3

## Acceptance Criteria
- [x] `test_get_git_tree_sha` verifies tree SHA format (40 or 64 hex chars for SHA-1/SHA-256)
- [x] `test_tree_sha_changes_on_modification` verifies tree SHA changes when file modified
- [x] `test_tree_sha_unchanged_for_same_content` verifies tree SHA stable
- [x] `test_git_diff_tree_detects_changes` verifies A/M/D detection
- [x] `test_diff_tree_parses_correctly` verifies output parsing
- [x] Integration tests for database state functions (5 tests, marked #[ignore] for database requirement)
- [x] All tests pass locally (8/8 git tests pass; 5/5 database tests compile and marked #[ignore])

## Technical Requirements
- Create test git repositories with controlled content
- Use actual git commands (not mocked)
- Test edge cases: empty repo, initial commit, deleted files
- Verify diff-tree handles renamed files (skip for now, just A/M/D)
- Test database functions with test pool
- Test concurrent updates to index state

## Implementation Notes

Test file: `crates/maproom/tests/git_integration.rs`

From `quality-strategy.md` section "Git Integration Tests":

```rust
#[test]
fn test_get_git_tree_sha() {
    let repo = create_test_repo();
    let tree_sha = get_git_tree_sha(&repo).unwrap();

    // Should be valid SHA (64 hex chars for SHA-256)
    assert_eq!(tree_sha.len(), 64);
    assert!(tree_sha.chars().all(|c| c.is_ascii_hexdigit()));
}

#[test]
fn test_tree_sha_changes_on_modification() {
    let repo = create_test_repo();
    let tree1 = get_git_tree_sha(&repo).unwrap();

    // Modify a file
    std::fs::write(repo.join("file.ts"), "modified").unwrap();
    git_commit(&repo, "Modify file");

    let tree2 = get_git_tree_sha(&repo).unwrap();

    assert_ne!(tree1, tree2, "Tree SHA must change when content changes");
}

#[test]
fn test_git_diff_tree_detects_changes() {
    let repo = create_test_repo();
    git_commit(&repo, "Initial");
    let tree1 = get_git_tree_sha(&repo).unwrap();

    // Add, modify, delete files
    std::fs::write(repo.join("new.ts"), "new").unwrap();
    std::fs::write(repo.join("existing.ts"), "modified").unwrap();
    std::fs::remove_file(repo.join("old.ts")).unwrap();
    git_commit(&repo, "Changes");

    let tree2 = get_git_tree_sha(&repo).unwrap();
    let changes = git_diff_tree(&tree1, &tree2, &repo).unwrap();

    assert_eq!(changes.iter().filter(|c| matches!(c.status, FileStatus::Added)).count(), 1);
    assert_eq!(changes.iter().filter(|c| matches!(c.status, FileStatus::Modified)).count(), 1);
    assert_eq!(changes.iter().filter(|c| matches!(c.status, FileStatus::Deleted)).count(), 1);
}
```

Database state tests in `crates/maproom/tests/index_state.rs`:
```rust
#[tokio::test]
async fn test_get_last_indexed_tree_returns_init() {
    let pool = get_test_pool().await;
    let result = get_last_indexed_tree(&pool, 999).await.unwrap();
    assert_eq!(result, "init");
}

#[tokio::test]
async fn test_update_and_retrieve_index_state() {
    let pool = get_test_pool().await;
    let worktree_id = create_test_worktree(&pool).await.unwrap();

    let stats = UpdateStats {
        files_processed: 10,
        chunks_processed: 100,
        embeddings_generated: 50,
    };

    update_index_state(&pool, worktree_id, "abc123", &stats).await.unwrap();

    let retrieved = get_last_indexed_tree(&pool, worktree_id).await.unwrap();
    assert_eq!(retrieved, "abc123");
}
```

See `quality-strategy.md` section "Git Integration Tests" for complete test suite.

## Dependencies
- BRANCHX-1004 complete (git functions implemented)
- BRANCHX-1005 complete (database functions implemented)

## Risk Assessment
- **Risk**: Tests fail in CI due to git not available
  - **Mitigation**: Document git requirement, add CI setup step
- **Risk**: Test repos conflict with actual repos
  - **Mitigation**: Use temp directories, clean up after tests

## Files/Packages Affected
- `crates/maproom/tests/git_integration.rs` (new - 301 lines, 8 tests)
- `crates/maproom/tests/index_state.rs` (new - 336 lines, 5 tests)
