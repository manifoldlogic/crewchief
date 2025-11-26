# Ticket: VECSTORE-1006: Cleanup and Maintenance Methods

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - tests executed and passing (or N/A if no tests)
- [ ] **Verified** - by the verify-ticket agent

**Note on "Tests pass"**:
- If tests were created/modified, you MUST run them and show output
- "Tests pass" means tests were EXECUTED and all passed
- "Tests pass - N/A" is only valid for documentation-only tickets
- Test file existence alone does NOT satisfy this requirement

## Agents
- rust-indexer-engineer
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary
Add cleanup and maintenance methods to the `VectorStore` trait: `detect_stale_worktrees()`, `delete_worktree_data()`, `delete_chunks_by_file()`, and `get_chunks_by_blob_sha()`. These enable detecting and cleaning up stale indexed data.

## Background
The cleanup command needs to detect stale worktrees (deleted from disk but still indexed) and remove their data. The indexer needs incremental operations to delete/update individual file chunks. PostgreSQL has cleanup logic in `cleanup.rs`, but it needs to be exposed through the trait.

**Current State**:
- PostgreSQL: `db/cleanup.rs` has `StaleWorktreeDetector` class - partial exists
- SQLite: No cleanup functionality - must be implemented
- Trait: No cleanup methods defined

**Reference**: Plan Phase 5 - Cleanup Methods (VECSTORE-1006)

## Acceptance Criteria
- [ ] `StaleWorktree` and `CleanupReport` types defined in `db/mod.rs`
- [ ] `detect_stale_worktrees()` method added to trait and implemented
- [ ] `delete_worktree_data()` method added to trait and implemented
- [ ] `delete_chunks_by_file()` method added to trait and implemented
- [ ] `get_chunks_by_blob_sha()` method added to trait and implemented
- [ ] PostgresStore wraps/refactors existing `cleanup.rs` functionality
- [ ] SqliteStore has equivalent implementation
- [ ] Cleanup returns accurate counts
- [ ] Contract tests pass for both backends

## Technical Requirements

### Domain Types
Add to `crates/maproom/src/db/mod.rs`:

```rust
/// A worktree that no longer exists on disk
pub struct StaleWorktree {
    pub id: i64,
    pub repo_id: i64,
    pub name: String,
    pub abs_path: String,
    pub reason: String,  // e.g., "path_not_found", "not_a_directory"
}

/// Report from cleanup operations
pub struct CleanupReport {
    pub worktree_id: i64,
    pub chunks_deleted: u64,
    pub files_deleted: u64,
    pub embeddings_deleted: u64,
}
```

### Trait Method Signatures
Add to `VectorStore` trait:

```rust
/// Detect worktrees that no longer exist on disk
async fn detect_stale_worktrees(&self, repo_id: i64) -> anyhow::Result<Vec<StaleWorktree>>;

/// Delete all data for a worktree (chunks, files, embeddings)
async fn delete_worktree_data(&self, worktree_id: i64) -> anyhow::Result<CleanupReport>;

/// Delete all chunks for a specific file (for incremental re-indexing)
async fn delete_chunks_by_file(&self, file_id: i64) -> anyhow::Result<u64>;

/// Get chunks by blob SHA (for content-addressed deduplication)
async fn get_chunks_by_blob_sha(&self, blob_sha: &str) -> anyhow::Result<Vec<ChunkSummary>>;
```

### PostgreSQL Implementation

**Refactor from cleanup.rs**:

The existing `StaleWorktreeDetector` class needs to be refactored into standalone functions:

```rust
// crates/maproom/src/db/queries.rs (or cleanup.rs refactored)

pub async fn detect_stale_worktrees(
    client: &impl GenericClient,
    repo_id: i64,
) -> anyhow::Result<Vec<StaleWorktree>> {
    // Get all worktrees for repo
    // Check each path exists on disk
    // Return those that don't exist
}

pub async fn delete_worktree_data(
    client: &impl GenericClient,
    worktree_id: i64,
) -> anyhow::Result<CleanupReport> {
    // Transaction:
    // 1. Count chunks to be deleted
    // 2. Delete embeddings for chunks
    // 3. Delete chunk_edges
    // 4. Delete chunks
    // 5. Delete files
    // 6. Delete index_state
    // 7. Optionally delete worktree record
    // Return counts
}

pub async fn delete_chunks_by_file(
    client: &impl GenericClient,
    file_id: i64,
) -> anyhow::Result<u64> {
    // DELETE FROM embeddings WHERE chunk_id IN (SELECT id FROM chunks WHERE file_id = $1)
    // DELETE FROM chunk_edges WHERE src_chunk_id IN (...) OR dst_chunk_id IN (...)
    // DELETE FROM chunks WHERE file_id = $1
    // Return count of deleted chunks
}

pub async fn get_chunks_by_blob_sha(
    client: &impl GenericClient,
    blob_sha: &str,
) -> anyhow::Result<Vec<ChunkSummary>> {
    // SELECT chunk fields FROM chunks WHERE blob_sha = $1
}
```

### SQLite Implementation

**File: `crates/maproom/src/db/sqlite/cleanup.rs` (NEW)**

```rust
pub fn detect_stale_worktrees(conn: &Connection, repo_id: i64) -> anyhow::Result<Vec<StaleWorktree>> {
    let mut stmt = conn.prepare(
        "SELECT id, repo_id, name, abs_path FROM worktrees WHERE repo_id = ?"
    )?;

    let worktrees: Vec<_> = stmt.query_map([repo_id], |row| {
        Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?, row.get::<_, String>(2)?, row.get::<_, String>(3)?))
    })?.collect::<Result<Vec<_>, _>>()?;

    let mut stale = Vec::new();
    for (id, repo_id, name, abs_path) in worktrees {
        let path = std::path::Path::new(&abs_path);
        if !path.exists() {
            stale.push(StaleWorktree {
                id,
                repo_id,
                name,
                abs_path,
                reason: "path_not_found".to_string(),
            });
        } else if !path.is_dir() {
            stale.push(StaleWorktree {
                id,
                repo_id,
                name,
                abs_path,
                reason: "not_a_directory".to_string(),
            });
        }
    }
    Ok(stale)
}

pub fn delete_worktree_data(conn: &Connection, worktree_id: i64) -> anyhow::Result<CleanupReport> {
    // SQLite transaction for atomic deletion
    // Similar deletion sequence as PostgreSQL
}

pub fn delete_chunks_by_file(conn: &Connection, file_id: i64) -> anyhow::Result<u64> {
    // Delete embeddings, edges, chunks
}

pub fn get_chunks_by_blob_sha(conn: &Connection, blob_sha: &str) -> anyhow::Result<Vec<ChunkSummary>> {
    // Query chunks by blob_sha
}
```

## Implementation Notes

### Transaction Handling
`delete_worktree_data()` involves multiple tables and should be atomic:
- PostgreSQL: Use explicit transaction
- SQLite: Use `conn.execute_batch()` or `Transaction`

### Stale Detection Logic
Stale detection checks filesystem paths:
- Path doesn't exist → stale
- Path exists but not a directory → stale
- Path exists and is directory → not stale

### Cascade Deletion Order
When deleting worktree data, respect foreign keys:
1. Delete embeddings (references chunks)
2. Delete chunk_edges (references chunks)
3. Delete chunks (references files)
4. Delete files (references worktree)
5. Delete index_state (references worktree)
6. Optionally delete worktree record itself

### Counts in CleanupReport
The report should return accurate counts of what was deleted for user feedback.

## Dependencies
- **VECSTORE-1004**: Repository Query Methods (for listing worktrees)

## Risk Assessment
- **Risk**: Cascade deletion misses related records
  - **Mitigation**: Careful query ordering, test with actual data
- **Risk**: Filesystem check has race conditions
  - **Mitigation**: Accept that stale detection is advisory; deletion is idempotent
- **Risk**: Large deletion causes long lock
  - **Mitigation**: Could batch deletions, but for MVP single transaction is acceptable

## Files/Packages Affected
- `crates/maproom/src/db/mod.rs` (types + trait)
- `crates/maproom/src/db/queries.rs` (PostgreSQL queries)
- `crates/maproom/src/db/cleanup.rs` (refactor existing)
- `crates/maproom/src/db/postgres/mod.rs` (PostgresStore impl)
- `crates/maproom/src/db/sqlite/mod.rs` (SqliteStore impl)
- `crates/maproom/src/db/sqlite/cleanup.rs` (NEW - SQLite queries)
