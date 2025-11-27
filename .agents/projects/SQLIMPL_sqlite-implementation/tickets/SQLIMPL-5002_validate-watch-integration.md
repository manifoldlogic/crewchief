# Ticket: SQLIMPL-5002: Validate Watch Integration

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
Validate that the watch command correctly detects file changes and triggers re-indexing via the incremental module. Update and enable the watch integration tests.

## Background
With the watch command enabled (SQLIMPL-5001), we need to validate the full integration: file changes should be detected, trigger incremental updates, and reflect in search results.

This ticket implements Plan Phase 5, Ticket 5002: "Validate Watch Integration".

## Acceptance Criteria
- [ ] Watch detects file creation events
- [ ] Watch detects file modification events
- [ ] Watch detects file deletion events
- [ ] Changes trigger re-indexing via incremental module
- [ ] `tests/watch_integration.rs` tests pass
- [ ] `tests/unified_watch_test.rs` tests pass (if applicable)
- [ ] Phase 5 gate achieved: watch monitors and updates continuously

## Technical Requirements
- Enable/update watch tests from Phase 1 migration
- Test with real file system operations
- Verify database updates after file changes
- Ensure debouncing works correctly

## Implementation Notes

### Test Scenarios

#### 1. File Creation Detection
```rust
#[tokio::test]
async fn watch_detects_file_creation() {
    let dir = tempdir().unwrap();
    let store = setup_test_store();

    // Start watch in background
    let watch_handle = spawn_watch(&store, dir.path());

    // Create new file
    fs::write(dir.path().join("new.rs"), "fn new() {}").unwrap();

    // Wait for watch to process
    tokio::time::sleep(Duration::from_secs(1)).await;

    // Verify chunk was created
    let results = store.search_fts("new").await.unwrap();
    assert!(!results.is_empty(), "New file should be indexed");

    watch_handle.abort();
}
```

#### 2. File Modification Detection
```rust
#[tokio::test]
async fn watch_detects_file_modification() {
    let dir = tempdir().unwrap();
    let store = setup_test_store();

    // Create initial file
    let file = dir.path().join("existing.rs");
    fs::write(&file, "fn original() {}").unwrap();

    // Initial index
    store.scan(dir.path()).await.unwrap();

    // Start watch
    let watch_handle = spawn_watch(&store, dir.path());

    // Modify file
    fs::write(&file, "fn modified() {}").unwrap();

    // Wait for watch to process
    tokio::time::sleep(Duration::from_secs(1)).await;

    // Verify update
    let results = store.search_fts("modified").await.unwrap();
    assert!(!results.is_empty(), "Modified content should be searchable");

    let old_results = store.search_fts("original").await.unwrap();
    assert!(old_results.is_empty(), "Old content should not be found");

    watch_handle.abort();
}
```

#### 3. File Deletion Detection
```rust
#[tokio::test]
async fn watch_detects_file_deletion() {
    let dir = tempdir().unwrap();
    let store = setup_test_store();

    // Create and index file
    let file = dir.path().join("delete_me.rs");
    fs::write(&file, "fn delete_me() {}").unwrap();
    store.scan(dir.path()).await.unwrap();

    // Verify indexed
    let results = store.search_fts("delete_me").await.unwrap();
    assert!(!results.is_empty());

    // Start watch
    let watch_handle = spawn_watch(&store, dir.path());

    // Delete file
    fs::remove_file(&file).unwrap();

    // Wait for watch to process
    tokio::time::sleep(Duration::from_secs(1)).await;

    // Verify removed from index
    let results = store.search_fts("delete_me").await.unwrap();
    assert!(results.is_empty(), "Deleted file should be removed from index");

    watch_handle.abort();
}
```

### Phase 5 Gate Verification
Manual verification steps:
```bash
# Terminal 1: Start watch
cargo run -p crewchief-maproom -- watch --repo test --path ./test-repo

# Terminal 2: Make changes
echo "fn new_function() {}" >> ./test-repo/src/lib.rs

# Terminal 1 should show: "Processing 1 changed files..."

# Verify search finds new content
cargo run -p crewchief-maproom -- search "new_function"
```

### Test Files to Update
- `tests/watch_integration.rs` - Main integration tests
- `tests/unified_watch_test.rs` - Unified watch tests

## Dependencies
- SQLIMPL-5001 (Enable Watch Command)
- Phase 3 Complete (Incremental working)

## Risk Assessment
- **Risk**: Tests may be flaky due to timing
  - **Mitigation**: Use generous timeouts; retry logic
- **Risk**: Platform-specific file watcher behavior
  - **Mitigation**: Test on CI; document platform differences

## Files/Packages Affected
- `crates/maproom/tests/watch_integration.rs`
- `crates/maproom/tests/unified_watch_test.rs`
