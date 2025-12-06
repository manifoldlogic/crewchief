# Project: Enhanced Worktree Clean

**Slug:** WTCLEAN
**Status:** Completed
**Created:** 2025-12-05
**Completed:** 2025-12-06
**Effort:** M (2.5-4 days)
**Actual Duration:** ~1 day

## Summary

Extend `crewchief worktree clean` to perform **complete cleanup** by removing directory, git worktree metadata, git branch, and maproom database records. Current implementation only removes directory and git metadata, leaving behind branches and stale database entries that cause duplicate search results and require manual cleanup.

## Problem Statement

The current `worktree clean` command is incomplete:

**Current behavior:**
- ✅ Removes worktree directory
- ✅ Removes git worktree metadata
- ❌ Leaves git branch behind
- ❌ Leaves maproom database records

**Impact:**
- Developers must manually run `git branch -d <branch>` after cleanup
- Developers must manually run `crewchief-maproom db cleanup-stale`
- Database bloat with stale entries
- Duplicate search results from deleted branches
- Error-prone multi-step cleanup process

## Proposed Solution

Add two new cleanup steps to the `worktree clean` command:

1. **Maproom cleanup** - Call `crewchief-maproom db cleanup-stale --confirm` after directory removal
2. **Branch deletion** - Call `GitMergeService.deleteBranch()` to remove the branch

**Key principles:**
- **Best-effort cleanup** - Failures logged as warnings, don't block subsequent steps
- **Graceful degradation** - Works even if maproom binary missing
- **User control** - Add `--keep-branch` and `--keep-maproom` opt-out flags
- **Safe by default** - Use `git branch -d` (safe delete, requires merge)

**Value proposition:** Single command removes all traces of a worktree, preventing database bloat and stale search results.

## Relevant Agents

- **project-planner** - Planning phase (this document)
- **typescript-engineer** - Implement binary resolution, command integration, tests
- **unit-test-runner** - Run test suite after implementation
- **verify-ticket** - Verify acceptance criteria for each ticket
- **commit-ticket** - Create commits after verification

## Planning Documents

- [analysis.md](planning/analysis.md) - Problem analysis (why current cleanup is incomplete)
- [architecture.md](planning/architecture.md) - Solution design (how to integrate all cleanup steps)
- [plan.md](planning/plan.md) - Execution plan (3 phases, 9 tickets, 2-3 days)
- [quality-strategy.md](planning/quality-strategy.md) - Testing approach (unit, integration, failure scenarios)
- [security-review.md](planning/security-review.md) - Security assessment (LOW risk, no new attack vectors)

## Key Decisions

### 1. Batch Cleanup vs Targeted Deletion

**Decision:** Use batch cleanup (`db cleanup-stale --confirm`)

**Rationale:**
- Simpler (no repo name parsing)
- More robust (cleans up ALL stale worktrees)
- Already exists (IDXCLEAN project built it)
- Minimal performance impact (<1 second difference)

### 2. Direct Spawn vs Daemon Client

**Decision:** Use direct binary spawn with `spawnSync`

**Rationale:**
- Simpler (no daemon lifecycle management)
- Adequate performance (cleanup is infrequent)
- Follows existing pattern (`WorktreeService.runMaproomScan()`)

### 3. Safe Delete vs Force Delete

**Decision:** Use `git branch -d` (safe delete, requires merge)

**Rationale:**
- Safe by default (prevents loss of unmerged work)
- Matches `worktree merge` behavior
- Can add `--force` flag later if needed

### 4. Opt-Out vs Opt-In Flags

**Decision:** Use opt-out flags (`--keep-branch`, `--keep-maproom`)

**Rationale:**
- Better UX (complete cleanup is what users want)
- "Clean" implies "remove everything"
- Old behavior available via flags if needed

## Implementation Phases

### Phase 1: Binary Resolution (Tickets 1001-1003)

- Implement `findMaproomBinary()` utility
- Create `cleanMaproomRecords()` helper function
- Add unit tests for binary discovery

### Phase 2: Command Integration (Tickets 2001-2004)

- Add `--keep-branch` and `--keep-maproom` flags
- Integrate maproom cleanup in clean command
- Integrate branch deletion using `GitMergeService`
- Add error handling and logging

### Phase 3: Testing and Documentation (Tickets 3001-3003)

- Add integration tests for complete cleanup workflow
- Add failure scenario tests (binary missing, branch protected)
- Update README documentation

## Dependencies

**MRBIN (binary resolution logic):**
- NOT a blocking dependency
- This project copies code from maproom-mcp for MVP
- Creates temporary code duplication with planned consolidation
- Follow-up ticket will be created after MRBIN delivers to use shared utility

**WTPATH (path utilities):**
- Optional - uses existing if available

## Acceptance Criteria

- [x] `clean` command deletes directory (existing)
- [x] `clean` command deletes git worktree metadata (existing)
- [x] `clean` command deletes git branch (new)
- [x] `clean` command deletes maproom records (new)
- [x] Cleanup succeeds even if maproom unavailable (best-effort)
- [x] Clear logging for each cleanup step
- [x] `--keep-branch` flag preserves branch
- [x] Tests cover failure scenarios

## Technical Approach

**Enhanced clean command flow:**

```typescript
worktree.command('clean')
  .option('--keep-branch', 'Keep git branch after removing worktree')
  .option('--keep-maproom', 'Skip maproom database cleanup')
  .action(async (selector, opts) => {
    const branch = matches[0].branch  // Get BEFORE removal

    // EXISTING: Remove git worktree + directory
    await wt.removeWorktree(targetPath)
    removeDirSync(targetPath)

    // NEW: Clean maproom records (best-effort)
    if (!opts.keepMaproom) {
      try {
        await cleanMaproomRecords()  // Calls db cleanup-stale
      } catch (err) {
        logger.warn('Could not clean maproom:', err.message)
      }
    }

    // NEW: Delete git branch (best-effort)
    if (branch && !opts.keepBranch) {
      try {
        await mergeService.deleteBranch(branch)  // git branch -d
        logger.success(`Deleted branch ${branch}`)
      } catch (err) {
        logger.warn(`Could not delete branch:`, err.message)
      }
    }
  })
```

**Key components:**

1. **Binary discovery** (`packages/cli/src/utils/maproom-binary.ts`)
   - Multi-strategy fallback (env var, packaged, dev builds, PATH)
   - Copied from `packages/maproom-mcp/src/utils/process.ts`

2. **Maproom cleanup** (helper function in command or WorktreeService)
   - Calls `crewchief-maproom db cleanup-stale --confirm`
   - Handles exit codes: 0 (success), 1 (error), 2 (no stale worktrees)

3. **Branch deletion** (uses existing `GitMergeService.deleteBranch()`)
   - Already exists and tested in `worktree merge` command
   - Uses `git branch -d` (safe delete, requires merge)

## Testing Strategy

**Unit tests:**
- Binary discovery (all fallback strategies)
- Maproom cleanup (success, failure, binary missing)
- Command flags (--keep-branch, --keep-maproom)

**Integration tests:**
- Complete cleanup workflow (all steps succeed)
- Graceful degradation (maproom binary missing)
- Branch protection (branch not fully merged)

**Manual tests:**
- Cross-platform verification (macOS, Linux, Windows)
- Real worktrees and maproom database
- Performance (should complete in <5 seconds)

## Security Assessment

**Risk level:** LOW

**No new security risks introduced:**
- No command injection (parameterized commands)
- No path traversal (paths validated and canonicalized)
- No privilege escalation (runs with user permissions)
- Follows existing secure patterns (`worktree merge`, `worktree create`)

**Ship confidently:** Multiple safeguards, graceful degradation, clear error messages.

## Breaking Changes

**Non-breaking enhancement:**
- Adds new cleanup steps without changing command interface
- Existing flags (`--stale`, `--all`, `--keep-dir`) unchanged
- New flags (`--keep-branch`, `--keep-maproom`) opt-out of new behavior

**New default behavior:**
- Branches deleted by default (use `--keep-branch` to preserve)
- Maproom records cleaned by default (use `--keep-maproom` to skip)

Users who want old behavior can use opt-out flags.

## Edge Cases

**Handled gracefully:**

1. **Maproom binary not found**
   - Warning logged: "Could not clean maproom records"
   - Helpful message: "Run manually: crewchief-maproom db cleanup-stale --confirm"
   - Cleanup continues

2. **Branch not fully merged**
   - Warning logged: "Could not delete branch (not fully merged)"
   - Helpful message: "Delete manually: git branch -D <branch>"
   - Cleanup continues

3. **Branch doesn't exist**
   - Warning logged (or no-op)
   - Cleanup continues

4. **Database locked**
   - Maproom handles SQLite locking internally
   - Warning logged if fails
   - Cleanup continues

5. **Branch checked out elsewhere**
   - Error from git
   - Warning logged
   - Cleanup continues

6. **Concurrent cleanup operations**
   - NOT SUPPORTED: Running `worktree clean` in multiple terminals simultaneously may cause race conditions
   - Recommendation: Run cleanup commands sequentially
   - Acceptable risk: Uncommon scenario, no data loss (just confusing errors)

## Timeline

| Phase | Duration | Tickets |
|-------|----------|---------|
| Phase 1: Binary Resolution | 0.5-1 day | WTCLEAN-1001 to 1003 |
| Phase 2: Command Integration | 1.5-2 days | WTCLEAN-2001 to 2004c (split error handling) |
| Phase 3: Testing & Docs | 0.5-1 day | WTCLEAN-3001 to 3003 |
| **Total** | **2.5-4 days** | **11 tickets** |

**Timeline includes buffer for:**
- Cross-platform testing (especially Windows)
- Integration complexity with multiple cleanup steps
- Branch extraction sequencing verification (critical)

## Next Steps

1. **Review planning documents** - Run `/review-project WTCLEAN` to validate planning
2. **Create tickets** - Run `/create-project-tickets WTCLEAN` to generate tickets
3. **Execute project** - Run `/work-on-project WTCLEAN` to implement all tickets
4. **Manual testing** - Verify on local machine before merge

## References

- **Similar command:** `worktree merge` already demonstrates complete cleanup
- **Maproom cleanup:** IDXCLEAN project (archived) built the infrastructure
- **Binary resolution:** `WorktreeService.runMaproomScan()` shows the pattern
- **Branch deletion:** `GitMergeService.deleteBranch()` already exists and tested

## Success Metrics

**Must-have for MVP:**
- ✅ Single command removes all worktree artifacts
- ✅ Graceful degradation when tools missing
- ✅ Clear feedback for each cleanup step
- ✅ No breaking changes to existing behavior
- ✅ Tests cover critical paths and failure scenarios

**Nice-to-have (future):**
- `--force` flag for unmerged branch deletion
- `--dry-run` flag to preview cleanup
- Daemon client support for faster cleanup
- Parallel execution of cleanup steps

## Notes

**Why this matters:**
- Database bloat affects search quality (duplicate results from stale branches)
- Manual cleanup is error-prone (easy to forget steps)
- "Clean" should mean "completely clean" (user expectation)

**Key insight:**
The `worktree merge` command already does complete cleanup. We're adapting its proven pattern to `worktree clean`, ensuring consistency and reliability.

**Philosophy:**
Ship value, not ceremonies. Focus on the 80% case (complete cleanup), handle failures gracefully, and let users opt-out if they need old behavior.
