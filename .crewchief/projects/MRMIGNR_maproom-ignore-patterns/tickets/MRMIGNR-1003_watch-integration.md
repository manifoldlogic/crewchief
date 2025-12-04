# Ticket: [MRMIGNR-1003]: Watch Integration with .maproomignore

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
- rust-indexer-engineer
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary
Integrate .maproomignore pattern filtering into watch operation by adding event filter in `event_conversion_task()` to skip FileEvents matching ignore patterns.

## Background
The git poller emits FileEvents for all tracked file changes (respecting .gitignore automatically). However, users need additional control to exclude files from indexing without adding them to .gitignore. This ticket adds post-filtering of FileEvents based on .maproomignore patterns before they're converted to IndexingEvents.

Pattern loading happens once at watcher startup. If .maproomignore changes during watch operation, watcher restart is required (hot-reload not supported in MVP).

Reference: Phase 1 - Watch Integration (plan.md lines 129-137), Architecture Component 3 (architecture.md lines 129-171)

## Acceptance Criteria
- [ ] `IgnorePatternMatcher::from_repository()` loaded once at start of `event_conversion_task()`
- [ ] Filter added inside `while let Some(file_event) = file_event_rx.recv().await` loop
- [ ] FileEvents matching .maproomignore patterns are skipped (not converted to IndexingEvent)
- [ ] Non-ignored events still processed normally (no regression)
- [ ] Invalid patterns in `.maproomignore` cause watcher startup to fail with clear error message
- [ ] Debug logging added for filtered events (e.g., "Ignoring event for maproomignore path: ...")
- [ ] Manual test passes: modify file matching pattern, verify no indexing occurs
- [ ] All existing watcher tests pass (no regression)
- [ ] Code passes `cargo clippy -p crewchief-maproom` with no warnings
- [ ] Code formatted with `cargo fmt`

## Technical Requirements

**Location**: `crates/maproom/src/incremental/worktree_watcher.rs`

**Function**: `event_conversion_task()` (async task, lines 139-163)

**Integration point**: Inside the `while let Some(file_event) = file_event_rx.recv().await` loop (line 144)

**Implementation approach** (from architecture.md lines 143-171):

```rust
/// Task that converts FileEvents to IndexingEvents with worktree tagging.
async fn event_conversion_task(
    worktree_id: WorktreeId,
    mut file_event_rx: mpsc::Receiver<FileEvent>,
    indexing_event_tx: mpsc::Sender<IndexingEvent>,
    repo_root: PathBuf,  // ADD: need repo root for pattern loading
) {
    // Load ignore patterns once at start
    let ignore_matcher = match IgnorePatternMatcher::from_repository(&repo_root) {
        Ok(matcher) => matcher,
        Err(e) => {
            error!("Failed to load ignore patterns, watcher cannot start: {}", e);
            return;  // Fail-fast
        }
    };

    while let Some(file_event) = file_event_rx.recv().await {
        // NEW: Filter events based on .maproomignore patterns
        let path = file_event.path();
        if ignore_matcher.should_ignore(path) {
            debug!("Ignoring event for maproomignore path: {}", path.display());
            continue;
        }

        // Existing conversion logic (unchanged)
        let timestamp = SystemTime::now();
        let indexing_event =
            IndexingEvent::from_file_event(worktree_id.clone(), file_event, timestamp);

        if let Err(e) = indexing_event_tx.send(indexing_event).await {
            warn!("Failed to send indexing event for worktree {}: {}", worktree_id, e);
            return;
        }
    }
}
```

**Required changes**:
1. Add `repo_root: PathBuf` parameter to `event_conversion_task()` signature
2. Update caller to pass repo_root when spawning task
3. Load `IgnorePatternMatcher::from_repository(&repo_root)` at task start
4. Handle loading errors with fail-fast (log error, return early)
5. Add filter check `if ignore_matcher.should_ignore(path)` inside loop
6. Add debug logging for filtered events
7. Continue to next iteration for ignored events

**Error handling**:
- Pattern loading errors prevent watcher from starting (fail-fast)
- Log clear error message indicating .maproomignore has invalid patterns
- Do NOT partially start watcher with incorrect filtering

**Performance considerations** (from architecture.md lines 246-250):
- Pattern loading: Once per watcher start (negligible)
- Per-event filtering: ~100-500ns per file (globset is fast)
- For 1000 file changes: ~0.5ms overhead (acceptable)

## Implementation Notes

**Path normalization**:
- FileEvents contain paths relative to repo root (already normalized by git status)
- Use `normalize_to_relpath()` if needed to match pattern matching expectations
- Patterns are interpreted relative to repository root (git semantics)

**Hot-reload NOT supported** (from architecture.md line 290):
- If .maproomignore changes during active watch, patterns are NOT reloaded
- User must stop and restart watcher to pick up new patterns
- This is acceptable for MVP (can add hot-reload in future)
- Document this limitation in CLAUDE.md update (MRMIGNR-1006)

**Testing approach**:
- Run existing `cargo test -p crewchief-maproom incremental` tests
- Manual test: start watch, create .maproomignore with `*.tmp`, modify .tmp file, verify no index
- Next ticket (MRMIGNR-1005) adds integration test for watch filtering

## Dependencies
- **Prerequisite**: MRMIGNR-1001 (pattern loading infrastructure must exist)
- **Blocks**: MRMIGNR-1005 (integration tests need working implementation)
- **External dependencies**: None (uses existing IgnorePatternMatcher)

## Risk Assessment
- **Risk**: Event filtering breaks incremental updates
  - **Impact**: High - watch becomes unreliable
  - **Mitigation**: Only skip events that match patterns, continue loop for all others. Integration test will verify (MRMIGNR-1005).

- **Risk**: Pattern matcher loaded incorrectly, all events filtered
  - **Impact**: High - nothing gets indexed
  - **Mitigation**: Fail-fast on loading errors. Manual testing required. Add tracing logs to debug.

- **Risk**: Path normalization mismatch between git output and pattern matching
  - **Impact**: Medium - patterns don't work as expected
  - **Mitigation**: Use existing `normalize_to_relpath()` utility. Test with absolute and relative paths.

- **Risk**: Memory leak from unreleased matcher
  - **Impact**: Low - matcher is small and tied to task lifetime
  - **Mitigation**: Matcher is owned by task, dropped when task ends. No explicit cleanup needed.

## Files/Packages Affected
- `crates/maproom/src/incremental/worktree_watcher.rs` (modify `event_conversion_task()` function)
- Caller of `event_conversion_task()` in same file (pass repo_root parameter)

## Verification Notes
The verify-ticket agent should confirm:
1. `IgnorePatternMatcher::from_repository()` called at task start
2. Loading errors cause early return with error log
3. Filter check `should_ignore()` added inside recv loop
4. Filtered events log debug message
5. Continue to next iteration for ignored events
6. Non-ignored events still processed normally
7. Manual test passes:
   - Create .maproomignore with `*.tmp`
   - Start watch operation
   - Create/modify file.tmp
   - Verify no IndexingEvent emitted (check logs or database)
   - Create/modify file.rs
   - Verify IndexingEvent processed normally
8. Existing incremental tests pass
9. No clippy warnings
10. Code formatted properly

**Manual test procedure**:
```bash
# Setup
cd /tmp/test-repo
git init
echo "*.tmp" > .maproomignore
echo "fn main() {}" > main.rs
git add . && git commit -m "init"

# Start watch (in background or separate terminal)
crewchief-maproom watch --repo test --worktree main

# Test filtering
echo "ignored" > test.tmp  # Should NOT trigger indexing
echo "// updated" >> main.rs  # SHOULD trigger indexing

# Check logs or database to verify behavior
```
