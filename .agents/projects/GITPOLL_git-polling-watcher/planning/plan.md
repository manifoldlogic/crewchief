# Implementation Plan: Git Polling File Watcher

## Overview

Replace the notify-based file watcher with a git status polling mechanism to eliminate "too many open files" errors on large repositories.

## Phase 1: Core Implementation

**Goal**: Implement GitPoller and GitState components

### Deliverables

1. **git_state.rs** - State representation and parsing
   - `FileStatus` enum (Clean, Modified, New, Deleted, Renamed)
   - `GitState` struct with HashMap storage
   - `from_git_status()` parser for porcelain output
   - `diff()` method to compare states and emit FileEvents
   - Path validation (reject absolute paths, `..` traversal)

2. **git_poller.rs** - Polling loop and event emission
   - `GitPollerConfig` configuration struct
   - `GitPoller` struct with polling loop
   - `poll_once()` for single poll cycle
   - `run()` async loop with configurable interval
   - `shutdown()` graceful termination
   - Error handling with retry logic

3. **Unit tests**
   - Git status parsing (all status codes)
   - State diff logic (all change types)
   - Path validation
   - Error handling

**Agent**: `rust-indexer-engineer`

### Acceptance Criteria

- [ ] Parse all git status codes (M, A, D, R, ??, etc.)
- [ ] Detect file modifications, additions, deletions, renames
- [ ] Handle paths with spaces and unicode
- [ ] Reject path traversal attempts
- [ ] Configurable poll interval (default 3s)
- [ ] Graceful handling of git failures

## Phase 2: Integration

**Goal**: Wire GitPoller into existing watcher infrastructure

### Deliverables

1. **Modify watcher.rs**
   - Replace `RecommendedWatcher` with `GitPoller`
   - Update `FileWatcher` to use `GitPoller` internally
   - Preserve existing interface (`watch()`, `stop()`)

2. **Modify worktree_watcher.rs**
   - Update to create `GitPoller` instead of notify watcher
   - Preserve `IndexingEvent` emission interface

3. **Update WatcherConfig**
   - Add git polling configuration options
   - Backward compatible defaults

4. **Integration tests**
   - End-to-end: modify file → event received
   - Multi-file changes in single poll cycle
   - Recovery from git failures

**Agent**: `rust-indexer-engineer`

### Acceptance Criteria

- [ ] FileWatcher API unchanged (drop-in replacement)
- [ ] WorktreeWatcher works with GitPoller
- [ ] Existing debouncing logic still works
- [ ] No notify crate usage in active code path

## Phase 3: Cleanup and Documentation

**Goal**: Remove old implementation and update docs

### Deliverables

1. **Remove notify dependency** (or make it optional)
   - Update Cargo.toml
   - Remove unused code

2. **Update CLAUDE.md**
   - Document git polling approach
   - Document configuration options

3. **Manual testing**
   - Test on crewchief codebase (large repo)
   - Test on repo with deep node_modules
   - Verify no FD errors

**Agent**: `rust-indexer-engineer`

### Acceptance Criteria

- [ ] No "too many open files" errors on large repos
- [ ] Documentation updated
- [ ] Unused code removed

## Testing Milestones

| Phase | Test Type | Coverage |
|-------|-----------|----------|
| 1 | Unit | Parsing, diffing, validation |
| 2 | Integration | End-to-end event flow |
| 3 | Manual | Large repo stress test |

## Security Checkpoints

| Phase | Security Item | Status |
|-------|--------------|--------|
| 1 | Path validation implemented | Pending |
| 1 | No shell command injection | Pending |
| 2 | Interface unchanged (no new surface) | Pending |
| 3 | Final security review | Pending |

## Agent Assignments

| Phase | Primary Agent | Responsibilities |
|-------|---------------|------------------|
| 1 | rust-indexer-engineer | Core Rust implementation |
| 2 | rust-indexer-engineer | Integration with existing code |
| 3 | rust-indexer-engineer | Cleanup and documentation |

All phases use the same agent since this is pure Rust work in the maproom crate.

## Dependencies

### External

- None (git is assumed available)

### Internal

- Existing `FileEvent` types (preserved, not modified)
- Existing `IndexingEvent` types (preserved, not modified)
- `IgnorePatternMatcher` (may still be useful for non-git filtering)

## Risks and Mitigations

| Risk | Impact | Mitigation |
|------|--------|------------|
| Git not installed | High | Check for git on startup, clear error message |
| Git status slow on huge repos | Medium | Configurable timeout, skip cycle on timeout |
| Breaking existing watch command | High | Preserve FileEvent interface exactly |

## Success Criteria

1. **Primary**: Zero "too many open files" errors
2. **Secondary**: File changes detected within configured interval
3. **Tertiary**: No regression in existing functionality

## Estimated Scope

- **Phase 1**: ~500 lines of new Rust code
- **Phase 2**: ~100 lines of modifications
- **Phase 3**: ~50 lines removed, documentation updates

Total: Moderate complexity, well-bounded scope.
