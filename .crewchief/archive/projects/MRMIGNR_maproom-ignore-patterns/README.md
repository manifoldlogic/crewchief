# Project: Maproom Ignore Patterns

**Slug:** MRMIGNR
**Status:** ✅ Complete
**Created:** 2025-12-04
**Completed:** 2025-12-05

## Summary

Add `.maproomignore` file support to enable users to exclude files from indexing that are committed to git but should not be in the semantic search index. Unifies ignore pattern handling across scan and watch operations for consistency.

## Problem Statement

Maproom currently lacks a way to exclude committed files from indexing. Users often need to version-control large test fixtures, generated baselines, or legacy code that creates search noise. While `.gitignore` prevents files from being version-controlled, there is no equivalent for "version-control this, but don't index it."

Additionally, scan and watch operations handle ignore patterns differently:
- **Scan**: Uses `WalkBuilder` with `.gitignore` support
- **Watch**: Uses git status polling (automatic `.gitignore` respect) plus separate hardcoded default patterns

This fragmentation makes behavior unpredictable and prevents users from controlling what gets indexed.

## Proposed Solution

Implement `.maproomignore` file support (analogous to `.gitignore`) that:

1. **Unified pattern loading**: Single `load_ignore_patterns()` function reads `.maproomignore` and merges with defaults
2. **Scan integration**: Use `ignore` crate's `OverrideBuilder` to exclude `.maproomignore` patterns
3. **Watch integration**: Filter `FileEvent` stream based on `.maproomignore` patterns before processing
4. **Pattern precedence**: CLI `--exclude` > `.maproomignore` > `.gitignore` > defaults

**Key architectural decision**: Reuse existing `ignore` crate infrastructure (already handling `.gitignore`) and extend it with `.maproomignore` support via the `OverrideBuilder` API.

## Key Deliverables

1. Enhanced `crates/maproom/src/incremental/ignore.rs`:
   - `load_ignore_patterns(root: &Path) -> Result<Vec<String>>` function
   - `IgnorePatternMatcher::from_repository(root: &Path)` constructor

2. Updated `crates/maproom/src/indexer/mod.rs`:
   - Integrate `.maproomignore` patterns via `OverrideBuilder`
   - Preserve CLI `--exclude` precedence

3. Watch event filtering (in processor or worktree_watcher):
   - Filter `FileEvent` stream with `should_ignore()` check
   - Debug logging for filtered events

4. Comprehensive tests:
   - Unit tests for pattern loading and precedence
   - Integration tests for scan and watch behavior
   - New test file: `crates/maproom/tests/maproomignore_test.rs`

5. Updated documentation:
   - `crates/maproom/CLAUDE.md` with `.maproomignore` usage
   - Example `.maproomignore` file

## Relevant Agents

- **project-planner**: Planning phase (complete)
- **ticket-creator**: Ticket generation (next step)
- **rust-indexer-engineer**: Core implementation
- **unit-test-runner**: Test execution
- **documentation-writer**: CLAUDE.md updates
- **verify-ticket**: Verification
- **commit-ticket**: Commit

## Planning Documents

- [analysis.md](planning/analysis.md) - Problem analysis and research findings
- [architecture.md](planning/architecture.md) - Solution design and component architecture
- [plan.md](planning/plan.md) - Single-phase execution plan
- [quality-strategy.md](planning/quality-strategy.md) - Pragmatic testing approach
- [security-review.md](planning/security-review.md) - Security assessment (low risk)

## Technical Highlights

**No new dependencies**: Uses existing `ignore` and `globset` crates

**Performance**: <1% overhead on scan, <1ms per event on watch

**Backward compatible**:
- Repos without `.maproomignore` work unchanged
- Existing `.gitignore` behavior preserved
- CLI `--exclude` continues to work

**MVP scope**:
- Single `.maproomignore` at repo root
- Gitignore-style glob patterns
- Comment and blank line support
- Fail-fast on invalid patterns

**Future extensions** (not in MVP):
- Global ignore: `~/.config/crewchief/maproomignore`
- Per-worktree overrides: `.maproomignore.local`
- Environment variable: `MAPROOM_IGNORE_PATTERNS`

## Example Usage

After implementation, users will create `.maproomignore` in their repository:

```
# .maproomignore - Exclude from maproom indexing

# Large test fixtures
test-fixtures/**
tests/baseline/**

# Generated files that are version-controlled
*.sql
*.generated.ts

# Legacy code directories
legacy/
vendor/old-deps/

# Documentation in other languages
docs/translations/**
```

Then run scan or watch - excluded files will not be indexed:

```bash
# Scan respects .maproomignore
crewchief-maproom scan --path /repo --repo myrepo --worktree main

# Watch also respects .maproomignore
crewchief-maproom watch --repo myrepo --worktree main --path /repo
```

## Success Criteria

- [x] Planning documents complete
- [x] Tickets created (7 tickets: Phase 1 + Phase 2)
- [x] Implementation complete (all code changes committed)
- [x] Tests passing (13 unit tests, 4 integration tests)
- [x] Documentation updated (CLAUDE.md with comprehensive guide)
- [x] Manual verification successful (all tickets verified)

## Final Deliverables

**Implementation commits (5 commits):**
1. `465997d1` - MRMIGNR-1002: Scan integration with OverrideBuilder
2. `023ee4e3` - MRMIGNR-1003: Watch integration with event filtering
3. `e564224c` - MRMIGNR-1004: Comprehensive unit tests (9 tests)
4. `38d87e0f` - MRMIGNR-1005: Integration tests (4 tests)
5. `532b8f09` - MRMIGNR-1006: Documentation in CLAUDE.md

**Plus MRMIGNR-2001 from previous session**: Clean ignored command

**Total implementation time**: ~2 days
**Lines of code added**: ~800 lines (implementation + tests + documentation)

## Related Work

- Initiative: `.crewchief/initiatives/2025-12-03_maproom-ignore-patterns/`
- Existing ignore handling: `crates/maproom/src/incremental/ignore.rs`
- Watch implementation: `crates/maproom/src/incremental/watcher.rs`
- Scan implementation: `crates/maproom/src/indexer/mod.rs`
