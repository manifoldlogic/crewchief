# Plan: maproom ignore patterns

## Overview

This plan implements `.maproomignore` support and unifies ignore pattern handling across scan and watch operations in a single phase. The work is tightly scoped with clear deliverables.

## Phases

### Phase 1: Unified Ignore Pattern Implementation

**Objective:** Implement complete `.maproomignore` support with unified pattern handling across both scan and watch operations.

**Deliverables:**
- Enhanced `load_ignore_patterns()` function in `ignore.rs` that reads `.maproomignore`
- Updated `IgnorePatternMatcher` with `from_repository()` constructor
- Scan integration using `OverrideBuilder` for `.maproomignore` patterns
- Watch integration with event filtering based on `.maproomignore`
- Comprehensive unit tests for pattern loading and precedence
- Integration tests for scan and watch behavior
- Updated CLAUDE.md documentation with `.maproomignore` usage examples

**Agent Assignments:**
- **rust-indexer-engineer**: Core implementation
  - Modify `crates/maproom/src/incremental/ignore.rs`:
    - Add `load_ignore_patterns(root: &Path) -> Result<Vec<String>>`
    - Add `IgnorePatternMatcher::from_repository(root: &Path) -> Result<Self>`
    - Update tests for new constructors
  - Modify `crates/maproom/src/indexer/mod.rs`:
    - Integrate `.maproomignore` patterns via `OverrideBuilder`
    - Merge with existing programmatic exclude logic (if provided)
    - Note: No CLI flag changes needed (exclude parameter is programmatic-only)
  - Modify `crates/maproom/src/incremental/worktree_watcher.rs`:
    - Exact location: `event_conversion_task()` function (lines 139-163)
    - Add filter inside the `while let Some(file_event) = file_event_rx.recv().await` loop
    - Load `IgnorePatternMatcher::from_repository()` once at task start
    - Call `should_ignore()` on each FileEvent before converting to IndexingEvent
    - Skip events that match patterns (continue to next iteration)
  - Update error handling for invalid patterns (fail-fast at watcher startup)

- **unit-test-runner**: Execute tests
  - Run `cargo test -p crewchief-maproom incremental::ignore`
  - Run `cargo test -p crewchief-maproom indexer`
  - Verify all existing tests still pass

- **documentation-writer**: Update documentation
  - Add `.maproomignore` section to `crates/maproom/CLAUDE.md`
  - Include example `.maproomignore` file
  - Document pattern precedence rules
  - Add CLI help text updates if needed

**Acceptance Criteria:**
- [ ] Can create `.maproomignore` with pattern `test-fixtures/**` and files are not indexed during scan
- [ ] Watch operation respects `.maproomignore` patterns (verified via log output or test)
- [ ] Existing `.gitignore` behavior unchanged (regression test)
- [ ] Invalid patterns in `.maproomignore` cause watcher startup to fail with clear error message
- [ ] Unit tests pass for pattern loading logic
- [ ] Integration test demonstrates scan + watch consistency
- [ ] Documentation includes working example with pattern precedence explanation

## Dependencies

**No cross-phase dependencies** - single phase implementation.

**External dependencies:**
- `ignore` crate (already in `Cargo.toml`)
- `globset` crate (already in `Cargo.toml` via `ignore`)
- Git must be installed (already required for watch)

**Pre-requisites:**
- Existing `IgnorePatternMatcher` infrastructure
- Current `WalkBuilder` usage in scan
- Current `GitPoller` implementation in watch

## Risk Mitigation

| Risk | Probability | Impact | Mitigation |
|------|-------------|--------|------------|
| Breaking existing gitignore behavior | Low | High | Comprehensive regression tests before integration |
| Pattern syntax incompatibility with gitignore | Low | Medium | Use same glob syntax, validate with tests |
| Performance regression in scan | Low | Medium | Benchmark scan with/without .maproomignore |
| Watch event filtering breaks incremental updates | Medium | High | Integration test with file changes and .maproomignore |
| OverrideBuilder API misuse | Low | Medium | Test negative override syntax with multiple patterns |

**Mitigation actions:**
1. Create test repository with `.gitignore` and `.maproomignore` to verify independence
2. Benchmark scan operation before/after changes (should be <1% difference)
3. Add tracing logs to watch filter for debugging
4. Test edge cases: empty file, comment-only, invalid patterns

## Success Metrics

- [x] **Functional:**
  - [ ] Scan operation ignores files matching `.maproomignore` patterns
  - [ ] Watch operation ignores file changes matching `.maproomignore` patterns
  - [ ] `.gitignore` patterns continue to work unchanged
  - [ ] Invalid patterns cause fail-fast errors at startup

- [x] **Quality:**
  - [ ] All unit tests pass (including new pattern tests)
  - [ ] Integration tests demonstrate scan/watch parity
  - [ ] No regression in existing test suite
  - [ ] Code passes `cargo clippy` with no warnings

- [x] **Documentation:**
  - [ ] CLAUDE.md updated with `.maproomignore` section
  - [ ] Example `.maproomignore` file provided
  - [ ] Pattern precedence clearly documented

- [x] **Performance:**
  - [ ] Scan benchmark shows <1% overhead with .maproomignore
  - [ ] Watch event filtering adds <1ms per event

## Implementation Notes

### Order of Work

1. **Foundation (ignore.rs):**
   - Implement `load_ignore_patterns()` function
   - Add `from_repository()` constructor
   - Write unit tests

2. **Scan Integration (indexer/mod.rs):**
   - Call `load_ignore_patterns()` before WalkBuilder setup
   - Build overrides with patterns
   - Merge with CLI excludes
   - Test with sample repository

3. **Watch Integration:**
   - Modify `worktree_watcher.rs::event_conversion_task()` (line 139-163)
   - Load `IgnorePatternMatcher::from_repository()` once at start of async task
   - Add filter inside recv() loop (line 144) before IndexingEvent conversion
   - Skip events that match patterns using `should_ignore()`
   - Add debug logging for filtered events
   - Handle pattern loading errors (fail-fast if invalid patterns)
   - Test with file changes
   - **Note:** Pattern reload NOT supported - watcher restart required if .maproomignore changes

4. **Documentation:**
   - Update CLAUDE.md
   - Add example to docs/

5. **Verification:**
   - Run full test suite
   - Manual testing with .maproomignore
   - Performance benchmarks

### Critical Path Tests

These tests MUST pass before merge:

1. **Pattern loading:**
   - Load .maproomignore with various patterns
   - Handle missing file gracefully
   - Parse comments and blank lines correctly

2. **Scan integration:**
   - Create .maproomignore with `test/**`
   - Verify files under test/ not indexed
   - Verify .gitignore patterns still work

3. **Watch integration:**
   - Start watch with .maproomignore
   - Modify file matching pattern
   - Verify no indexing event triggered

4. **Error handling:**
   - .maproomignore contains invalid pattern `[invalid`
   - Watcher startup fails with clear error message
   - Scan fails with clear error message

### Testing Strategy

**Unit tests** (`crates/maproom/src/incremental/ignore.rs`):
```rust
#[test]
fn test_load_ignore_patterns_missing_file() { ... }

#[test]
fn test_load_ignore_patterns_with_comments() { ... }

#[test]
fn test_from_repository_constructor() { ... }
```

**Integration tests** (new file: `crates/maproom/tests/maproomignore_test.rs`):
```rust
#[test]
fn test_scan_respects_maproomignore() { ... }

#[tokio::test]
async fn test_watch_filters_maproomignore_events() { ... }

#[test]
fn test_invalid_patterns_fail_startup() { ... }

#[test]
fn test_gitignore_still_works_independently() { ... }
```

## Rollback Plan

If critical issues discovered post-merge:

1. **Immediate:** Revert the commit (single-phase = single revert)
2. **Investigation:** Identify root cause in isolated test environment
3. **Fix forward:** Apply targeted fix with additional tests
4. **Re-deploy:** With expanded test coverage

**Safe to rollback because:**
- No database schema changes
- Feature is opt-in (requires creating .maproomignore)
- Backward compatible (repos without .maproomignore unaffected)
