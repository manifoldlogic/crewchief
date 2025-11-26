# Ticket: SQLITE-1002: CRUD Updates for Junction Table

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
Update all CRUD operations to use the new `chunk_worktrees` junction table instead of the deprecated `worktree_ids JSON` column.

## Background
With the schema migration to junction table complete, all queries and mutations need to use the new structure. This affects chunk insertion, worktree filtering, and batch operations.

Implements: Plan Phase 1 - Schema Foundation

## Acceptance Criteria
- [x] `insert_chunk` inserts into both `chunks` and `chunk_worktrees` tables
- [x] `add_chunk_to_worktree(chunk_id, worktree_id)` method adds junction record
- [x] `get_chunk_worktrees(chunk_id)` returns list of worktree IDs for a chunk
- [x] All queries filtering by worktree use JOIN on `chunk_worktrees`
- [x] Batch insert operations use transactions for atomicity
- [x] Removing worktree_ids JSON references doesn't break existing code
- [x] Unit tests pass for junction table operations

## Technical Requirements
Update `crates/maproom/src/db/sqlite/mod.rs`:

```rust
impl SqliteStore {
    /// Insert chunk with worktree associations
    pub async fn insert_chunk(
        &self,
        file_id: i64,
        chunk: &ChunkRecord,
        worktree_ids: &[i64],
    ) -> Result<i64> {
        self.run(|conn| {
            let tx = conn.transaction()?;

            // Insert chunk (without worktree_ids column)
            let chunk_id = tx.execute(
                "INSERT INTO chunks (...) VALUES (...)",
                params![...],
            )?;

            // Insert junction records
            for &wt_id in worktree_ids {
                tx.execute(
                    "INSERT OR IGNORE INTO chunk_worktrees (chunk_id, worktree_id) VALUES (?1, ?2)",
                    params![chunk_id, wt_id],
                )?;
            }

            tx.commit()?;
            Ok(chunk_id)
        }).await
    }

    /// Add chunk to additional worktree
    pub async fn add_chunk_to_worktree(&self, chunk_id: i64, worktree_id: i64) -> Result<()>;

    /// Get all worktrees containing this chunk
    pub async fn get_chunk_worktrees(&self, chunk_id: i64) -> Result<Vec<i64>>;
}
```

Update worktree filtering in search queries:
```sql
-- Before (JSON)
WHERE json_array_contains(c.worktree_ids, ?worktree_id)

-- After (JOIN)
JOIN chunk_worktrees cw ON c.id = cw.chunk_id
WHERE cw.worktree_id = ?worktree_id
```

## Implementation Notes
- Use `INSERT OR IGNORE` for junction to handle duplicates gracefully
- Batch inserts should use a single transaction for multiple chunks
- The `worktree_ids` column may still exist during transition - don't query it
- Update any existing tests that mock worktree_ids JSON

## Dependencies
- SQLITE-1001 (Schema Migration) - junction table must exist

## Risk Assessment
- **Risk**: Performance regression from additional JOINs
  - **Mitigation**: Junction table has covering index on worktree_id; benchmark if slow
- **Risk**: Existing code references worktree_ids JSON column
  - **Mitigation**: grep for worktree_ids usage, update all references

## Files/Packages Affected
- `crates/maproom/src/db/sqlite/mod.rs` (update CRUD operations)
- `crates/maproom/src/db/sqlite/schema.rs` (update schema documentation)
- Any test files that use worktree_ids

---

## Implementation Summary

All CRUD operations have been successfully updated to use the `chunk_worktrees` junction table. The implementation includes:

### Changes Made

1. **Updated `insert_chunk` method** (lines 240-306):
   - Removed `worktree_ids` JSON column from INSERT statement
   - Added transaction wrapper for atomicity
   - Inserts into `chunk_worktrees` junction table after chunk insertion
   - Uses `INSERT OR IGNORE` to handle duplicates gracefully
   - Updates FTS index within same transaction

2. **Updated `insert_chunks_batch` method** (lines 308-371):
   - Removed `worktree_ids` JSON column from batch INSERT
   - Added junction table inserts within transaction loop
   - Prepared statement for junction inserts for efficiency
   - Maintains atomicity with single transaction for all operations

3. **Added `add_chunk_to_worktree` method** (lines 760-768):
   - Public async method for adding chunks to additional worktrees
   - Uses `INSERT OR IGNORE` for idempotency
   - Follows existing async `spawn_blocking` pattern

4. **Added `get_chunk_worktrees` method** (lines 771-782):
   - Public async method to retrieve all worktrees for a chunk
   - Returns `Vec<i64>` of worktree IDs
   - Efficient single query with prepared statement

5. **Updated `search_chunks_fts` method** (lines 611-665):
   - Changed from JSON array contains to JOIN on `chunk_worktrees`
   - Conditional SQL generation based on worktree filter
   - Separate query branches to handle worktree filtering efficiently
   - Restructured result collection to avoid Rust type inference issues

6. **Code cleanup**:
   - Removed unused imports (`DateTime`, `Utc`, `Mutex`)
   - Prefixed unused parameters with `_` (`_dimension`, `_debug`)

### Tests Added

Two comprehensive integration tests in `mod.rs`:

1. **`test_junction_table_operations`**:
   - Tests single chunk insertion with junction table
   - Verifies `get_chunk_worktrees` returns correct worktrees
   - Tests `add_chunk_to_worktree` for adding to additional worktrees
   - Validates idempotency of junction inserts

2. **`test_batch_insert_with_junction`**:
   - Tests batch insertion of multiple chunks
   - Verifies all chunks are properly inserted into junction table
   - Validates transaction atomicity

### Verification

- Compilation: `cargo check --features sqlite` - SUCCESS
- Migration tests: All 5 tests pass
- Junction table tests: Both new tests pass (2/2)
- No breaking changes to existing code

### Performance Notes

- Junction table has index on `worktree_id` (created by migration 2)
- JOIN operations are efficient due to covering index
- Transaction-wrapped operations maintain data integrity
- `INSERT OR IGNORE` prevents duplicate key violations without errors
