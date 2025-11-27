# Ticket: IDXCLEAN-4003: Document Configuration and Test Watch Cleanup Integration

## Status
- [ ] **Task completed** - **BLOCKED** - watch command not functional
- [ ] **Tests pass** - N/A
- [ ] **Verified** - by the verify-ticket agent

### BLOCKER (2025-11-27)

Same blocker as IDXCLEAN-4001 and 4002: The watch command is not functional (removed in IDXABS-2001). Cannot document or test watch cleanup integration until watch is reimplemented.

**Resolution:** DEFERRED until watch command is reimplemented (IDXABS-2006). Phase 4 is [Optional Enhancement].

## Agents
- rust-indexer-engineer
- integration-tester (test review)
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary
Document configuration options for automatic cleanup during watch mode and add comprehensive integration tests to verify watch cleanup functionality works correctly in all scenarios including startup, periodic execution, rate limiting, and queue state awareness.

## Background
With startup cleanup (IDXCLEAN-4001) and periodic cleanup (IDXCLEAN-4002) implemented, users need clear documentation on how to enable and configure automatic cleanup behavior. Additionally, comprehensive integration tests are required to ensure the watch cleanup feature works correctly across all scenarios: background execution, rate limiting, queue idle detection, and configuration toggling.

This ticket implements Phase 4 - Watch Integration, specifically ticket IDXCLEAN-4003 from plan.md (lines 646-685).

## Acceptance Criteria
- [ ] Environment variable `MAPROOM_AUTO_CLEANUP` documented in README.md with clear examples
- [ ] Example usage and troubleshooting tips in crates/maproom/CLAUDE.md
- [ ] Integration test: Watch startup cleanup runs in background without blocking
- [ ] Integration test: Periodic cleanup respects rate limiting (30-minute intervals)
- [ ] Integration test: Cleanup skips execution when indexer queue is busy
- [ ] Integration test: `MAPROOM_AUTO_CLEANUP=false` disables cleanup functionality
- [ ] Integration test: `MAPROOM_AUTO_CLEANUP=true` enables cleanup functionality
- [ ] Performance test: Watch startup delay < 200ms with cleanup enabled

## Technical Requirements
- Documentation must be clear, user-friendly, and include practical examples
- Include troubleshooting section explaining how to verify cleanup is running
- Tests must verify non-blocking behavior (watch starts immediately)
- Tests must verify rate limiting prevents excessive cleanup operations
- Tests must verify queue idle detection works correctly
- Tests must verify configuration flag controls cleanup behavior
- Use tokio::time::advance for time-based tests if using pause/resume time
- All tests must be deterministic and not rely on actual wall-clock time

## Implementation Notes

### Documentation Example (from plan.md lines 660-675)

Add to README.md:
```md
## Auto-Cleanup Configuration

Enable automatic cleanup during `maproom watch`:

```bash
export MAPROOM_AUTO_CLEANUP=true
maproom watch
```

Behavior:
- Runs quick cleanup on watch startup (non-blocking)
- Periodic cleanup every 30 minutes (only when indexer idle)
- Rate limited to prevent excessive operations

Troubleshooting:
- Check logs for "Running stale worktree cleanup" messages
- Verify environment variable is set: `echo $MAPROOM_AUTO_CLEANUP`
- Cleanup only runs when indexer queue is idle
```

### Test Structure

Create `crates/maproom/tests/watch_cleanup_test.rs` with:
1. Startup test: Verify cleanup spawns without blocking watch initialization
2. Rate limiting test: Verify 30-minute interval between cleanups
3. Queue state test: Verify cleanup skips when queue has pending operations
4. Config toggle tests: Verify MAPROOM_AUTO_CLEANUP flag behavior
5. Performance test: Measure watch startup time with cleanup enabled

### Test Patterns

Use mocked time or tokio test utilities:
```rust
#[tokio::test]
async fn test_startup_cleanup_non_blocking() {
    // Verify watch starts in < 200ms even with cleanup enabled
}

#[tokio::test]
async fn test_periodic_cleanup_rate_limiting() {
    // Use tokio::time::advance to test 30-minute intervals
}
```

## Dependencies
- IDXCLEAN-4002 (periodic cleanup implementation must be complete)
- IDXCLEAN-4001 (startup cleanup implementation must be complete)

## Risk Assessment
- **Risk**: Integration tests may be flaky if they rely on actual timing
  - **Mitigation**: Use tokio test time manipulation (pause/resume/advance) for deterministic tests

- **Risk**: Documentation may not be clear enough for users unfamiliar with environment variables
  - **Mitigation**: Include concrete examples and troubleshooting steps

- **Risk**: Performance test may fail on slower CI machines
  - **Mitigation**: Use generous threshold (200ms) and focus on "non-blocking" rather than absolute speed

## Files/Packages Affected
- `README.md` - Add auto-cleanup configuration section with examples
- `crates/maproom/CLAUDE.md` - Add watch cleanup usage notes and troubleshooting
- `crates/maproom/tests/watch_cleanup_test.rs` - New file with integration tests

## Planning References
- `.agents/projects/IDXCLEAN_index-stale-worktree-cleanup/planning/plan.md` (lines 646-685)

## Estimated Effort
0.5-1 day

## Priority
Medium (completes Phase 4 watch integration)
