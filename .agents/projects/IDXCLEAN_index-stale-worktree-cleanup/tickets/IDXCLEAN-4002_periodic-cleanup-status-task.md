# Ticket: IDXCLEAN-4001: Extend Status Task with Periodic Cleanup Checks

## Status
- [ ] **Task completed** - **BLOCKED** - watch command not functional
- [ ] **Tests pass** - N/A
- [ ] **Verified** - by the verify-ticket agent

### BLOCKER (2025-11-27)

Same blocker as IDXCLEAN-4001: The `watch_worktree()` function and status_task loop were **removed** in IDXABS-2001 (SQLite-only migration). The watch command currently returns an error.

**Resolution:** DEFERRED until watch command is reimplemented (IDXABS-2006). Phase 4 is [Optional Enhancement].

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
Extend the existing status_task loop (line ~1432 in indexer/mod.rs) to perform periodic cleanup checks every 30 minutes when the indexer queue is idle, based on findings from Watch integration analysis.

## Background
The Watch integration analysis (architecture.md Section 5) revealed that the status_task loop already runs every 10 seconds and has access to queue stats, making it the ideal hook point for periodic cleanup. This ticket implements Hook Point #2 (Status Task Extension) using Option A from the recommended integration approach.

This approach requires no refactoring of existing watch code and enables automated cleanup during long-running watch sessions without blocking indexing operations.

**References:**
- Plan: Phase 4 - Watch Integration, ticket IDXCLEAN-4002 (lines 587-642)
- Architecture: Section 5 - Watch Integration Analysis, Hook Point #2 (lines 569-578)
- Architecture: Option A implementation (lines 625-657)

## Acceptance Criteria
- [ ] Extend status_task loop (line ~1432 in indexer/mod.rs) with cleanup check
- [ ] Cleanup runs every 30 minutes (configurable interval)
- [ ] Rate limiting: skip if cleanup ran in last 15 minutes
- [ ] Queue idle detection: only run if `stats.pending == 0 && stats.processing == 0`
- [ ] Cleanup spawned as tokio::spawn (non-blocking)
- [ ] Track last_cleanup timestamp (Option<Instant>)
- [ ] Controlled by same MAPROOM_AUTO_CLEANUP env variable
- [ ] Integration test: cleanup defers when indexer busy

## Technical Requirements
- Extend status_task loop at line ~1432 in `crates/maproom/src/indexer/mod.rs`
- Add `let mut last_cleanup: Option<Instant> = None;` before loop
- Check `stats.pending == 0 && stats.processing == 0` before spawning cleanup
- Use `Instant::now()` to track last cleanup time
- Skip if `last_cleanup.elapsed() < Duration::from_secs(900)` (15 min rate limit)
- Cleanup interval: `Duration::from_secs(30 * 60)` (30 minutes)
- Read `MAPROOM_AUTO_CLEANUP` env variable to enable/disable feature
- Spawn cleanup as non-blocking tokio task to avoid delaying status updates
- Use same cleanup logic pattern as startup cleanup (from IDXCLEAN-4001)

## Implementation Notes

**Location:** Extend status_task at line ~1432 in `crates/maproom/src/indexer/mod.rs`

**Implementation pattern (from architecture.md lines 626-657):**

```rust
// In status_task (around line 1432)
let mut interval = tokio::time::interval(Duration::from_secs(10));
let mut last_cleanup: Option<Instant> = None;
let cleanup_interval = Duration::from_secs(30 * 60); // 30 minutes

let enable_auto_cleanup = std::env::var("MAPROOM_AUTO_CLEANUP")
    .unwrap_or_else(|_| "false".to_string())
    .parse::<bool>()
    .unwrap_or(false);

loop {
    interval.tick().await;
    let stats = queue_clone.lock().await.stats();

    // ... existing status logging ...

    // Periodic cleanup check (if enabled)
    if enable_auto_cleanup {
        let should_cleanup = match last_cleanup {
            None => false, // Don't run on first check (startup already did)
            Some(instant) => instant.elapsed() > cleanup_interval,
        };

        // Only cleanup if queue idle and enough time passed
        if should_cleanup && stats.pending == 0 && stats.processing == 0 {
            let pool_cleanup = pool_clone.clone();
            tokio::spawn(async move {
                // Run cleanup (same logic as startup)
                use crewchief_maproom::db::cleanup::{StaleWorktreeDetector, WorktreeCleaner};

                match StaleWorktreeDetector::new(pool_cleanup.clone()).detect_stale_worktrees().await {
                    Ok(stale) if !stale.is_empty() => {
                        tracing::info!("🧹 Periodic cleanup: found {} stale worktrees", stale.len());
                        match WorktreeCleaner::new(pool_cleanup, false).cleanup_stale_worktrees(stale).await {
                            Ok(report) => tracing::info!("✅ Periodic cleanup complete: {} deleted", report.deleted_count),
                            Err(e) => tracing::warn!("⚠️  Periodic cleanup failed: {}", e),
                        }
                    }
                    Ok(_) => {} // No stale worktrees
                    Err(e) => tracing::warn!("Periodic cleanup detection failed: {}", e),
                }
            });
            last_cleanup = Some(Instant::now());
        }
    }
}
```

**Key Design Decisions:**

1. **Queue Idle Detection**: Only run when `stats.pending == 0 && stats.processing == 0` to avoid competing with active indexing
2. **Rate Limiting**: Track last cleanup time to prevent running too frequently
3. **Non-blocking**: Spawn as tokio task to avoid delaying status updates
4. **First Check Skip**: Don't run on first status check (None case) since startup cleanup already ran
5. **Configurable**: Use same MAPROOM_AUTO_CLEANUP env variable for consistency

**Testing Considerations:**

Create integration test that:
1. Starts watch with MAPROOM_AUTO_CLEANUP=true
2. Queues multiple indexing tasks
3. Verifies cleanup does NOT run while queue busy (stats.pending > 0)
4. Waits for queue to drain
5. Advances time (if possible) or waits 30 minutes
6. Verifies cleanup runs when queue idle

## Dependencies
- IDXCLEAN-1001 (stale detection module - cleanup pattern established)
- IDXCLEAN-1002 (safe deletion module - cleanup implementation)
- IDXCLEAN-3004 (manual validation - startup cleanup tested)

Note: This ticket assumes IDXCLEAN-4001 (startup cleanup) has established the cleanup integration pattern. If ticket numbering differs, adjust dependency accordingly.

## Risk Assessment
- **Risk**: Periodic cleanup might run during critical indexing operations
  - **Mitigation**: Queue idle detection (`stats.pending == 0 && stats.processing == 0`) ensures cleanup only runs when indexer idle

- **Risk**: Cleanup might run too frequently, causing performance degradation
  - **Mitigation**: Rate limiting (15 minute minimum between cleanups) and 30-minute interval prevents excessive execution

- **Risk**: Spawned cleanup task might fail silently
  - **Mitigation**: Comprehensive tracing/logging in cleanup task, errors logged as warnings

- **Risk**: Integration might break existing status_task behavior
  - **Mitigation**: Cleanup check is additive, only runs when feature enabled, non-blocking spawn ensures status updates continue

## Files/Packages Affected
- `crates/maproom/src/indexer/mod.rs` - Extend status_task loop (~20 lines added)
- Integration test file (new or extend existing watch tests) - Test cleanup deferral when busy

**Estimated Lines of Code:** ~30-40 lines (20 in mod.rs, 10-20 in tests)
