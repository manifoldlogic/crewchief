# Ticket: SQLIMPL-3004: Implement Tree SHA Update

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - 82 incremental tests passing
- [x] **Verified** - by the verify-ticket agent

## Agents
- rust-indexer-engineer
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary
Implement tree SHA update functionality for worktree tracking and coordinated incremental re-indexing. This ties together the change detection and processing into a cohesive incremental update flow.

## Background
The tree SHA updater at `src/incremental/tree_sha_update.rs` has 2 stubbed methods for worktree management. These coordinate the overall incremental update process.

This ticket implements Plan Phase 3, Ticket 3004: "Implement Tree SHA Update".

## Acceptance Criteria
- [x] `remove_worktree_from_chunks()` cleans up `chunk_worktrees` entries
- [x] `incremental_update()` coordinates the full incremental re-index flow
- [x] Worktree cleanup works correctly when worktrees are removed
- [x] Incremental update orchestrates tree SHA comparison → git diff-tree → file processing
- [x] UpdateStats tracks files processed, chunks processed, embeddings generated

## Technical Requirements
- Clean up `chunk_worktrees` table when worktrees are removed
- Coordinate the incremental pipeline:
  1. Detect changes (detector.rs)
  2. Process changes (processor.rs)
  3. Update edges (edge_updater.rs)
- Transaction support for atomic updates
- Handle partial failures gracefully

## Implementation Notes

### Current Stubs (to implement)
```rust
// src/incremental/tree_sha_update.rs:129
// remove_worktree_from_chunks() - stub

// src/incremental/tree_sha_update.rs:188
// incremental_update() - stub
```

### Schema Reference
```sql
CREATE TABLE chunk_worktrees (
    chunk_id INTEGER REFERENCES chunks(id),
    worktree_id INTEGER REFERENCES worktrees(id),
    PRIMARY KEY (chunk_id, worktree_id)
);
```

### Target Implementation Patterns

#### Remove Worktree from Chunks
```rust
pub async fn remove_worktree_from_chunks(&self, worktree_id: i64) -> Result<()> {
    self.store.run(move |conn| {
        // Remove chunk_worktrees entries
        conn.execute(
            "DELETE FROM chunk_worktrees WHERE worktree_id = ?",
            [worktree_id]
        )?;

        // Optionally: remove orphaned chunks (chunks with no worktrees)
        conn.execute(
            "DELETE FROM chunks WHERE id NOT IN (SELECT chunk_id FROM chunk_worktrees)",
            []
        )?;

        Ok(())
    }).await
}
```

#### Incremental Update
```rust
pub async fn incremental_update(&self, repo_id: i64, worktree_id: i64) -> Result<UpdateStats> {
    let mut stats = UpdateStats::default();

    // 1. Detect changes
    let changes = self.detector.detect_changes(repo_id, worktree_id).await?;
    stats.files_checked = changes.total_files;

    // 2. Process each type of change
    for path in changes.added {
        self.processor.index_new_file(&path, repo_id, worktree_id).await?;
        stats.files_added += 1;
    }

    for (path, file_id) in changes.modified {
        self.processor.update_file(&path, file_id).await?;
        stats.files_modified += 1;
    }

    for file_id in changes.removed {
        self.processor.remove_file(file_id).await?;
        stats.files_removed += 1;
    }

    // 3. Update edges for all affected chunks
    let affected_chunks = changes.get_affected_chunk_ids();
    self.edge_updater.update_edges(&affected_chunks).await?;
    stats.edges_updated = affected_chunks.len();

    Ok(stats)
}

#[derive(Default)]
pub struct UpdateStats {
    pub files_checked: usize,
    pub files_added: usize,
    pub files_modified: usize,
    pub files_removed: usize,
    pub edges_updated: usize,
}
```

### Phase 3 Gate Verification
After implementation, verify:
```bash
# Index a repo
cargo run -p crewchief-maproom -- scan --path ./test-repo

# Modify a file
echo "// new comment" >> ./test-repo/src/main.rs

# Run incremental update
cargo run -p crewchief-maproom -- upsert --paths ./test-repo/src/main.rs

# Verify database was updated (search should find new content)
cargo run -p crewchief-maproom -- search "new comment"
```

## Dependencies
- SQLIMPL-3001 (Change Detector)
- SQLIMPL-3002 (Processor)

## Risk Assessment
- **Risk**: Partial update leaves database inconsistent
  - **Mitigation**: Wrap in transaction; rollback on failure
- **Risk**: Large changesets may be slow
  - **Mitigation**: Process in batches; show progress

## Files/Packages Affected
- `crates/maproom/src/incremental/tree_sha_update.rs` (primary)
