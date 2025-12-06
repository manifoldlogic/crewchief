# Plan: Enhanced Worktree Clean

## Overview

This plan outlines the phased approach to extending `crewchief worktree clean` with maproom database cleanup and git branch deletion. The work is divided into 3 phases focusing on binary resolution, command integration, and testing/documentation.

**Estimated effort:** M (2.5-4 days)
**Dependencies:** MRBIN (binary resolution logic - copied from maproom-mcp, will consolidate later)

## Phases

### Phase 1: Binary Resolution and Maproom Integration

**Objective:** Enable TypeScript CLI to discover and invoke maproom binary for database cleanup

**Deliverables:**
- Binary discovery utility (`packages/cli/src/utils/maproom-binary.ts`)
- Maproom cleanup helper function
- Unit tests for binary discovery
- Mock tests for cleanup invocation

**Agent Assignments:**
- typescript-engineer: Implement `findMaproomBinary()` utility
- typescript-engineer: Create `cleanMaproomRecords()` helper function
- unit-test-runner: Run test suite
- verify-ticket: Verify binary discovery works across platforms

**Tickets:**
- WTCLEAN-1001: Implement binary discovery utility
- WTCLEAN-1002: Create maproom cleanup helper function
- WTCLEAN-1003: Add unit tests for binary resolution

**Success Criteria:**
- Binary discovery finds packaged, dev, and PATH binaries
- Cleanup function calls `db cleanup-stale --confirm` successfully
- Tests cover all fallback strategies
- Graceful handling when binary not found

**Note on binary resolution:** This phase copies `findMaproomBinary()` from maproom-mcp as a pragmatic MVP decision. This creates temporary code duplication that will be consolidated when the MRBIN project completes. A follow-up ticket should be created after MRBIN delivers to replace this copied code with the shared utility.

### Phase 2: Enhanced Clean Command

**Objective:** Integrate maproom cleanup and branch deletion into `worktree clean` command

**Deliverables:**
- Updated `worktree clean` command with new cleanup steps
- New CLI flags: `--keep-branch`, `--keep-maproom`
- Branch deletion using `GitMergeService`
- Graceful error handling with clear logging

**Agent Assignments:**
- typescript-engineer: Modify `worktree clean` command handler
- typescript-engineer: Add opt-out flags to CLI
- typescript-engineer: Integrate `GitMergeService.deleteBranch()`
- unit-test-runner: Run test suite
- verify-ticket: Verify complete cleanup workflow

**Tickets:**
- WTCLEAN-2001: Add `--keep-branch` and `--keep-maproom` flags
- WTCLEAN-2002: Integrate maproom cleanup in clean command
- WTCLEAN-2003: Integrate branch deletion in clean command (⚠️ extract branch BEFORE worktree removal)
- WTCLEAN-2004a: Add error handling for maproom cleanup
- WTCLEAN-2004b: Add error handling for branch deletion
- WTCLEAN-2004c: Add logging for all cleanup steps

**Success Criteria:**
- Clean command deletes directory, metadata, branch, and database records
- Branch name extracted BEFORE worktree removal (critical sequencing)
- Failures logged as warnings, don't block cleanup
- Clear feedback for each step
- `--keep-branch` and `--keep-maproom` flags work correctly
- Manual recovery instructions provided for each failure scenario

**Dependencies:**
- Requires Phase 1 (binary resolution and cleanup function)

### Phase 3: Testing and Documentation

**Objective:** Comprehensive test coverage and user documentation

**Deliverables:**
- Integration tests for complete cleanup workflow
- Failure scenario tests (binary missing, branch protected, etc.)
- Updated README with new flags and behavior
- Test cleanup with actual maproom database

**Agent Assignments:**
- typescript-engineer: Write integration tests
- typescript-engineer: Write failure scenario tests
- typescript-engineer: Update CLI README documentation
- unit-test-runner: Run full test suite
- verify-ticket: Verify all scenarios covered

**Tickets:**
- WTCLEAN-3001: Add integration tests for cleanup workflow
- WTCLEAN-3002: Add failure scenario tests
- WTCLEAN-3003: Update README documentation

**Success Criteria:**
- Tests cover happy path (all cleanup succeeds)
- Tests cover partial failure (maproom missing, branch protected)
- Tests verify graceful degradation
- README documents new flags and behavior
- All tests pass

**Dependencies:**
- Requires Phase 2 (command integration)

## Execution Strategy

### Phase Ordering Rationale

**Why Phase 1 first:**
- Binary resolution is foundational infrastructure
- Can be tested independently
- Unblocks both maproom and branch cleanup

**Why Phase 2 second:**
- Requires Phase 1 utilities
- Core value delivery (complete cleanup)
- Integration point for all components

**Why Phase 3 last:**
- Requires working implementation to test
- Validates entire integration
- Documentation reflects actual behavior

### Parallel Work Opportunities

- **Phase 1 & 2 overlap:** Branch deletion can be implemented while binary resolution is being tested
- **Within Phase 2:** Maproom and branch cleanup can be developed in parallel (independent features)
- **Phase 3 start early:** Documentation can begin once Phase 2 design is complete

### Critical Path

```
Binary Resolution (1001-1003)
        ↓
Maproom Integration (2002) → Error Handling (2004a-2004c)
        ↓                            ↓
Branch Integration (2003) ──────────┘
        ↓
Integration Tests (3001-3002)
        ↓
Documentation (3003)
```

**Bottlenecks:**
- Binary resolution must complete before maproom integration
- Branch extraction MUST happen before worktree removal in ticket 2003 (critical sequencing)
- Error handling blocks final testing
- All implementation must complete before integration tests

**Ticket 2003 Critical Requirement:** Code must extract branch name from worktree metadata BEFORE calling `removeWorktree()`. Once worktree is removed, git metadata is gone and branch name cannot be determined. This sequencing is easy to miss and must be verified during ticket review.

## Dependencies

### External Dependencies

**MRBIN (Binary Resolution):**
- Status: NOT a blocking dependency - this project copies code from maproom-mcp
- Impact: Creates temporary code duplication
- Mitigation:
  - Copy `findMaproomBinary()` from maproom-mcp to packages/cli/src/utils/
  - Create follow-up ticket after MRBIN completes to consolidate into shared utility
  - Document this as technical debt with planned resolution
- Future action: When MRBIN delivers, replace copied code with import from shared utility

**WTPATH (Path Utilities):**
- Status: Optional
- Impact: Could simplify worktree path parsing
- Mitigation: Use existing path utilities if WTPATH not available

### Internal Dependencies

**Phase 1 → Phase 2:**
- `findMaproomBinary()` must be complete before command integration
- `cleanMaproomRecords()` must be tested before integration

**Phase 2 → Phase 3:**
- Command integration must be complete before integration tests
- All features implemented before documentation finalized

### Codebase Dependencies

**Existing components we depend on:**
- `GitMergeService.deleteBranch()` - Already exists, tested
- `WorktreeService.removeWorktree()` - Already exists, tested
- `removeDirSync()` - Already exists, tested
- `logger` utilities - Already exists

**Components that may depend on us:**
- None (this is a leaf enhancement)

## Risk Mitigation

| Risk | Probability | Impact | Mitigation |
|------|-------------|--------|------------|
| Maproom binary not found | Medium | Medium | Graceful degradation, clear warning, manual cleanup instructions |
| Branch deletion fails (not merged) | Medium | Low | Catch error, log warning, don't block cleanup |
| Database locked during cleanup | Low | Low | Maproom handles SQLite locking, log warning if fails |
| Cross-platform binary resolution fails | Low | High | Test on all platforms (darwin, linux, win32), multiple fallback strategies |
| Breaking existing `clean` command | Low | High | Add new behavior without changing existing flags, comprehensive tests |
| Tests fail to catch edge cases | Medium | Medium | Explicit failure scenario tests, integration tests with real git/maproom |
| Branch extraction timing error | Medium | High | Explicit acceptance criterion, code review checkpoint, integration test validates sequencing |
| Batch cleanup performance | Low | Medium | Document performance characteristics, test with 50+ worktrees, acceptable for MVP |

### Contingency Plans

**If binary resolution proves difficult:**
- Simplify to PATH-only lookup
- Document that users must install crewchief-maproom globally
- Still better than current state (no cleanup at all)

**If branch deletion causes issues:**
- Make it opt-in instead of opt-out (`--delete-branch` flag)
- Document limitation clearly
- Users can manually delete branches as before

**If test coverage insufficient:**
- Manual testing on dev machines
- Document known limitations
- Add tests in follow-up PR

## Success Metrics

### Must-Have (MVP)

- [x] `findMaproomBinary()` discovers binary on all platforms
- [x] `clean` command calls `db cleanup-stale --confirm` after removal
- [x] `clean` command deletes branch with `git branch -d`
- [x] `--keep-branch` flag preserves branch
- [x] `--keep-maproom` flag skips database cleanup
- [x] Maproom cleanup failures logged as warnings, not errors
- [x] Branch deletion failures logged as warnings, not errors
- [x] Integration tests verify complete cleanup
- [x] Failure tests verify graceful degradation
- [x] README documents new behavior

### Nice-to-Have (Future Enhancements)

- [ ] `--force` flag for `git branch -D` (force delete unmerged)
- [ ] `--dry-run` flag to preview cleanup
- [ ] Daemon client support for faster cleanup
- [ ] Parallel execution of cleanup steps
- [ ] Targeted maproom deletion (specific repo/worktree)

### Behavioral Specifications

**`--all` mode behavior:**
- Loops through all non-current worktrees
- Applies full cleanup flow to each worktree (directory, metadata, branch, maproom)
- Branch deletion happens for each worktree unless `--keep-branch` specified
- If any branch deletion fails, logs warning but continues with remaining worktrees
- Maproom cleanup runs once after all worktree removals (batch cleanup detects all stale)

**Batch cleanup performance:**
- Uses `db cleanup-stale --confirm` which scans all worktrees in database
- For databases with 50+ worktrees, cleanup may take 2-5 seconds
- Acceptable for MVP (cleanup is infrequent operation)
- Future optimization: Pass specific repo/worktree to cleanup command
- Manual testing should include databases with 50+ worktrees to validate performance

## Verification Plan

### Phase 1 Verification

**Test binary discovery:**
```bash
# Test packaged binary
node -e "console.log(require('./dist/utils/maproom-binary').findMaproomBinary())"

# Test with env var
CREWCHIEF_MAPROOM_BIN=/custom/path node ...

# Test with missing binary
mv bin/crewchief-maproom /tmp/ && node ...

# Test Windows .exe extension handling (Windows only)
# Verify binary resolution finds crewchief-maproom.exe
# Verify spawnSync works with .exe extension
```

**Expected results:**
- Finds packaged binary in normal case
- Respects CREWCHIEF_MAPROOM_BIN override
- Returns null when binary missing (doesn't crash)
- Windows: Correctly handles .exe extension
- Cross-platform: Works on macOS, Linux, Windows

### Phase 2 Verification

**Test complete cleanup:**
```bash
# Create worktree
crewchief worktree create test-feature

# Index it (if maproom available)
cd .crewchief/worktrees/test-feature
crewchief-maproom scan

# Clean it
cd ../../..
crewchief worktree clean test-feature

# Verify everything cleaned:
git worktree list | grep test-feature  # Should be empty
ls .crewchief/worktrees/ | grep test-feature  # Should be empty
git branch | grep test-feature  # Should be empty
crewchief-maproom status | grep test-feature  # Should be empty
```

**Test graceful degradation:**
```bash
# Rename binary so it's not found
mv bin/crewchief-maproom /tmp/

# Clean should still work (with warning)
crewchief worktree clean test-feature
# Expect: Warning about maproom, but directory/branch still cleaned

# Restore binary
mv /tmp/crewchief-maproom bin/
```

**Test branch protection:**
```bash
# Create worktree with unmerged changes
crewchief worktree create unmerged-branch
cd .crewchief/worktrees/unmerged-branch
echo "changes" >> file.txt
git add file.txt && git commit -m "unmerged"

# Clean should warn but not fail
cd ../../..
crewchief worktree clean unmerged-branch
# Expect: Warning about branch not deleted, but directory/maproom cleaned
```

**Test opt-out flags:**
```bash
# Keep branch
crewchief worktree create keep-branch-test
crewchief worktree clean keep-branch-test --keep-branch
git branch | grep keep-branch-test  # Should exist

# Keep maproom records
crewchief worktree clean keep-branch-test --keep-maproom
crewchief-maproom status  # Should still have record (if was indexed)
```

### Phase 3 Verification

**Run full test suite:**
```bash
cd packages/cli
pnpm test
```

**Verify integration tests:**
- Create/clean worktree → all artifacts removed
- Mock binary missing → graceful degradation
- Mock branch protected → warning logged
- Test --keep-branch flag → branch preserved
- Test --keep-maproom flag → skip cleanup

**Verify documentation:**
- README shows new flags
- Examples demonstrate complete cleanup
- Edge cases documented (binary missing, branch protected)

## Timeline Estimate

| Phase | Tickets | Estimated Time | Notes |
|-------|---------|----------------|-------|
| Phase 1 | WTCLEAN-1001 to 1003 | 0.5-1 day | Binary resolution is straightforward |
| Phase 2 | WTCLEAN-2001 to 2004c | 1.5-2 days | Main implementation work (6 tickets, includes split error handling) |
| Phase 3 | WTCLEAN-3001 to 3003 | 0.5-1 day | Testing and documentation |
| **Total** | **11 tickets** | **2.5-4 days** | Includes buffer for integration complexity |

**Assumptions:**
- No major blockers or design changes
- Tests pass on first attempt (or minor fixes needed)
- Binary resolution works as expected across platforms
- Branch extraction sequencing implemented correctly

**Buffer included:**
- 0.5-1 day buffer for unexpected issues
- Cross-platform testing time (especially Windows)
- Integration complexity with multiple cleanup steps
- Ticket 2003 sequencing verification

**Why 2.5-4 days (not 2-3):**
- Review identified timeline optimism
- Cross-platform testing adds time
- Integration testing with real git/maproom takes longer than anticipated
- Error handling across three separate tickets (2004a-c) adds granularity but ensures quality

## Rollout Strategy

### Pre-Rollout Checklist

- [ ] All tests pass on CI
- [ ] Manual testing on macOS (darwin-arm64, darwin-x64)
- [ ] Manual testing on Linux (linux-x64, linux-arm64)
- [ ] Manual testing on Windows (win32-x64)
- [ ] Documentation updated
- [ ] CHANGELOG entry added

### Rollout Phases

**Phase 1: Merge to main**
- All tickets complete and verified
- Tests passing
- Documentation updated

**Phase 2: Release (following normal release process)**
- CLI package version bump
- Release notes mention new behavior
- Users informed about `--keep-branch` flag for old behavior

**Phase 3: Monitor for issues**
- Watch for bug reports about cleanup failures
- Monitor performance (should be <5 seconds)
- Collect feedback on new behavior

### Rollback Plan

**If critical issues found:**
- Add feature flag to disable new cleanup steps
- Revert to old behavior by default
- Fix issues and re-release

**Rollback is easy because:**
- New code is additive (doesn't change existing logic)
- Opt-out flags allow users to get old behavior
- No database migrations or breaking changes

## Notes

**Why batch cleanup is better than targeted deletion:**
- Simpler implementation (no repo name parsing)
- More robust (cleans up ANY stale worktrees)
- Already exists (IDXCLEAN project built it)
- Minimal performance impact (<1 second difference)

**Why direct spawn is better than daemon client:**
- Simpler (no daemon lifecycle management)
- Less coupling (don't depend on daemon running)
- Adequate performance (cleanup is infrequent)
- Can switch to daemon later if needed

**Why safe delete is better than force delete:**
- Prevents accidental loss of unmerged work
- Matches existing `worktree merge` behavior
- Error message guides user to manual cleanup
- Can add `--force` flag later if needed

**Key insight from `worktree merge`:**
The merge command already demonstrates complete cleanup. We're adapting its proven pattern to the clean command, ensuring consistency and reliability.
