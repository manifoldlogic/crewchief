# Ticket: INCRSCAN-1002: Add State Persistence After Scan Completion

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - tests executed and passing (or N/A if no tests)
- [x] **Verified** - by the verify-ticket agent

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
Call `update_index_state()` after scan operations complete successfully to save the git tree SHA and scan statistics to the `worktree_index_state` table. This enables the skip logic from INCRSCAN-1001 to work on subsequent scans of the same worktree.

## Background
The `worktree_index_state` table and `update_index_state()` function exist (from migration 0020 and BRANCHX-1005), but are never called during scan operations. After implementing skip logic in INCRSCAN-1001, we need to save state so future scans can check if code changed.

From planning/analysis.md: "Lines 316-320 in tree_sha_update.rs show the code to update state is commented out with TODO BRANCHX-1008. The scan command never populates worktree_index_state."

From planning/architecture.md: "After scan completes, collect statistics (files/chunks processed) and call update_index_state() with new tree SHA. Handle errors gracefully (non-fatal - scan succeeded even if state update fails)."

This ticket implements the state persistence component of the incremental scan completion feature, ensuring that successful scans are recorded for future optimization.

## Acceptance Criteria
- [x] State saved after successful scan - `worktree_index_state` table populated with record including worktree_id, last_tree_sha, last_indexed timestamp
- [x] Correct tree SHA stored - matches the tree SHA retrieved before scan, 40-character hex format verified, enables skip logic on next scan
- [x] Stats accurately tracked - files processed count matches actual files scanned, chunks processed matches database inserts, embeddings generated tracked (if embedding step ran)
- [x] Update errors non-fatal - scan returns success even if state update fails, warning logged with error details, user informed they can continue
- [x] Works for all scan modes - sequential scan updates state, parallel scan updates state, force scan updates state (overwrites with same SHA), skipped scan (from INCRSCAN-1001) requires no update

## Technical Requirements

### 1. Stat Collection
- Track files processed during scan
- Track chunks processed during scan
- Track embeddings generated (if applicable)
- Get final counts from ProgressTracker or track separately

### 2. Worktree ID Retrieval
- Get worktree_id using same logic as INCRSCAN-1001
- Reuse `get_or_create_worktree_id()` helper
- Handle case where worktree created during scan

### 3. State Update Call
- Use existing `update_index_state(client, worktree_id, tree_sha, stats)`
- Function exists in `crates/maproom/src/db/index_state.rs`
- Uses ON CONFLICT DO UPDATE (upsert pattern)
- Atomic and safe for concurrent scans

### 4. Error Handling (Non-Fatal)
- State update errors should NOT fail the scan
- Log warning if update fails
- Scan results are still valid
- Next scan will be slower but still correct

### 5. Integration Points
- Insert after line 635 in main.rs (after scan_worktree completes)
- After embedding generation if enabled
- Works for both sequential and parallel scan modes
- Must have tree_sha from INCRSCAN-1001

## Implementation Notes

### Insertion Point

**IMPORTANT: Database Connection and Stats Collection**

The state persistence code must handle both sequential and parallel scan modes, which use different database connection types. The scan functions return `Result<()>`, so we must use ProgressTracker getter methods (added in fix for CRITICAL-2) to collect statistics.

**Strategy:**
1. Collect stats from ProgressTracker using newly added getter methods
2. For sequential mode: reuse `client` from INCRSCAN-1001
3. For parallel mode: get client from pool
4. Update state AFTER embedding generation (single update, includes embedding count)

Insert the following logic AFTER line 653 in main.rs (after embedding generation completes):

```rust
// State persistence (INCRSCAN-1002)
// Collect statistics from ProgressTracker using getter methods
// Note: ProgressTracker getter methods added to fix CRITICAL-2
let files_processed = progress.files_processed() as i32;
let chunks_processed = progress.chunks_processed() as i32;

// Get embedding count if embeddings were generated
let embeddings_generated = if generate_embeddings {
    chunks_processed  // All chunks got embeddings
} else {
    0
};

let scan_stats = crate::db::UpdateStats {
    files_processed,
    chunks_processed,
    embeddings_generated,
};

// Update index state if we have tree SHA from INCRSCAN-1001
// Note: tree_sha variable comes from INCRSCAN-1001 skip logic
if let Some(ref tree_sha) = tree_sha {
    // Get database client based on scan mode
    // For sequential: reuse client from skip check
    // For parallel: get client from pool (need to have pool in scope)
    let state_client = if parallel {
        // Get client from pool for parallel mode
        // Note: pool variable should be accessible here
        match pool.get().await {
            Ok(c) => Some(c),
            Err(e) => {
                tracing::warn!("Could not get client from pool for state update: {}", e);
                None
            }
        }
    } else {
        // Reuse client from sequential scan
        Some(&client)
    };

    if let Some(state_client) = state_client {
        // Get worktree ID using existing functions (same as INCRSCAN-1001)
        let root_abs = path.canonicalize().context("invalid root path")?;
        let repo_id = crate::db::get_or_create_repo(
            state_client,
            &repo,
            root_abs.to_string_lossy().as_ref()
        ).await.ok();

        if let Some(repo_id) = repo_id {
            let worktree_id = crate::db::get_or_create_worktree(
                state_client,
                repo_id,
                &worktree,
                root_abs.to_string_lossy().as_ref()
            ).await.ok();

            if let Some(wt_id) = worktree_id {
                match crate::db::update_index_state(state_client, wt_id, tree_sha, &scan_stats).await {
                    Ok(_) => {
                        tracing::info!("✓ Updated index state: tree {} ({} files, {} chunks, {} embeddings)",
                            tree_sha, files_processed, chunks_processed, embeddings_generated);
                    }
                    Err(e) => {
                        tracing::warn!("Failed to update index state: {}", e);
                        tracing::warn!("Scan completed successfully, but next scan may be slower");
                        // Don't fail the scan - state update is advisory only
                    }
                }
            }
        }
    }
}
```

### Alternative Simpler Approach

If managing the pool/client reference is complex, you can create a fresh connection for state update:

```rust
// Simpler approach: create fresh connection for state update
if let Some(ref tree_sha) = tree_sha {
    // Create new connection just for state update
    match db::connect().await {
        Ok(state_client) => {
            let root_abs = path.canonicalize().context("invalid root path")?;
            let repo_id = crate::db::get_or_create_repo(
                &state_client,
                &repo,
                root_abs.to_string_lossy().as_ref()
            ).await.ok();

            if let Some(repo_id) = repo_id {
                let worktree_id = crate::db::get_or_create_worktree(
                    &state_client,
                    repo_id,
                    &worktree,
                    root_abs.to_string_lossy().as_ref()
                ).await.ok();

                if let Some(wt_id) = worktree_id {
                    let _ = crate::db::update_index_state(&state_client, wt_id, tree_sha, &scan_stats).await;
                }
            }
        }
        Err(e) => {
            tracing::warn!("Could not create connection for state update: {}", e);
        }
    }
}
```

**Note:** The simpler approach creates an extra database connection but avoids complexity of managing pool/client references. Choose based on your preference.

### Key Design Decisions
1. **Non-fatal errors**: State update failures don't fail the scan because index data is already committed
2. **Single update after embeddings**: Simplified from two-stage approach - update once with all stats including embeddings
3. **ProgressTracker getters**: Uses newly added `files_processed()` and `chunks_processed()` getter methods (CRITICAL-2 fix)
4. **Idempotent**: Force scans can safely update state with same SHA
5. **Database connection flexibility**: Provides both approaches (reuse connection or create fresh) - implementer can choose

## Dependencies
- **INCRSCAN-1001** (tree-sha-check-skip-logic) - Must have tree_sha variable from skip logic implementation

## Risk Assessment
- **Risk**: Progress tracker doesn't expose counts
  - **Mitigation**: Add accessor methods or track separately in main.rs
- **Risk**: Worktree ID changes during scan
  - **Mitigation**: Get ID after scan when guaranteed to exist
- **Risk**: Concurrent scans race on state update
  - **Mitigation**: ON CONFLICT DO UPDATE handles this (last writer wins, which is fine)
- **Risk**: State update fails but scan succeeded
  - **Mitigation**: Non-fatal warning, next scan will still work (just slower)

## Files/Packages Affected
- `crates/maproom/src/main.rs` - Add state update after scan completion (after line 635, after line 653 for embeddings)
- `crates/maproom/src/progress/mod.rs` - Add accessor methods if needed (optional)

## Testing Strategy
- Integration test verifies state persistence (covered in INCRSCAN-1003)
- Manual verification: Check database after scan for state record
- Test concurrent scans (covered in INCRSCAN-1003)
- Error handling test for state update failure (covered in INCRSCAN-1004)

## Metadata
- **Priority**: P0 (enables skip logic to persist)
- **Complexity**: Low
- **Estimated Time**: 1-2 hours
- **Phase**: 1 (Core Implementation)
