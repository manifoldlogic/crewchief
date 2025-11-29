# Ticket: GITPOLL-3001: Cleanup and Documentation

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

Remove the unused `notify` dependency (or make it optional), clean up any dead code, and update documentation to reflect the git polling approach.

## Background

With git polling fully integrated and tested, the notify-based implementation is no longer needed. This ticket cleans up the codebase and ensures documentation accurately describes the new behavior.

Reference: [plan.md](../planning/plan.md) - Phase 3: Cleanup and Documentation

## Acceptance Criteria

- [ ] `notify` crate removed from dependencies (or made optional/feature-gated)
- [ ] Dead code removed (old watcher implementation if any remains)
- [ ] `crates/maproom/CLAUDE.md` updated with git polling documentation
- [ ] No "too many open files" errors when running watch on large repo (manual test)
- [ ] All tests pass after cleanup

## Technical Requirements

### Dependency Cleanup

Update `crates/maproom/Cargo.toml`:

```toml
# REMOVE (or feature-gate):
# notify = "6.0"

# KEEP if needed for other features, otherwise remove:
# [features]
# native-watcher = ["notify"]  # Optional: keep as fallback
```

### Code Cleanup

Review and remove:
- Any unused imports related to `notify`
- Dead code paths that reference the old watcher
- Unused configuration options
- Comments referencing the old approach

Files to check:
- `crates/maproom/src/incremental/watcher.rs`
- `crates/maproom/src/incremental/mod.rs`
- Any files that import `notify`

### Documentation Updates

Update `crates/maproom/CLAUDE.md` with:

```markdown
## File Watching

Maproom uses git-based polling for file change detection:

### How It Works

- Polls `git status --porcelain` at configurable intervals (default: 3 seconds)
- Compares state between polls to detect changes
- Emits FileEvent/IndexingEvent for downstream processing

### Configuration

```rust
WatcherConfig {
    poll_interval_ms: 3000,    // Polling interval in milliseconds
    include_untracked: true,   // Watch untracked files (respects .gitignore)
    detect_renames: true,      // Detect file renames
    git_timeout_ms: 10000,     // Timeout for git command
}
```

### Why Git Polling?

The previous `notify`-based approach caused "too many open files" (EMFILE) errors on large repositories because it created file descriptors for every watched directory. Git polling:

- Uses zero file descriptors
- Automatically respects `.gitignore`
- Works consistently across platforms
- Trades instant detection for 2-5s latency (acceptable for dev workflow)

### Requirements

- Git must be installed and in PATH
- Must be run in a git repository
- Returns error for non-git directories
```

### Manual Testing Checklist

Verify on a large repository:
1. Run `maproom watch` on this codebase
2. Verify no "too many open files" errors
3. Modify a file, verify event received within poll interval
4. Delete a file, verify deletion event
5. Create new file, verify new file event
6. Run during git rebase, verify no crashes

### Verification Commands

```bash
# Check notify dependency removed
cargo tree -p crewchief-maproom | grep -i notify
# Should return nothing if removed

# Run all tests
cargo test -p crewchief-maproom

# Check for unused code warnings
cargo clippy -p crewchief-maproom -- -W dead_code

# Manual test on large repo
cd /path/to/large/repo
maproom watch
```

## Implementation Notes

### Gradual Removal

If uncertain about removing `notify`:
1. First, make it optional with feature flag
2. Disable by default
3. Remove entirely in future release if no issues

```toml
[features]
default = []
native-watcher = ["notify"]

[dependencies]
notify = { version = "6.0", optional = true }
```

### Dead Code Detection

Run clippy with dead code warnings:
```bash
cargo clippy -p crewchief-maproom -- -W dead_code -W unused_imports
```

### Documentation Location

Primary documentation in `crates/maproom/CLAUDE.md`. This is read by both developers and AI agents working on the codebase.

## Dependencies

- GITPOLL-2001: FileWatcher integration
- GITPOLL-2002: WorktreeWatcher integration
- GITPOLL-2901: Integration tests (validates system works before cleanup)

## Risk Assessment

- **Risk**: Removing notify breaks other code paths
  - **Mitigation**: Search codebase for all `notify` usages before removal. Feature-gate if uncertain.

- **Risk**: Documentation becomes outdated
  - **Mitigation**: Link to source of truth (this project's architecture.md) in docs

- **Risk**: Manual testing missed on CI
  - **Mitigation**: Document manual testing procedure. Consider adding performance test to CI.

## Files/Packages Affected

- `crates/maproom/Cargo.toml` (remove/feature-gate notify)
- `crates/maproom/src/incremental/watcher.rs` (cleanup imports)
- `crates/maproom/src/incremental/mod.rs` (cleanup if needed)
- `crates/maproom/CLAUDE.md` (documentation updates)
