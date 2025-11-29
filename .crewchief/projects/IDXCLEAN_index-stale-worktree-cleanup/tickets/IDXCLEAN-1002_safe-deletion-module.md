# Ticket: IDXCLEAN-1002: Implement Safe Worktree Deletion Module with Array-Based Cleanup

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - unit tests executed and passing (5/5 passed)
- [x] **Verified** - by the verify-ticket agent

## Agents
- rust-indexer-engineer
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary
Create transaction-based deletion module that safely removes stale worktrees using array-based JSONB removal (not CASCADE) to protect multi-worktree chunks, with dry-run support and audit logging.

## Background
This is the second ticket in Phase 1 of the IDXCLEAN project. After implementing stale detection (IDXCLEAN-1001), we need safe deletion logic that removes stale worktrees from the database without accidentally deleting valid ones or corrupting the database.

The database uses Migration 0020's `worktree_ids JSONB` array pattern where chunks can belong to multiple worktrees. Deleting a worktree requires:
1. Removing worktree ID from all chunks' `worktree_ids` arrays
2. Garbage collecting chunks with empty arrays
3. Deleting the worktree record

This pattern mirrors `incremental/tree_sha_update.rs::remove_worktree_from_chunks()` but operates at worktree-level scope instead of file-level scope.

**Critical safety requirement:** Multi-worktree chunks MUST be preserved when only one worktree is deleted.

**References:**
- plan.md: Phase 1 - Core Cleanup Infrastructure, ticket IDXCLEAN-1002 (lines 141-180)
- architecture.md: Section 2 - Safe Deletion Module (lines 90-335)

## Acceptance Criteria
- [ ] `WorktreeCleaner` struct created in `crates/maproom/src/db/cleanup.rs`
- [ ] `cleanup_stale_worktrees()` method deletes worktrees within single transaction
- [ ] Array-based deletion: `UPDATE chunks SET worktree_ids = worktree_ids - $1::TEXT WHERE worktree_ids ? $1::TEXT`
- [ ] Garbage collection: `DELETE FROM chunks WHERE jsonb_array_length(worktree_ids) = 0`
- [ ] Worktree deletion: `DELETE FROM worktrees WHERE id = $1`
- [ ] Multi-worktree chunks preserved (chunk in 2 worktrees, delete 1, verify chunk preserved)
- [ ] Single-worktree chunks garbage collected (chunk in 1 worktree, delete it, verify chunk deleted)
- [ ] Reuses SQL pattern from `incremental/tree_sha_update.rs::remove_worktree_from_chunks()`
- [ ] Dry-run mode supported (no actual deletion, returns report showing what would be deleted)
- [ ] Returns `CleanupReport` with statistics: total_stale, deleted_count, chunks_cleaned, failed_count
- [ ] Audit logging: Every deletion logged with tracing::info (worktree_id, name, abs_path, chunks_cleaned)
- [ ] Transaction rollback on any error (ACID guarantees)
- [ ] Partial failure handling: Continue deleting even if one fails, collect errors in report
- [ ] Unit tests for dry-run mode, report generation
- [ ] Integration test: Multi-worktree chunk safety (Scenario 4 from quality-strategy.md)
- [ ] Integration test: Garbage collection accuracy (Scenario 5 from quality-strategy.md)

## Technical Requirements
- All deletions in single transaction for atomicity
- Use sqlx transactions: `let mut tx = client.transaction().await?`
- Array-based deletion pattern (NOT CASCADE) to protect multi-worktree chunks
- GIN index on `worktree_ids` makes `WHERE worktree_ids ? 'X'` queries fast
- Structured audit logging with tracing::info (JSON-compatible fields)
- Handle concurrent operations safely (transaction isolation)
- Performance target: < 2 seconds for 95 deletions with 500,000 chunks

## Implementation Notes

### Core Structure
```rust
// crates/maproom/src/db/cleanup.rs

pub struct WorktreeCleaner {
    db: DatabaseConnection,
    dry_run: bool,
}

impl WorktreeCleaner {
    /// Deletes stale worktrees within a transaction
    pub async fn cleanup_stale_worktrees(
        &self,
        stale: Vec<StaleWorktree>,
    ) -> Result<CleanupReport> {
        if self.dry_run {
            return Ok(self.create_dry_run_report(&stale));
        }

        let mut tx = self.db.begin_transaction().await?;
        let mut deleted_ids = Vec::new();
        let mut chunks_cleaned = 0;
        let mut failed_deletions = Vec::new();

        for wt in stale {
            match self.delete_worktree_tx(&mut tx, wt.id).await {
                Ok(cleaned) => {
                    deleted_ids.push(wt.id);
                    chunks_cleaned += cleaned;
                    tracing::info!(
                        worktree_id = wt.id,
                        name = %wt.name,
                        abs_path = %wt.abs_path,
                        chunks_cleaned = cleaned,
                        "Deleted stale worktree"
                    );
                }
                Err(e) => {
                    tracing::error!(
                        worktree_id = wt.id,
                        error = %e,
                        "Failed to delete stale worktree"
                    );
                    failed_deletions.push((wt.id, e));
                }
            }
        }

        tx.commit().await?;

        Ok(CleanupReport {
            total_stale: stale.len(),
            deleted_count: deleted_ids.len(),
            chunks_cleaned,
            failed_count: failed_deletions.len(),
            deleted_ids,
            failed_deletions,
        })
    }

    async fn delete_worktree_tx(
        &self,
        tx: &mut Transaction,
        worktree_id: i32,
    ) -> Result<i64> {
        // Step 1: Remove worktree from chunks.worktree_ids JSONB arrays
        // Uses same pattern as incremental/tree_sha_update.rs::remove_worktree_from_chunks
        let affected = sqlx::query(
            r#"
            UPDATE maproom.chunks
            SET worktree_ids = worktree_ids - $1::TEXT,
                updated_at = NOW()
            WHERE worktree_ids ? $1::TEXT
            "#
        )
        .bind(worktree_id.to_string())
        .execute(&mut **tx)
        .await?
        .rows_affected();

        // Step 2: Garbage collection - delete chunks with empty worktree_ids
        let deleted = sqlx::query(
            r#"
            DELETE FROM maproom.chunks
            WHERE jsonb_array_length(worktree_ids) = 0
            "#
        )
        .execute(&mut **tx)
        .await?
        .rows_affected();

        // Step 3: Delete worktree record
        sqlx::query("DELETE FROM maproom.worktrees WHERE id = $1")
            .bind(worktree_id)
            .execute(&mut **tx)
            .await?;

        Ok(deleted as i64)
    }
}
```

### Why Array-Based Deletion vs CASCADE
From architecture.md (lines 256-270):

**Why CASCADE Doesn't Work:**
1. `DELETE FROM worktrees WHERE id = X` triggers:
2. `files.worktree_id` → SET TO NULL (not deleted)
3. `chunks.file_id` still points to file → chunks NOT deleted
4. `chunks.worktree_ids` still contains X (stale reference!)

**Why Array-Based Deletion Works:**
1. Remove X from all `worktree_ids` arrays: `worktree_ids = worktree_ids - 'X'::TEXT`
2. Garbage collect empty arrays: `DELETE WHERE jsonb_array_length(worktree_ids) = 0`
3. Delete worktree record: `DELETE FROM worktrees WHERE id = X`
4. Multi-worktree chunks preserved (e.g., `worktree_ids = [A, B]` → `[B]` after removing A)

### Relationship to Existing Code
Reuses SQL pattern from `incremental/tree_sha_update.rs::remove_worktree_from_chunks()`:

**Existing Function:** File-level removal (specific relpath)
```sql
UPDATE chunks SET worktree_ids = worktree_ids - $1::TEXT WHERE relpath = $2;
DELETE FROM chunks WHERE jsonb_array_length(worktree_ids) = 0;
```

**This Module:** Worktree-level removal (all chunks)
```sql
UPDATE chunks SET worktree_ids = worktree_ids - $1::TEXT WHERE worktree_ids ? $1::TEXT;
DELETE FROM chunks WHERE jsonb_array_length(worktree_ids) = 0;
```

**Key Difference:** WHERE clause changes from `relpath = $2` (file-level) to `worktree_ids ? $1` (worktree-level)

### Safety Mechanisms
1. **Dry-run default:** CLI requires explicit `--confirm` flag
2. **Transaction rollback:** Failure aborts entire cleanup
3. **Audit trail:** All deletions logged to tracing system
4. **Pre-validation:** Only delete worktrees that failed existence check
5. **Multi-worktree protection:** Shared chunks preserved via array-based deletion

### Testing Requirements
From quality-strategy.md (lines 425-512):

**Scenario 4: Multi-Worktree Chunk Safety**
- Create 2 worktrees sharing a chunk
- Delete one worktree (stale)
- Verify chunk still exists with updated worktree_ids array
- Verify remaining worktree still valid

**Scenario 5: Garbage Collection Accuracy**
- Create stale worktree with chunk belonging ONLY to it
- Delete worktree
- Verify chunk is garbage collected
- Verify report shows chunks_cleaned count

**Additional Tests:**
- Transaction rollback test: Inject error mid-transaction, verify rollback
- Dry-run test: Verify no changes made to database
- Partial failure test: Multiple deletions, one fails, verify others succeed and report accurate

## Dependencies
- **IDXCLEAN-1001** (Stale Detection Module) - Provides `StaleWorktree` struct

## Risk Assessment
- **Risk: Database corruption (partial deletion)**
  - **Mitigation:** Transaction safety ensures all-or-nothing commits

- **Risk: False deletion (delete valid worktree)**
  - **Mitigation:** Validation in IDXCLEAN-1001 ensures only stale worktrees marked for deletion

- **Risk: Cascade failure (multi-worktree chunks deleted)**
  - **Mitigation:** Array-based deletion pattern preserves shared chunks

- **Risk: Performance degradation with large chunk counts**
  - **Mitigation:** GIN index on worktree_ids makes queries fast (~10-50ms for millions of chunks)

## Files/Packages Affected
- `crates/maproom/src/db/cleanup.rs` (extend existing file, ~200-250 lines added)
- Tests: `crates/maproom/tests/cleanup_deletion_tests.rs` (new file)

## Estimated Effort
1-2 days

## Priority
High (core safety mechanism for cleanup operations)
