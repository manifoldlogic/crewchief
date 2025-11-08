# Ticket: BRANCHX-1009: Handle file deletions in incremental updates

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - related tests pass
- [ ] **Verified** - by the verify-ticket agent

## Agents
- rust-indexer-engineer
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary
Remove worktree from chunks when files are deleted, and optionally clean up orphan chunks with no worktrees.

## Background
This is Phase 3, Step 3.3 of BRANCHX. When git diff-tree detects deleted files (FileStatus::Deleted), we need to remove the current worktree from those chunks' worktree_ids arrays. If a chunk has no worktrees remaining, it should be deleted (garbage collection).

This completes the incremental update logic: Add/Modify handled in BRANCHX-1008, Delete handled here.

Reference: `.agents/projects/BRANCHX_branch-aware-indexing/planning/plan.md` - Phase 3.3

## Acceptance Criteria
- [ ] `remove_worktree_from_chunks(pool, worktree_id, file_path)` function implemented
- [ ] Function removes worktree_id from worktree_ids array for all chunks in file
- [ ] Optional: Delete chunks with empty worktree_ids array (garbage collection)
- [ ] Incremental update algorithm calls this for FileStatus::Deleted
- [ ] Unit test verifies worktree removed from array
- [ ] Integration test verifies deleted file reduces chunk count

## Technical Requirements
- Use JSONB `-` operator to remove element from array
- Query: `UPDATE chunks SET worktree_ids = worktree_ids - $worktree_id::TEXT WHERE file_path = $file_path`
- Optional cleanup: `DELETE FROM chunks WHERE jsonb_array_length(worktree_ids) = 0`
- Handle case where file_path doesn't exist (no-op)
- Return number of chunks affected (for metrics)

## Implementation Notes

Add to `crates/maproom/src/incremental.rs`:

```rust
pub async fn remove_worktree_from_chunks(
    pool: &PgPool,
    worktree_id: i32,
    file_path: &Path,
) -> Result<i32> {
    let affected = sqlx::query!(
        r#"
        UPDATE chunks
        SET worktree_ids = worktree_ids - $1::TEXT
        WHERE file_path = $2
        "#,
        worktree_id.to_string(),
        file_path.to_str().unwrap(),
    )
    .execute(pool)
    .await?
    .rows_affected();

    // Optional: Clean up chunks with no worktrees
    let deleted = sqlx::query!(
        "DELETE FROM chunks WHERE jsonb_array_length(worktree_ids) = 0"
    )
    .execute(pool)
    .await?
    .rows_affected();

    info!("Removed worktree from {} chunks, deleted {} orphan chunks", affected, deleted);

    Ok(affected as i32)
}
```

Update incremental_update in BRANCHX-1007 to handle deletions:
```rust
FileStatus::Deleted => {
    remove_worktree_from_chunks(pool, worktree_id, &file.path).await?;
    stats.files_processed += 1;
}
```

See `architecture.md` section "Remove Worktree from Chunks" for design.

## Dependencies
- BRANCHX-1007 complete (incremental update algorithm)
- BRANCHX-1008 complete (upsert with worktree tracking)

## Risk Assessment
- **Risk**: Accidental deletion of chunks still used in other worktrees
  - **Mitigation**: Only delete if worktree_ids array empty (tested)
- **Risk**: Orphan chunks accumulate (memory leak)
  - **Mitigation**: Garbage collection deletes empty worktree_ids
- **Risk**: File deletion doesn't trigger removal (diff-tree missed)
  - **Mitigation**: Test git diff-tree detection (BRANCHX-1006)

## Files/Packages Affected
- `crates/maproom/src/incremental.rs` (add function)
- Update BRANCHX-1007 code to call this for deletions

## Planning References
- `.agents/projects/BRANCHX_branch-aware-indexing/planning/plan.md` - Phase 3.3 (lines 215-241)
- `.agents/projects/BRANCHX_branch-aware-indexing/planning/architecture.md` - Remove Worktree from Chunks (lines 258-288)
