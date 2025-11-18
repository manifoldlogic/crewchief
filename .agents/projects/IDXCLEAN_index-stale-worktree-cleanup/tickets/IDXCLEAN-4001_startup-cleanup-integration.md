# Ticket: IDXCLEAN-4001: Add Non-Blocking Startup Cleanup to Watch Command

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
- rust-indexer-engineer (primary)
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary
Add non-blocking startup cleanup to the `watch_worktree()` function that runs in the background when the watch command starts. The cleanup is controlled by the `MAPROOM_AUTO_CLEANUP` environment variable and uses the existing `StaleWorktreeDetector` and `WorktreeCleaner` from the `db::cleanup` module.

## Background
The Watch command architecture was analyzed in Phase 3 and determined to be well-suited for cleanup integration with NO REFACTORING required. The existing pool-based database access and background task pattern are perfect for adding a simple startup cleanup task.

This ticket implements Hook Point #1 from the Watch Integration Analysis (architecture.md lines 558-567), which adds a non-blocking background cleanup task after database pool creation but before starting file watchers. This ensures the database is cleaned of stale worktrees at watch startup without delaying the user experience.

**References:**
- `plan.md` Phase 4 - Watch Integration, ticket IDXCLEAN-4001 (lines 537-583)
- `architecture.md` Section 5 - Watch Integration Analysis (lines 508-717), Hook Point #1 (lines 558-567)

## Acceptance Criteria
- [ ] Add `tokio::spawn` for startup cleanup after pool creation (line ~1140 in `crates/maproom/src/indexer/mod.rs`)
- [ ] Cleanup runs in background (non-blocking, startup delay < 200ms)
- [ ] Uses existing `StaleWorktreeDetector` and `WorktreeCleaner` from `db::cleanup` module
- [ ] Controlled by `MAPROOM_AUTO_CLEANUP` environment variable (default: false)
- [ ] Errors logged with `tracing::warn!` but don't break watch startup
- [ ] Cleanup logs with emoji indicators (🧹 starting, ✅ success, ⚠️ failure)
- [ ] Integration test verifies watch starts immediately even if cleanup is running

## Technical Requirements
- Add startup cleanup after pool creation at line ~1140 in `crates/maproom/src/indexer/mod.rs`
- Use `tokio::spawn` for non-blocking background execution
- Parse `MAPROOM_AUTO_CLEANUP` environment variable with default value "false"
- Clone pool for background task: `let pool_cleanup = pool.clone();`
- Use `tracing::info!` for successful cleanup operations
- Use `tracing::warn!` for cleanup errors (non-fatal)
- Use `tracing::debug!` for no-op cases (no stale worktrees found)
- Watch command must start within 200ms even if cleanup takes longer

## Implementation Notes

The implementation adds approximately 30 lines of code after pool creation in `watch_worktree()`. The code structure follows this pattern:

```rust
// In indexer/mod.rs::watch_worktree() after pool creation (line ~1142)

let enable_auto_cleanup = std::env::var("MAPROOM_AUTO_CLEANUP")
    .unwrap_or_else(|_| "false".to_string())
    .parse::<bool>()
    .unwrap_or(false);

if enable_auto_cleanup {
    let pool_cleanup = pool.clone();
    tokio::spawn(async move {
        use crate::db::cleanup::{StaleWorktreeDetector, WorktreeCleaner};

        match StaleWorktreeDetector::new(pool_cleanup.clone()).detect_stale_worktrees().await {
            Ok(stale) if !stale.is_empty() => {
                tracing::info!("🧹 Startup cleanup: found {} stale worktrees", stale.len());
                match WorktreeCleaner::new(pool_cleanup, false).cleanup_stale_worktrees(stale).await {
                    Ok(report) => tracing::info!("✅ Cleanup complete: {} deleted", report.deleted_count),
                    Err(e) => tracing::warn!("⚠️  Cleanup failed: {}", e),
                }
            }
            Ok(_) => tracing::debug!("No stale worktrees found"),
            Err(e) => tracing::warn!("Cleanup detection failed: {}", e),
        }
    });
}
```

**Key Design Points:**
- **Non-blocking:** Uses `tokio::spawn` so watch startup continues immediately
- **Safe:** Clones the pool, cleanup runs independently, errors don't affect watch
- **Observable:** Logs with emoji indicators for easy identification in logs
- **Configurable:** Opt-in via environment variable (default disabled)
- **Minimal:** ~30 lines, no refactoring of existing watch code required

**Testing Considerations:**
- Integration test should verify watch starts quickly (< 200ms) even with cleanup enabled
- Test with stale worktrees present to verify cleanup runs in background
- Test with cleanup disabled to verify no-op behavior
- Test error handling: cleanup failure should log warning but not stop watch

## Dependencies
- **IDXCLEAN-1001** - Stale Worktree Detection (provides `StaleWorktreeDetector`)
- **IDXCLEAN-1002** - Worktree Deletion (provides `WorktreeCleaner`)

Both dependencies must be completed as this ticket uses the `db::cleanup` module components.

## Risk Assessment
- **Risk**: Cleanup could delay watch startup if not properly non-blocking
  - **Mitigation**: Use `tokio::spawn` to ensure cleanup runs in background. Integration test verifies < 200ms startup time.

- **Risk**: Cleanup errors could crash watch command
  - **Mitigation**: All cleanup errors are caught and logged with `tracing::warn!`. Watch continues normally even if cleanup fails.

- **Risk**: Database contention between cleanup and indexing operations
  - **Mitigation**: Cleanup runs once at startup before file watchers start. Pool handles connection management. Cleanup is read-heavy (detection) followed by minimal writes (deletion).

## Files/Packages Affected
- `crates/maproom/src/indexer/mod.rs` - Add startup cleanup spawn (~30 lines after line 1140)
- Integration test file (new or existing) - Add test for non-blocking startup behavior

## Estimated Effort
0.5-1 day

## Priority
Medium (enhancement, not critical)
