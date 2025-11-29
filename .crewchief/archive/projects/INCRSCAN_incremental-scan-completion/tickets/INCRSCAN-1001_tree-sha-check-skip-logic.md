# Ticket: INCRSCAN-1001: Add Tree SHA Check and Skip Logic to Scan Command

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
Implement tree SHA checking before scan operations to determine if a worktree's code has changed. If unchanged (and not using --force flag), skip the entire scan operation. This enables 10,000x speedup for unchanged worktrees.

## Background
The incremental scanning infrastructure exists (database table, query functions, git functions) but is never used by the scan command. The scan command in main.rs currently always performs full scans. This ticket wires together existing components to add the skip decision logic.

From analysis.md: "The scan command never calls the tree SHA check or state query functions. Every scan processes all files as if it's the first time."

From architecture.md: "Add tree SHA check at scan command level before calling scan_worktree(). Compare current tree SHA against last indexed SHA from worktree_index_state table. If match and not --force: return early."

The Maproom indexer's incremental scanning feature is incomplete. The `worktree_index_state` table is never populated after scans, causing every scan to process all ~474K chunks taking 2-3 hours instead of seconds. This ticket adds tree SHA checking to enable skip logic.

## Acceptance Criteria
- [ ] **Unchanged tree skips scanning:** When tree SHA matches cached SHA, log message "No changes detected (tree SHA match), skipping scan" and return in < 1 second (vs 2-3 hours currently). Early return before file walking.
- [ ] **Changed tree performs full scan:** When tree SHA differs from cached SHA, log message "Tree changed from {last} to {current}" and process all files as before. No behavior change from current.
- [ ] **Force flag overrides skip:** --force always performs full scan even if tree SHA unchanged. Log message: "Force flag enabled, performing full scan".
- [ ] **First-time scans work:** When no cached state exists (last_sha = "init"), perform full scan with log message "First-time indexing detected".
- [ ] **Error handling is safe:** Git errors → full scan + warning logged. DB errors → full scan + warning logged. Never skip incorrectly on error.

## Technical Requirements

### 1. Git Tree SHA Retrieval
- Call existing `get_git_tree_sha(&path)` function before scan
- Function exists in `crates/maproom/src/git/mod.rs`
- Returns 40-character hex string
- Handle errors gracefully (fallback to full scan)

### 2. Database State Query
- Get worktree ID from (repo, worktree) names
- Query `worktree_index_state` table for `last_tree_sha`
- Use existing `get_last_indexed_tree(client, worktree_id)` function
- Returns "init" if never indexed (first-time case)

### 3. Skip Decision Logic
- Compare current_sha == last_sha
- If true AND not --force: skip scan (early return)
- If false OR --force: proceed with full scan
- Log decision clearly for debugging

### 4. Error Handling (Fail-Safe)
- Git command fails → log warning, proceed with full scan
- Database query fails → log warning, proceed with full scan
- Any doubt → full scan (never skip incorrectly)

### 5. Integration Points
- Insert after line 572 in main.rs (after logging scan mode)
- Before calling `scan_worktree()` or `scan_worktree_parallel()`
- Works for both sequential and parallel scan modes

## Implementation Notes

### Insertion Point

**IMPORTANT: Database Connection Strategy**

The tree SHA check must happen BEFORE the parallel/sequential scan mode decision because we need database access. The recommended approach is:

1. Create database client connection BEFORE tree SHA check (around line 573)
2. Perform tree SHA check and skip decision using this client
3. If skipping, return early
4. If not skipping, create pool (for parallel) or reuse client (for sequential)

Insert the following logic after line 583 in main.rs (after logging scan mode), BEFORE the parallel/sequential mode decision:

```rust
// Create database connection for tree SHA check
// This must happen before parallel/sequential decision so we can skip if needed
let client = db::connect().await?;

// Get git tree SHA using existing function from git.rs
let tree_sha = match crate::git::get_git_tree_sha(&path) {
    Ok(sha) => {
        tracing::info!("Current tree SHA: {}", sha);
        Some(sha)
    }
    Err(e) => {
        tracing::warn!("Could not get tree SHA: {}, proceeding with full scan", e);
        None
    }
};

// Query worktree_index_state if we have tree SHA
if let Some(ref current_sha) = tree_sha {
    // Get repo and worktree IDs using EXISTING functions from db/queries.rs
    // Note: Using get_or_create functions ensures worktrees are created if they don't exist
    let root_abs = path.canonicalize().context("invalid root path")?;
    let repo_id = match crate::db::get_or_create_repo(
        &client,
        &repo,
        root_abs.to_string_lossy().as_ref()
    ).await {
        Ok(id) => Some(id),
        Err(e) => {
            tracing::warn!("Could not get repo ID: {}, proceeding with full scan", e);
            None
        }
    };

    if let Some(repo_id) = repo_id {
        let worktree_id = match crate::db::get_or_create_worktree(
            &client,
            repo_id,
            &worktree,
            root_abs.to_string_lossy().as_ref()
        ).await {
            Ok(id) => Some(id),
            Err(e) => {
                tracing::warn!("Could not get worktree ID: {}, proceeding with full scan", e);
                None
            }
        };

        if let Some(wt_id) = worktree_id {
            // Get last indexed tree SHA using existing function from db/index_state.rs
            match crate::db::get_last_indexed_tree(&client, wt_id).await {
                Ok(last_sha) if last_sha == *current_sha && !force => {
                    println!("✓ No changes detected (tree SHA match), skipping scan");
                    tracing::info!("Scan skipped: tree {} already indexed", current_sha);
                    return Ok(());  // Early return!
                }
                Ok(last_sha) if last_sha != "init" => {
                    tracing::info!("Tree changed: {} -> {}", last_sha, current_sha);
                }
                Ok(_) => {
                    tracing::info!("First-time indexing (no cached state)");
                }
                Err(e) => {
                    tracing::warn!("Could not query index state: {}, proceeding with full scan", e);
                }
            }
        }
    }
}

// [Now handle parallel vs sequential mode with existing code]
// For parallel: create pool as before
// For sequential: reuse the 'client' we already created above
```

### Sequential vs Parallel Mode Handling

After the tree SHA check, handle the two scan modes:

**For Sequential Mode:**
```rust
// Reuse the client we created for tree SHA check
indexer::scan_worktree(
    &client,  // Reuse existing client
    &repo,
    &worktree,
    &path,
    &commit,
    concurrency,
    languages,
    exclude,
    Some(&progress),
).await?;
```

**For Parallel Mode:**
```rust
// Create connection pool for parallel processing
let pool = db::create_pool().await?;
indexer::scan_worktree_parallel(
    &pool,  // Use new pool
    &repo,
    &worktree,
    &path,
    &commit,
    languages,
    exclude,
    config,
    Some(&progress),
).await?;
```

### Additional Notes
- The `get_git_tree_sha` function already exists in `crates/maproom/src/git.rs` and is public
- The `get_or_create_repo` and `get_or_create_worktree` functions exist in `crates/maproom/src/db/queries.rs`
- The `get_last_indexed_tree` function exists in `crates/maproom/src/db/index_state.rs`
- NO NEW HELPER FUNCTIONS NEEDED - use existing, tested functions
- Ensure all error paths default to full scan (fail-safe behavior)
- Test with both unchanged and changed repositories
- Verify logging output is clear and actionable
- The `tree_sha` variable must remain in scope for INCRSCAN-1002 to use

## Dependencies
- None (first ticket in sequence)

## Risk Assessment
- **Risk**: Git command fails in unusual repos (bare, corrupt)
  - **Mitigation**: Wrap in Result, fallback to full scan on error
- **Risk**: Database query slow
  - **Mitigation**: Queries are indexed, should be <5ms
- **Risk**: Logic error causes false skip
  - **Mitigation**: Extensive testing in INCRSCAN-1003

## Files/Packages Affected
- `crates/maproom/src/main.rs` - Add skip logic to scan command handler (lines 593-673)
- `crates/maproom/src/progress.rs` - Add getter methods for files_processed() and chunks_processed() (CRITICAL-2 fix, prerequisite for INCRSCAN-1002)
- `crates/maproom/src/git/mod.rs` - Uses existing `get_git_tree_sha` (no changes required - already public)

## Testing Strategy
- Unit tests for skip decision logic
- Integration test with unchanged repo (INCRSCAN-1003)
- Manual validation with genetic optimizer (INCRSCAN-1005)

## Additional Metadata
- **Priority**: P0 (blocks all other tickets)
- **Complexity**: Medium
- **Estimated Time**: 2-3 hours
- **Phase**: 1 (Core Implementation)
