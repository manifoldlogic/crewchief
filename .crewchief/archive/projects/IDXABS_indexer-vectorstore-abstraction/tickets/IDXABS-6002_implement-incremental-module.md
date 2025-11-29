# Ticket: IDXABS-6002: Implement Stubbed Incremental Module Functions

## Status
- [ ] **Task completed** - all incremental functions implemented
- [ ] **Tests pass** - incremental module tests pass
- [ ] **Verified** - by the verify-ticket agent

## Agents
- rust-indexer-engineer
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary
Implement the stubbed functions in the incremental module that were left as TODOs during the PostgreSQL to SQLite migration. These functions are required for the `watch` command to work.

## Background
The IDXABS-2006 ticket marked the incremental module as complete, but the functions were only stubbed with TODO comments. The actual implementation was deferred. This ticket completes that work.

## Acceptance Criteria
- [ ] `IncrementalProcessor::index_new_file()` fully implemented
- [ ] `IncrementalProcessor::update_file()` fully implemented
- [ ] `IncrementalProcessor::remove_file()` fully implemented
- [ ] `get_hash_from_db()` queries file hash from SQLite
- [ ] `store_hash_in_db()` stores file hash in SQLite
- [ ] `EdgeUpdater::update_edges()` updates chunk edges
- [ ] `remove_worktree_from_chunks()` removes worktree from chunk arrays
- [ ] `incremental_update()` performs git-based incremental indexing
- [ ] All implementations use existing `SqliteStore` methods
- [ ] Unit tests validate each function

## Technical Requirements

### 1. processor.rs - IncrementalProcessor

#### `index_new_file(path, hash)` Implementation
```rust
async fn index_new_file(&self, path: &Path, hash: &ContentHash) -> Result<()> {
    // 1. Read file content (already done in stub)
    // 2. Parse file to extract chunks using crate::indexer::parser
    // 3. Get or create file record: store.upsert_file(FileRecord { ... })
    // 4. Insert chunks: store.insert_chunks_batch(chunks)
    // 5. Update edges via edge_updater.update_edges(file_id)
    // 6. Update file hash in database
}
```

#### `update_file(path, new_hash)` Implementation
```rust
async fn update_file(&self, path: &Path, new_hash: &ContentHash) -> Result<()> {
    // 1. Read file content (already done in stub)
    // 2. Parse file to extract new chunks
    // 3. Get file_id from database by relpath
    // 4. Delete old chunks: store.delete_chunks_by_file(file_id)
    // 5. Insert new chunks: store.insert_chunks_batch(chunks)
    // 6. Update edges: edge_updater.update_edges(file_id)
    // 7. Update file hash in database
}
```

#### `remove_file(path)` Implementation
```rust
async fn remove_file(&self, path: &Path) -> Result<()> {
    // 1. Get file_id from database by relpath
    // 2. Delete chunks: store.delete_chunks_by_file(file_id)
    //    (CASCADE handles edges automatically)
    // 3. Optionally delete file record
}
```

### 2. detector.rs - Change Detection

#### `get_hash_from_db(store, file_id)` Implementation
```rust
pub async fn get_hash_from_db(store: &SqliteStore, file_id: i64) -> Result<Option<ContentHash>> {
    // Query: SELECT blake3_hash FROM files WHERE id = ?
    // Parse hex string to ContentHash
}
```

#### `store_hash_in_db(store, file_id, hash)` Implementation
```rust
pub async fn store_hash_in_db(store: &SqliteStore, file_id: i64, hash: ContentHash) -> Result<()> {
    // Query: UPDATE files SET blake3_hash = ? WHERE id = ?
}
```

### 3. edge_updater.rs - Edge Updates

#### `update_edges(file_id)` Implementation
```rust
pub async fn update_edges(&self, file_id: i64) -> Result<()> {
    // 1. Get chunk_ids for file: store.get_file_chunks(file_id)
    // 2. Delete old edges involving these chunks
    // 3. Compute new edges: compute_edges(store, chunk_ids)
    // 4. Insert new edges: insert_edges(store, edges)
}
```

### 4. tree_sha_update.rs - Git-based Updates

#### `remove_worktree_from_chunks(store, worktree_id, relpath)` Implementation
```rust
pub async fn remove_worktree_from_chunks(
    store: &SqliteStore,
    worktree_id: i64,
    relpath: &str,
) -> Result<i64> {
    // 1. Find chunks for this file/worktree
    // 2. Remove worktree_id from chunk's worktree_ids array
    // 3. Delete chunks with empty worktree_ids arrays
    // Return count of affected chunks
}
```

#### `incremental_update(store, worktree_id, repo_path)` Implementation
```rust
pub async fn incremental_update(
    store: &SqliteStore,
    worktree_id: i64,
    repo_path: &Path,
) -> Result<UpdateStats> {
    // 1. Get current git tree SHA: get_git_tree_sha(repo_path)
    // 2. Get last indexed tree SHA: store.get_last_indexed_tree(worktree_id)
    // 3. If same, return UpdateStats::skipped()
    // 4. Find changed files: git_diff_tree(repo_path, old_sha, new_sha)
    // 5. Process changes using IncrementalProcessor
    // 6. Update index state: store.update_index_state(worktree_id, new_sha, stats)
    // 7. Return stats
}
```

## Required SqliteStore Methods (Already Exist)
- `upsert_file(FileRecord)` - Create/update file record
- `insert_chunks_batch(chunks)` - Batch insert chunks
- `delete_chunks_by_file(file_id)` - Delete chunks for file
- `get_file_chunks(file_id)` - Get chunks for file
- `insert_chunk_edge(edge)` - Insert single edge
- `get_last_indexed_tree(worktree_id)` - Get last indexed SHA
- `update_index_state(worktree_id, sha, stats)` - Update index state

## Required SqliteStore Methods (May Need Adding)
- `get_file_by_relpath(relpath)` - Get file by path
- `update_file_hash(file_id, hash)` - Update file hash
- `delete_edges_for_chunks(chunk_ids)` - Delete edges by chunk IDs

## Dependencies
- IDXABS-6001 (tests must compile first to validate implementations)

## Risk Assessment
- **Risk**: Missing SqliteStore methods
  - **Mitigation**: Add methods following existing patterns
- **Risk**: Performance issues with large files
  - **Mitigation**: Use batch operations, existing size limits

## Files/Packages Affected
Files to MODIFY:
- `crates/maproom/src/incremental/processor.rs`
- `crates/maproom/src/incremental/detector.rs`
- `crates/maproom/src/incremental/edge_updater.rs`
- `crates/maproom/src/incremental/tree_sha_update.rs`
- `crates/maproom/src/db/sqlite/mod.rs` (if new methods needed)

## Estimated Effort
Medium-High - Core business logic implementation with database integration.
