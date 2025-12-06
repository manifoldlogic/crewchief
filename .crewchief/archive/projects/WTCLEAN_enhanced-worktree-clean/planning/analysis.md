# Analysis: Enhanced Worktree Clean

## Problem Definition

The current `crewchief worktree clean` command performs **incomplete cleanup** when removing worktrees:

**Current behavior (lines 172-176 in `packages/cli/src/cli/worktree.ts`):**
```typescript
await wt.removeWorktree(targetPath)  // Removes git worktree metadata
if (!opts.keepDir) {
  removeDirSync(targetPath)          // Removes directory
}
```

**What's missing:**
1. **Maproom database records** - Worktree entries remain in `~/.maproom/maproom.db`, causing:
   - Duplicate search results from deleted branches
   - Database bloat with stale entries
   - Confusion about which worktrees are actually active
2. **Git branch** - The feature branch remains in `.git/refs/heads/`, requiring manual deletion

**Impact:** Developers must manually run `crewchief-maproom db cleanup-stale` and `git branch -d <branch>` after every worktree cleanup, which is error-prone and breaks the "single command" user experience.

## Context

### How `worktree merge` Handles Cleanup

The `worktree merge` command (lines 552-572) demonstrates **complete cleanup**:
```typescript
// Remove the worktree first (before deleting the branch)
await wt.removeWorktree(worktreePath)
removeDirSync(worktreePath)
logger.success(`Removed worktree at ${worktreePath}`)

// Now delete the worktree branch
try {
  await mergeService.deleteBranch(worktreeBranch)
  logger.success(`Deleted branch ${worktreeBranch}`)
} catch (error) {
  logger.warn(`Could not delete branch ${worktreeBranch}: ${error}`)
}

// Run git worktree prune
await wt.pruneWorktrees({ mode: 'stale' })
```

This pattern should be replicated in `worktree clean`.

### Maproom Cleanup Capabilities

**Database cleanup module exists** (`crates/maproom/src/db/cleanup.rs`):
- `StaleWorktreeDetector::detect_stale_worktrees()` - Finds worktrees where `abs_path` doesn't exist
- `WorktreeCleaner::cleanup_stale_worktrees()` - Deletes worktree DB records and associated chunks
- Exposed via CLI: `crewchief-maproom db cleanup-stale --confirm`

**Binary resolution available** (`packages/maproom-mcp/src/utils/process.ts`):
- `findMaproomBinary()` - Multi-strategy binary discovery:
  1. `CREWCHIEF_MAPROOM_BIN` environment variable
  2. Platform-specific packaged binary in `bin/<platform>/`
  3. Development build paths
  4. System PATH fallback

**Problem:** The TypeScript CLI doesn't currently call maproom cleanup when removing worktrees.

### Git Branch Deletion

**Existing implementation** (`packages/cli/src/git/merge.ts`, line 137):
```typescript
async deleteBranch(branch: string, force: boolean = false): Promise<void> {
  const flag = force ? '-D' : '-d'
  await this.git.raw(['branch', flag, branch])
}
```

This is already available and used by `worktree merge` but not by `worktree clean`.

## Existing Solutions

### Industry Patterns

**Git worktree best practices:**
- `git worktree remove <path>` - Removes worktree and metadata
- `git branch -d <branch>` - Deletes branch (safe, requires merge)
- `git branch -D <branch>` - Force delete (unsafe)

Most tools leave branch deletion to users, but CrewChief can do better.

### Codebase Patterns

**IDXCLEAN project (archived)** - Built the cleanup infrastructure:
- Stale worktree detection via filesystem validation
- Safe batch deletion with transaction rollback
- CLI command: `db cleanup-stale --confirm`
- Exit codes: 0 (success), 1 (error), 2 (nothing to clean)

**Current `worktree merge` command** - Reference implementation:
- Removes worktree directory and git metadata
- Deletes branch after worktree removal
- Handles errors gracefully with warnings
- Runs `git worktree prune` for cleanup

## Current State

**WorktreeService.removeWorktree()** (packages/cli/src/git/worktrees.ts):
- Calls `git worktree remove <path>`
- Does NOT delete branches
- Does NOT clean maproom database

**CLI clean command** (packages/cli/src/cli/worktree.ts, lines 112-187):
- Supports selector matching (branch name, path, basename)
- Prevents removing current working directory
- Optionally keeps directory with `--keep-dir`
- Has `--all` mode to remove all non-current worktrees
- Has `--stale` mode to prune only stale metadata

**Missing integration:**
- No call to maproom cleanup
- No branch deletion
- No flags to control new behavior

## Research Findings

### Repository Name Detection

**Finding:** Maproom `scan` command auto-detects repository from git config
- Uses `git config --get remote.origin.url`
- Parses URL to extract repo name
- Handles both HTTPS and SSH URLs

**Implication:** We can pass worktree path to maproom and let it auto-detect repo, OR we can extract repo name in TypeScript and pass it explicitly.

**Recommendation:** Call `crewchief-maproom db cleanup-stale --confirm` after directory removal. This detects ALL stale worktrees (not just the one we removed) and cleans them in batch.

**CRITICAL SEQUENCING REQUIREMENT:** Branch name MUST be extracted from worktree metadata BEFORE calling `removeWorktree()`, as the git metadata needed to determine the branch will be deleted by the removal operation. This is easy to miss during implementation - extraction must happen first in the code sequence.

### Binary Execution Patterns

**Finding:** Three approaches for calling maproom:
1. **Daemon client** - JSON-RPC over stdio, 20-50x faster
2. **Direct spawn** - `spawnSync('crewchief-maproom', ['db', 'cleanup-stale'])`
3. **Shell wrapper** - `sh -c "crewchief-maproom ..."`

**Recommendation:** Use direct spawn with `findMaproomBinary()` fallback. Cleanup is infrequent, so daemon overhead not justified.

### Failure Modes

**Branch deletion failures:**
- Branch not fully merged → `git branch -d` fails
- Branch checked out elsewhere → Error
- Branch doesn't exist → Error

**Maproom cleanup failures:**
- Binary not found → Silently skip
- Database locked → Transient, retry possible
- Worktree not in database → No-op (exit code 2)

**Best practice:** Fail gracefully, log warnings, don't block cleanup.

## Constraints

### Technical Constraints

1. **Maproom binary availability** - May not be installed or in PATH
   - Solution: Best-effort cleanup with graceful degradation

2. **Repository name detection** - Need to determine repo name from worktree path
   - The maproom `scan` command auto-detects repo from git config
   - Can use `git config --get remote.origin.url` to extract repo name

3. **Branch protection** - Cannot delete branch if:
   - Currently checked out in another worktree
   - Not fully merged (requires `--force` flag)

4. **Cross-package communication** - TypeScript CLI → Rust binary
   - Use `spawnSync` with `findMaproomBinary()` for discovery
   - Handle ENOENT errors gracefully

### Business Constraints

1. **Backward compatibility** - Cannot break existing `clean` command behavior
   - Solution: Add new cleanup steps without changing interface
   - Add opt-out flags: `--keep-branch`, `--keep-maproom`

2. **User expectations** - Developers expect atomic cleanup
   - All-or-nothing is wrong; best-effort is right
   - Clear logging for each step so users know what happened

3. **MVP scope** - Must deliver value quickly
   - Focus on happy path first
   - Edge cases can be handled in future iterations

### Time Constraints

- Estimated effort: M (2-3 days)
- Must integrate with existing worktree infrastructure
- Tests should cover failure scenarios (maproom missing, branch protected)

## Success Criteria

### Functional Requirements

1. **Complete cleanup by default:**
   - ✅ Remove directory (existing)
   - ✅ Remove git worktree metadata (existing)
   - ✅ Delete git branch (new)
   - ✅ Delete maproom database records (new)

2. **Graceful degradation:**
   - ✅ Cleanup succeeds even if maproom unavailable
   - ✅ Cleanup succeeds even if branch deletion fails
   - ✅ Clear warnings logged for each failure

3. **User control:**
   - ✅ `--keep-branch` flag to preserve branch
   - ✅ `--keep-maproom` flag to skip database cleanup (for testing)

4. **Clear feedback:**
   - ✅ Log message for each cleanup step
   - ✅ Distinguish between success, warning, and error
   - ✅ Helpful messages guide users on manual cleanup if needed

### Non-Functional Requirements

1. **Performance:** Cleanup should complete in <5 seconds for typical worktree
2. **Reliability:** Individual step failures don't prevent subsequent steps
3. **Testability:** Mock maproom binary, test branch deletion edge cases
4. **Maintainability:** Reuse existing `GitMergeService.deleteBranch()` and binary resolution

### Acceptance Criteria

From project summary:
- `clean` command deletes directory (existing)
- `clean` command deletes git worktree metadata (existing)
- `clean` command deletes git branch (new)
- `clean` command deletes maproom records (new)
- Cleanup succeeds even if maproom unavailable (best-effort)
- Clear logging for each cleanup step
- `--keep-branch` flag preserves branch
- Tests cover failure scenarios

## Assumptions

1. Worktrees are named after their branches (typical CrewChief usage)
2. Repository name can be derived from git remote URL or auto-detected by maproom
3. Maproom database is at default location (`~/.maproom/maproom.db`)
4. Users want complete cleanup by default (branches and database records deleted)
5. Graceful degradation is acceptable (e.g., if maproom binary missing)
6. `db cleanup-stale` is sufficient (batch cleanup of all stale worktrees, not targeted deletion)

## Risks and Mitigations

### High Priority

**Risk:** Repository name detection fails
**Impact:** Cannot clean maproom records
**Mitigation:**
- Use `db cleanup-stale` which auto-detects stale worktrees
- Log warning and continue if cleanup fails
- User can manually run command later

**Risk:** Branch is checked out in another worktree
**Impact:** `git branch -d` fails, leaving branch
**Mitigation:**
- Catch error and log helpful message
- Don't fail entire cleanup operation
- User can manually delete later

### Medium Priority

**Risk:** Maproom binary not found
**Impact:** Database records not cleaned
**Mitigation:**
- Use `findMaproomBinary()` with multiple fallback strategies
- Log warning, continue with rest of cleanup
- User can manually run `db cleanup-stale` later

**Risk:** Database is locked (another process using it)
**Impact:** Maproom cleanup fails
**Mitigation:**
- Maproom handles SQLite locking internally
- Log warning if cleanup fails
- User can retry later

### Low Priority

**Risk:** User accidentally deletes wrong worktree
**Impact:** Data loss
**Mitigation:**
- Existing selector disambiguation prevents this
- Branch deletion requires merge (use `-d` not `-D`)
- User can recover from git reflog if needed

## Summary

The current `worktree clean` command is incomplete, leaving behind git branches and maproom database records. This causes database bloat, duplicate search results, and requires manual cleanup steps.

**Solution approach:**
1. Add maproom cleanup call (best-effort, using binary resolution)
2. Add git branch deletion (using existing `GitMergeService.deleteBranch()`)
3. Add `--keep-branch` and `--keep-maproom` flags for user control
4. Ensure graceful degradation with clear logging

**Key insight:** The `worktree merge` command already demonstrates complete cleanup. We can adapt its pattern to `worktree clean`, making cleanup atomic and user-friendly while maintaining backward compatibility through opt-out flags.
