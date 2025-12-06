# Architecture: Enhanced Worktree Clean

## Overview

Extend `crewchief worktree clean` to perform **complete cleanup** by adding two new steps after existing directory/metadata removal:

1. **Maproom database cleanup** - Call `crewchief-maproom db cleanup-stale` to remove stale worktree records
2. **Git branch deletion** - Call `GitMergeService.deleteBranch()` to remove the branch

**MVP Philosophy:** Reuse existing components, add minimal new code, handle failures gracefully.

## Design Decisions

### Decision 1: Batch Cleanup vs Targeted Deletion

**Context:** Need to remove maproom database records for deleted worktree

**Options:**
- A) Targeted deletion: Parse repo/worktree names, call specific delete API
- B) Batch cleanup: Call `db cleanup-stale` to clean all stale worktrees

**Decision:** Use batch cleanup (`db cleanup-stale --confirm`)

**Rationale:**
- **Simpler:** No need to parse repo names or worktree names
- **More robust:** Cleans up ANY stale worktrees, not just the one we removed
- **Already exists:** The IDXCLEAN project built this infrastructure
- **Auto-detection:** Maproom detects stale worktrees by checking if `abs_path` exists
- **Best-effort:** If binary missing, cleanup gracefully degrades

**Tradeoff:** Slightly slower (scans all worktrees), but difference is negligible (<1 second).

### Decision 2: Direct Binary Spawn vs Daemon Client

**Context:** Need to call maproom binary from TypeScript

**Options:**
- A) Use daemon client (JSON-RPC over stdio)
- B) Direct spawn (`spawnSync('crewchief-maproom', ...)`)

**Decision:** Use direct binary spawn

**Rationale:**
- **Simpler:** No daemon lifecycle management
- **Less coupling:** Don't depend on daemon being running
- **Adequate performance:** Cleanup is infrequent, 1-2 second startup acceptable
- **Existing pattern:** `WorktreeService.runMaproomScan()` uses this pattern

**Future enhancement:** If cleanup becomes performance-critical, switch to daemon client.

### Decision 3: Safe Delete vs Force Delete

**Context:** Branch deletion can fail if branch not merged

**Options:**
- A) Always use `git branch -d` (safe, requires merge)
- B) Always use `git branch -D` (force, dangerous)
- C) Add `--force` flag, default to safe

**Decision:** Always use `git branch -d` (safe delete)

**Rationale:**
- **Safe by default:** Prevents accidental loss of unmerged work
- **Matches merge command:** Consistency with `worktree merge`
- **Clear feedback:** Error message guides user to manual cleanup if needed
- **MVP scope:** Can add `--force` flag in future if needed

**Tradeoff:** Users with unmerged branches must manually delete with `git branch -D`.

### Decision 4: Opt-Out Flags vs Opt-In Flags

**Context:** Need to allow users to control new behavior

**Options:**
- A) Opt-out flags: `--keep-branch`, `--keep-maproom` (new behavior by default)
- B) Opt-in flags: `--delete-branch`, `--delete-maproom` (old behavior by default)

**Decision:** Use opt-out flags (`--keep-branch`, `--keep-maproom`)

**Rationale:**
- **Better UX:** Complete cleanup is what users want by default
- **Non-breaking:** Still technically non-breaking (adds functionality, doesn't change existing flags)
- **Matches expectations:** "Clean" implies "remove everything"
- **Progressive enhancement:** Old behavior available via flags if needed

### Decision 5: Error Handling Strategy

**Context:** Cleanup steps can fail independently

**Options:**
- A) Fail fast: Stop on first error
- B) Continue on error: Log warnings, complete all steps
- C) Transactional: Rollback if any step fails

**Decision:** Continue on error (best-effort cleanup)

**Rationale:**
- **Pragmatic:** Partial cleanup better than no cleanup
- **User-friendly:** Clear logs show what succeeded/failed
- **Matches merge command:** Same pattern as `worktree merge`
- **No rollback needed:** Git/filesystem operations are idempotent

## Technology Choices

| Component | Choice | Rationale |
|-----------|--------|-----------|
| **Binary discovery** | `findMaproomBinary()` from maproom-mcp | Multi-strategy fallback (env var, packaged, dev builds, PATH) |
| **Process spawning** | Node `spawnSync` | Synchronous, simple error handling |
| **Git operations** | `simple-git` via `GitMergeService` | Already used by merge command |
| **Logging** | Existing `logger` utility | Consistent with rest of CLI |
| **CLI flags** | Commander.js `.option()` | Matches existing command structure |

## Component Design

### Enhanced Clean Command

**File:** `packages/cli/src/cli/worktree.ts`

**Current flow:**
```typescript
worktree.command('clean')
  .action(async (selector, opts) => {
    // 1. Resolve selector to worktree path
    // 2. Remove git worktree metadata
    // 3. Remove directory
  })
```

**New flow:**
```typescript
worktree.command('clean')
  .option('--keep-branch', 'Keep git branch after removing worktree')
  .option('--keep-maproom', 'Skip maproom database cleanup')
  .action(async (selector, opts) => {
    // ⚠️ CRITICAL: Get branch name BEFORE removing worktree
    // Once worktree is removed, git metadata is gone and we can't determine the branch
    const branch = matches[0].branch

    // EXISTING: Remove git worktree metadata
    await wt.removeWorktree(targetPath)

    // EXISTING: Remove directory
    if (!opts.keepDir) {
      removeDirSync(targetPath)
    }
    logger.success(`Removed worktree ${targetPath}`)

    // NEW: Clean maproom database records (best-effort)
    if (!opts.keepMaproom) {
      try {
        await cleanMaproomRecords()
      } catch (err) {
        logger.warn('Could not clean maproom records:', err.message)
        logger.info('Run manually: crewchief-maproom db cleanup-stale --confirm')
      }
    }

    // NEW: Delete git branch (best-effort)
    if (branch && !opts.keepBranch) {
      try {
        const mergeService = new GitMergeService()
        await mergeService.deleteBranch(branch)
        logger.success(`Deleted branch ${branch}`)
      } catch (err) {
        logger.warn(`Could not delete branch ${branch}:`, err.message)
        logger.info('Delete manually: git branch -d', branch)
      }
    }
  })
```

### Maproom Cleanup Function

**File:** `packages/cli/src/git/worktrees.ts` (new method) or inline in command

**Signature:**
```typescript
async function cleanMaproomRecords(): Promise<void>
```

**Implementation:**
```typescript
async function cleanMaproomRecords(): Promise<void> {
  // Find maproom binary (reuse pattern from runMaproomScan)
  const maproomBin = findMaproomBinary()

  if (!maproomBin) {
    throw new Error('Maproom binary not found')
  }

  // Run cleanup command
  const result = spawnSync(maproomBin, ['db', 'cleanup-stale', '--confirm'], {
    encoding: 'utf8',
    stdio: ['pipe', 'pipe', 'pipe'],
  })

  if (result.status !== 0 && result.status !== 2) {
    // Exit code 2 means "no stale worktrees", which is fine
    const errorMsg = result.stderr || result.stdout || 'Unknown error'
    throw new Error(errorMsg.split('\n')[0])
  }

  // Parse output for user feedback
  const output = result.stdout
  if (output.includes('Deleted')) {
    logger.info('Cleaned maproom database records')
  }
}
```

**Alternative:** Copy `findMaproomBinary()` from maproom-mcp to packages/cli/src/utils/ to avoid cross-package dependencies.

**Note on code duplication:** This project copies binary resolution logic from maproom-mcp as a pragmatic MVP decision. This is temporary code duplication that will be consolidated when the MRBIN (Binary Resolution) project completes. A follow-up ticket will be created to replace this copied code with the shared utility from MRBIN.

### Binary Resolution

**File:** `packages/cli/src/utils/maproom-binary.ts` (new file)

**Purpose:** Find crewchief-maproom binary using multiple strategies

**Implementation:** Copy `findMaproomBinary()` from `packages/maproom-mcp/src/utils/process.ts`:

```typescript
export function findMaproomBinary(): string | null {
  // Strategy 1: Environment variable
  if (process.env.CREWCHIEF_MAPROOM_BIN) {
    const binPath = process.env.CREWCHIEF_MAPROOM_BIN
    if (fs.existsSync(binPath)) return binPath
  }

  // Strategy 2: Platform-specific packaged binary
  const execName = process.platform === 'win32' ? 'crewchief-maproom.exe' : 'crewchief-maproom'
  const packagedPath = path.join(__dirname, '..', 'bin', `${process.platform}-${process.arch}`, execName)
  if (fs.existsSync(packagedPath)) return packagedPath

  // Strategy 3: Development builds
  const devPaths = [
    './target/release/crewchief-maproom',
    '../../../crates/maproom/target/release/crewchief-maproom',
  ]
  for (const devPath of devPaths) {
    if (fs.existsSync(devPath)) return path.resolve(devPath)
  }

  // Strategy 4: System PATH (will be tried by spawnSync)
  return 'crewchief-maproom'  // Fallback to command name
}
```

## Data Flow

```
User runs: crewchief worktree clean feature-branch
           │
           ├─> Resolve selector to worktree path + branch name
           │
           ├─> Remove git worktree metadata
           │   └─> git worktree remove <path>
           │
           ├─> Remove directory
           │   └─> removeDirSync(<path>)
           │
           ├─> Clean maproom records (NEW)
           │   ├─> findMaproomBinary()
           │   ├─> spawnSync('crewchief-maproom', ['db', 'cleanup-stale', '--confirm'])
           │   └─> Parse output, log result
           │
           └─> Delete git branch (NEW)
               ├─> GitMergeService.deleteBranch(branch)
               └─> git branch -d <branch>

Each step logs success/warning/error
Failures don't block subsequent steps
```

## Integration Points

### With Existing Code

1. **GitMergeService** (`packages/cli/src/git/merge.ts`)
   - Import and use `deleteBranch()` method
   - Already tested and working in merge command

2. **WorktreeService** (`packages/cli/src/git/worktrees.ts`)
   - Use existing `removeWorktree()` method
   - Follows same pattern as `runMaproomScan()`

3. **Logger** (`packages/cli/src/utils/logger.ts`)
   - Use `logger.success()`, `logger.warn()`, `logger.info()`
   - Consistent with rest of CLI

4. **Commander.js** (`packages/cli/src/cli/worktree.ts`)
   - Add `.option()` calls for new flags
   - Follow existing command structure

### With Maproom Binary

**Interface:** CLI command invocation
```bash
crewchief-maproom db cleanup-stale --confirm
```

**Exit codes:**
- `0` - Success (deleted records)
- `1` - Error (database connection failed)
- `2` - No stale worktrees found (not an error)

**Output parsing:** Look for "Deleted" in stdout to confirm success

### With Git

**Branch deletion:**
```typescript
await this.git.raw(['branch', '-d', branch])
```

**Error cases:**
- Branch not fully merged → Error thrown
- Branch doesn't exist → Error thrown
- Branch checked out elsewhere → Error thrown

**Error handling:** Catch and log warning, don't fail cleanup

## Performance Considerations

### Expected Performance

| Operation | Duration | Notes |
|-----------|----------|-------|
| Directory removal | <100ms | Depends on file count |
| Git metadata removal | <100ms | Git command overhead |
| Maproom cleanup | 1-3s | Cold start + database scan |
| Branch deletion | <100ms | Git command overhead |
| **Total** | **1-5s** | Acceptable for cleanup operation |

### Optimization Opportunities (Future)

1. **Use daemon client** - 20-50x faster than cold start (225ms → 10ms)
2. **Parallel execution** - Run maproom and branch deletion concurrently
3. **Targeted deletion** - Delete specific worktree instead of scanning all

**MVP Decision:** Accept 1-5 second cleanup time. Optimize only if users complain.

## Error Recovery

### Failure Scenarios

| Scenario | Behavior | User Action |
|----------|----------|-------------|
| Maproom binary not found | Warn, continue | Run `crewchief-maproom db cleanup-stale` manually |
| Database locked | Warn, continue | Retry later or wait for lock to release |
| Branch not merged | Warn, continue | Run `git branch -D <branch>` manually if intended |
| Branch checked out | Warn, continue | Switch to another branch first |
| Branch doesn't exist | Warn, continue | No action needed (already gone) |

### Logging Strategy

```typescript
// Success (green checkmark)
logger.success(`Removed worktree ${targetPath}`)
logger.success(`Deleted branch ${branch}`)

// Warning (yellow, non-fatal)
logger.warn('Could not clean maproom records:', err.message)
logger.info('Run manually: crewchief-maproom db cleanup-stale --confirm')

// Error (red, fatal)
logger.error('Failed to remove worktree:', err)
process.exitCode = 1
```

### Manual Recovery Procedures

When cleanup steps fail, users need clear guidance on how to complete the cleanup manually. Each failure scenario provides specific commands:

**Scenario 1: Maproom binary not found**
```
Warning: Could not clean maproom records: Binary not found
Manual recovery: crewchief-maproom db cleanup-stale --confirm
```
**User action:** Install crewchief-maproom or ensure it's in PATH, then run the command.

**Scenario 2: Maproom database locked**
```
Warning: Could not clean maproom records: Database is locked
Manual recovery: Wait for other maproom processes to complete, then run:
  crewchief-maproom db cleanup-stale --confirm
```
**User action:** Check for running maproom processes (`ps aux | grep maproom`), wait for completion, retry.

**Scenario 3: Branch not fully merged**
```
Warning: Could not delete branch feature-123: not fully merged
Manual recovery: Verify branch can be deleted safely, then run:
  git branch -D feature-123  (force delete, CAUTION: data loss possible)
```
**User action:** Review branch commits (`git log feature-123`), merge if needed, or force delete if certain.

**Scenario 4: Branch checked out in another worktree**
```
Warning: Could not delete branch feature-123: checked out in another worktree
Manual recovery: Switch other worktree to different branch, then run:
  git branch -d feature-123
```
**User action:** Find worktree with branch (`git worktree list`), switch branch there, retry deletion.

**Scenario 5: Branch doesn't exist**
```
Warning: Could not delete branch feature-123: branch not found
Manual recovery: No action needed (branch already gone)
```
**User action:** None - this is informational only.

### Concurrent Operations

**Concurrent cleanup is NOT a supported use case.**

**Scenario:** User runs `worktree clean` in multiple terminals simultaneously.

**Behavior:** Undefined - may result in:
- Race conditions on file system operations
- Database locking conflicts
- Inconsistent git state

**Mitigation:**
- Document this limitation in README
- No technical enforcement (lock files would add complexity)
- Acceptable risk (uncommon scenario, no data loss, just confusing errors)

**User guidance:** If concurrent cleanup is needed, run commands sequentially or target different worktrees.

## Maintainability

### Code Reuse

- **GitMergeService.deleteBranch()** - Already tested, no duplication
- **findMaproomBinary()** - Copy from maproom-mcp (avoid package dep)
- **spawnSync pattern** - Follow `runMaproomScan()` example
- **Error handling** - Follow `worktree merge` pattern

### Testing Strategy

- **Unit tests** - Mock `spawnSync`, test each step independently
- **Integration tests** - Create/clean worktrees, verify state
- **Failure tests** - Mock binary not found, branch protection, etc.

### Future Enhancements

**Possible additions (NOT in MVP):**
- `--force` flag for `git branch -D` (force delete unmerged branches)
- `--dry-run` flag to preview what will be deleted
- Targeted maproom deletion (delete specific repo/worktree)
- Daemon client support for faster cleanup
- Parallel execution of cleanup steps

**Extension points:**
- `cleanMaproomRecords()` can be enhanced to use daemon
- `deleteBranch()` can accept force flag
- Flags can be added without breaking changes

## Diagrams

### Component Interaction

```
┌─────────────────────────────────────────┐
│ worktree clean command                  │
│ (packages/cli/src/cli/worktree.ts)     │
└────────────┬────────────────────────────┘
             │
             ├──> WorktreeService.removeWorktree()
             │    (git worktree remove)
             │
             ├──> removeDirSync()
             │    (filesystem deletion)
             │
             ├──> cleanMaproomRecords()
             │    │
             │    ├──> findMaproomBinary()
             │    │    └──> Multi-strategy search
             │    │
             │    └──> spawnSync('crewchief-maproom', ['db', 'cleanup-stale'])
             │         └──> Rust binary
             │              └──> SQLite database
             │
             └──> GitMergeService.deleteBranch()
                  └──> git branch -d <branch>
```

### Decision Flow

```
User runs clean command
        │
        ├─> Selector provided?
        │   ├─> Yes: Find matching worktree
        │   └─> No: Use --all or --stale mode
        │
        ├─> --all mode?
        │   ├─> Yes: Process all non-current worktrees (loop)
        │   │   └─> For each worktree: Run full cleanup flow
        │   └─> No: Process single worktree
        │
        ├─> Directory exists?
        │   ├─> Yes: Remove worktree + directory
        │   └─> No: Remove metadata only
        │
        ├─> --keep-maproom flag?
        │   ├─> No: Try cleanup (best-effort)
        │   └─> Yes: Skip cleanup
        │
        └─> --keep-branch flag?
            ├─> No: Try delete (best-effort)
            └─> Yes: Skip deletion
```

**Note on --all mode:** When `--all` is used, the command loops through all non-current worktrees and applies the full cleanup flow to each. Branch deletion happens for each worktree unless `--keep-branch` is specified. If any branch deletion fails, a warning is logged but cleanup continues with remaining worktrees.

## Summary

The architecture extends `worktree clean` with two new steps: maproom database cleanup and git branch deletion. Both are best-effort (failures logged as warnings), maintaining the command's reliability while providing complete cleanup by default.

**Key principles:**
- **Reuse existing components** (GitMergeService, binary resolution pattern)
- **Graceful degradation** (maproom missing → warning, not error)
- **Clear feedback** (log each step's success/failure)
- **User control** (opt-out flags for new behavior)
- **MVP-focused** (solve 80% case, defer edge cases)

This design balances completeness with pragmatism, delivering immediate value without over-engineering.
