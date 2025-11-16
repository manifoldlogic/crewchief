# Ticket: UNIWATCH-1003: Extract and Reuse DebouncedHandler

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

Copy DebouncedHandler from watcher.rs and make it reusable for preventing rapid successive branch switch handling.

## Background

The existing BranchWatcher has a DebouncedHandler to prevent processing rapid successive events (e.g., when git does multiple writes to .git/HEAD). We need this same logic for our unified watcher, but it's currently tightly coupled to BranchWatcher.

This is part of Phase 1 (Foundation) of the UNIWATCH project, providing the debouncing infrastructure needed to handle .git/HEAD file change events gracefully.

## Acceptance Criteria

- [x] DebouncedHandler accessible from watch_worktree() context
- [x] Maintains thread-safe Mutex<Instant> pattern
- [x] Configurable debounce duration (default 2 seconds)
- [x] Unit test `test_debouncer_prevents_rapid_events()` passes
- [x] No clippy warnings

## Technical Requirements

- Location: Either inline in `crates/maproom/src/indexer/mod.rs` OR new `crates/maproom/src/indexer/debounce.rs` module
- Copy DebouncedHandler struct from `src/watcher.rs` lines 91-96
- Copy implementation from lines 98-156
- Make it generic (remove BranchWatcher coupling)
- Approximately 20 lines copied/refactored
- No unsafe code

## Implementation Notes

Reference implementation from watcher.rs:

```rust
struct DebouncedHandler {
    last_event: Mutex<Instant>,
    debounce_duration: Duration,
}

impl DebouncedHandler {
    fn new(debounce_duration: Duration) -> Self {
        Self {
            last_event: Mutex::new(Instant::now() - debounce_duration),
            debounce_duration,
        }
    }

    fn should_handle(&self) -> bool {
        let mut last = self.last_event.lock().unwrap();
        let now = Instant::now();

        if now.duration_since(*last) >= self.debounce_duration {
            *last = now;
            true
        } else {
            false
        }
    }
}
```

**Implementation options:**
- **Option A**: Inline in `mod.rs` (simpler, keeps related code together)
- **Option B**: New module `debounce.rs` (more organized, reusable)

Recommend Option A for this phase since it's only used in one place.

## Dependencies

None (can be done in parallel with other Phase 1 tickets)

## Risk Assessment

- **Risk**: Mutex poisoning if panic occurs while holding lock
  - **Mitigation**: Use same pattern as existing BranchWatcher (proven safe in production)

- **Risk**: Debounce duration too short causes events to be missed
  - **Mitigation**: Use same 2-second default as BranchWatcher, which has been tested

- **Risk**: Debounce duration too long causes lag in branch detection
  - **Mitigation**: 2 seconds is acceptable for user-triggered git checkout operations

## Files/Packages Affected

- **Option A**: `crates/maproom/src/indexer/mod.rs` (approximately 20 new lines)
- **Option B**: `crates/maproom/src/indexer/debounce.rs` (approximately 20 new lines) + mod export in `mod.rs`
