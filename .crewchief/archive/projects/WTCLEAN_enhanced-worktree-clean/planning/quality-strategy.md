# Quality Strategy: Enhanced Worktree Clean

## Testing Philosophy

**Test for confidence, not coverage.**

This project extends an existing command with new cleanup steps. Our testing strategy focuses on:

1. **Happy path works** - Complete cleanup succeeds when all tools available
2. **Graceful degradation** - Partial cleanup succeeds when tools missing
3. **No regressions** - Existing clean behavior unchanged
4. **Clear feedback** - Failures logged appropriately

**What we DON'T test:**
- Internal implementation details that may change
- Third-party libraries (git, maproom binary internals)
- Every possible edge case (diminishing returns)

**Pragmatic approach:**
- Mock external dependencies (file system, git, maproom binary)
- Real integration tests for critical paths
- Manual testing for cross-platform verification

## Test Types

### Unit Tests

**Scope:**
- Binary discovery logic (`findMaproomBinary()`)
- Maproom cleanup helper (`cleanMaproomRecords()`)
- Error handling for each cleanup step
- Flag parsing (--keep-branch, --keep-maproom)

**Tools:**
- Vitest (existing test framework)
- Mock `fs.existsSync`, `spawnSync`, `GitMergeService`

**Coverage Target:** 80% of new code (pragmatic, not ceremonial)

**Key tests:**
```typescript
describe('findMaproomBinary', () => {
  it('finds binary from CREWCHIEF_MAPROOM_BIN env var')
  it('finds packaged binary in bin/ directory')
  it('finds dev build in target/release')
  it('falls back to command name for PATH lookup')
  it('returns null when binary not found')
  it('handles Windows .exe extension correctly', () => {
    // Mock process.platform = 'win32'
    // Verify binary path includes .exe
    // Verify spawnSync works with .exe path
  })
})

describe('cleanMaproomRecords', () => {
  it('calls db cleanup-stale --confirm')
  it('handles exit code 0 (success)')
  it('handles exit code 2 (no stale worktrees)')
  it('throws on exit code 1 (error)')
  it('throws when binary not found')
})

describe('worktree clean command', () => {
  it('calls maproom cleanup when --keep-maproom not set')
  it('skips maproom cleanup when --keep-maproom set')
  it('calls branch deletion when --keep-branch not set')
  it('skips branch deletion when --keep-branch set')
  it('logs warning if maproom cleanup fails')
  it('logs warning if branch deletion fails')
  it('does not fail cleanup if maproom/branch steps fail')
})
```

### Integration Tests

**Scope:**
- Complete cleanup workflow (directory + metadata + branch + maproom)
- Flag behavior (--keep-branch, --keep-maproom)
- Error scenarios (binary missing, branch protected)

**Approach:**
- Use temp git repositories
- Create real worktrees
- Call command and verify state after
- Check git state (branches, worktrees)
- Check file system state (directories)

**Tools:**
- Vitest integration tests
- Temp directory helpers
- Real git commands (not mocked)

**Key tests:**
```typescript
describe('worktree clean integration', () => {
  it('removes directory, metadata, branch when all available')
  it('warns but continues when maproom binary missing')
  it('warns but continues when branch deletion fails')
  it('preserves branch when --keep-branch set')
  it('works in --all mode (removes all non-current worktrees)', () => {
    // Create multiple worktrees
    // Run clean --all
    // Verify all non-current worktrees removed
    // Verify all branches deleted (unless --keep-branch)
    // Verify maproom cleanup ran once after all removals
  })
  it('extracts branch name before worktree removal', () => {
    // Critical sequencing test
    // Verify branch deletion works (requires branch name was captured)
    // Verify error if branch name not available after removal
  })
})
```

**Note:** Maproom database integration requires maproom binary to be available. These tests may be skipped in CI if binary not found.

### End-to-End Tests

**Scope:** Manual testing only (no automated E2E)

**Critical paths:**
1. **Complete cleanup** - All steps succeed
2. **Graceful degradation** - Maproom binary missing
3. **Branch protection** - Branch not fully merged
4. **Cross-platform** - Works on macOS, Linux, Windows

**Approach:**
Manual verification on dev machines before merge:

```bash
# Test 1: Complete cleanup
crewchief worktree create test-feature
# (index if maproom available)
crewchief worktree clean test-feature
# Verify: directory gone, branch gone, maproom records gone

# Test 2: Binary missing
mv bin/crewchief-maproom /tmp/
crewchief worktree clean test-feature
# Verify: warning logged, directory/branch still cleaned

# Test 3: Branch not merged
crewchief worktree create unmerged
cd .crewchief/worktrees/unmerged
echo "test" > file.txt && git add file.txt && git commit -m "test"
cd ../../..
crewchief worktree clean unmerged
# Verify: warning logged, directory/maproom still cleaned, branch remains

# Test 4: Keep flags
crewchief worktree clean test --keep-branch
crewchief worktree clean test --keep-maproom
# Verify: branch/maproom preserved
```

## Critical Paths

The following paths MUST be tested (either automated or manual):

1. **Happy path: Complete cleanup**
   - Directory removed
   - Git worktree metadata removed
   - Git branch deleted
   - Maproom database records cleaned
   - Success messages logged

2. **Partial failure: Maproom binary missing**
   - Directory removed
   - Git worktree metadata removed
   - Git branch deleted
   - Warning logged for maproom
   - Helpful message about manual cleanup

3. **Partial failure: Branch not merged**
   - Directory removed
   - Git worktree metadata removed
   - Maproom database records cleaned
   - Warning logged for branch
   - Helpful message about manual deletion

4. **Opt-out flags work**
   - `--keep-branch` preserves branch
   - `--keep-maproom` skips database cleanup
   - Directory still removed

5. **No regression: Existing behavior**
   - `--stale` mode works
   - `--all` mode works
   - `--keep-dir` mode works
   - Selector matching unchanged

## Test Data Strategy

### Mock Data

**Binary paths:**
- Valid path: `/fake/path/to/crewchief-maproom`
- Missing path: `/nonexistent/binary`
- System PATH: `crewchief-maproom` (command name)

**Spawn results:**
- Success: `{ status: 0, stdout: 'Deleted 1 worktree', stderr: '' }`
- No stale: `{ status: 2, stdout: 'No stale worktrees', stderr: '' }`
- Error: `{ status: 1, stdout: '', stderr: 'Database error' }`
- Not found: `{ error: { code: 'ENOENT' } }`

**Git states:**
- Branch exists: `git branch` includes branch name
- Branch missing: `git branch` doesn't include branch name
- Branch not merged: `git branch -d` fails with "not fully merged"

### Real Test Data

**Integration tests:**
- Temp git repository created per test
- Real worktrees created with `git worktree add`
- Real branches created
- Cleanup verified with `git worktree list`, `git branch`, `ls`

**No fixtures needed** - tests create their own state

## Failure Scenarios to Test

### 1. Maproom Binary Not Found

**Scenario:** Binary not in any search paths
**Expected:** Warning logged, cleanup continues
**Test:**
```typescript
it('warns when maproom binary not found', async () => {
  // Mock findMaproomBinary to return null
  // Call clean command
  // Assert warning logged
  // Assert directory/branch still cleaned
})
```

### 2. Maproom Cleanup Fails

**Scenario:** Binary found but cleanup fails (exit code 1)
**Expected:** Warning logged, cleanup continues
**Test:**
```typescript
it('warns when maproom cleanup fails', async () => {
  // Mock spawnSync to return exit code 1
  // Call clean command
  // Assert warning logged
  // Assert directory/branch still cleaned
})
```

### 3. Branch Not Fully Merged

**Scenario:** `git branch -d` fails because branch not merged
**Expected:** Warning logged, cleanup continues
**Test:**
```typescript
it('warns when branch deletion fails', async () => {
  // Mock GitMergeService.deleteBranch to throw
  // Call clean command
  // Assert warning logged
  // Assert directory/maproom still cleaned
})
```

### 4. Branch Doesn't Exist

**Scenario:** Branch already deleted or never existed
**Expected:** Warning logged (or no-op), cleanup continues
**Test:**
```typescript
it('handles missing branch gracefully', async () => {
  // Create worktree without branch name
  // Call clean command
  // Assert no error thrown
})
```

### 5. Database Locked

**Scenario:** Maproom database locked by another process
**Expected:** Warning logged, cleanup continues
**Test:**
- Manual test only (hard to simulate SQLite locking reliably)
- Maproom handles locking internally, we just catch errors

### 6. No Worktree Match

**Scenario:** Selector doesn't match any worktree (existing behavior)
**Expected:** Error logged, exit code 1
**Test:**
- Already covered by existing tests
- No changes to this behavior

## Quality Gates

Before ticket verification:
- [x] All new unit tests pass
- [x] All existing tests still pass (no regressions)
- [x] Integration tests pass (if applicable)
- [x] No TypeScript errors
- [x] No linting errors (`pnpm lint`)
- [x] Code formatted (`pnpm format`)

Before project completion:
- [x] All tickets verified
- [x] Manual testing on macOS (at least one platform)
- [x] Documentation updated
- [x] CHANGELOG entry added

## Test Organization

### File Structure

```
packages/cli/
├── src/
│   ├── utils/
│   │   ├── maproom-binary.ts          # New: binary discovery
│   │   └── maproom-binary.test.ts     # New: unit tests
│   └── cli/
│       └── worktree.ts                 # Modified: add cleanup steps
└── tests/
    ├── worktrees.int.test.ts           # Modified: add integration tests
    └── worktree-cleanup.int.test.ts    # New: cleanup-specific integration tests
```

### Test Execution

```bash
# Run all tests
cd packages/cli
pnpm test

# Run specific test file
pnpm test maproom-binary.test.ts

# Run with coverage
pnpm test --coverage

# Watch mode during development
pnpm test:watch
```

## Mocking Strategy

### What to Mock

1. **File system operations** - `fs.existsSync`, `fs.realpathSync`
   - Reason: Fast, predictable, no side effects

2. **Process spawning** - `spawnSync('crewchief-maproom', ...)`
   - Reason: Don't depend on binary being installed

3. **Git operations** - `GitMergeService.deleteBranch()`
   - Reason: Unit tests shouldn't modify git state

### What NOT to Mock

1. **Integration tests** - Use real git commands
   - Reason: Verify actual behavior, not mocked behavior

2. **Path operations** - Use real `path.join`, `path.resolve`
   - Reason: Cross-platform behavior matters

3. **Logger** - Use real logger (or spy on it)
   - Reason: Verify correct messages logged

## Performance Testing

**Requirement:** Cleanup completes in <5 seconds for typical worktree

**Verification:**
- Manual timing: `time crewchief worktree clean test-feature`
- Acceptable range: 1-5 seconds (typical case)
- Dominated by maproom startup time (1-3 seconds)

**Batch cleanup performance validation:**
- Test with database containing 50+ worktrees
- Verify cleanup completes in reasonable time (2-5 seconds acceptable)
- Document performance characteristics for users
- If cleanup takes >10 seconds, consider optimization or documenting limitation

**Manual test procedure:**
```bash
# Create multiple worktrees to simulate large database
for i in {1..50}; do
  crewchief worktree create test-$i
  crewchief-maproom scan # Index each
done

# Clean one worktree and measure total time
time crewchief worktree clean test-1

# Expected: 2-5 seconds total (most time in cleanup-stale scanning)
```

**No automated performance tests** - manual verification sufficient for MVP

## Regression Testing

**Ensure existing behavior unchanged:**

1. **Selector matching** - Branch name, path, basename still work
2. **`--stale` mode** - Prunes stale metadata
3. **`--all` mode** - Removes all non-current worktrees
4. **`--keep-dir` mode** - Preserves directory
5. **Error handling** - Ambiguous selector, no match, current directory

**Verification:** Run existing test suite, all should pass

## Test Coverage Goals

| Component | Target | Rationale |
|-----------|--------|-----------|
| Binary discovery | 90% | Critical path, many branches |
| Cleanup helper | 85% | Core functionality |
| Command integration | 70% | Integration covered separately |
| Error handling | 80% | Safety-critical |
| **Overall new code** | **80%** | Pragmatic, not ceremonial |

**Coverage is a guide, not a goal.** Focus on testing critical paths, not hitting numbers.

## Confidence Checklist

Before declaring "done":

- [x] I can create a worktree, clean it, and verify all artifacts removed
- [x] I can clean a worktree with maproom binary missing and it still works
- [x] I can clean a worktree with unmerged branch and get helpful warning
- [x] I can use `--keep-branch` and `--keep-maproom` flags successfully
- [x] Existing `clean` command behavior unchanged
- [x] Tests pass on CI
- [x] Manual testing on at least one platform

If all checked, we're confident this works.

## Known Limitations

**Not tested:**
- Windows-specific binary resolution (manual testing only)
- SQLite locking scenarios (hard to simulate reliably)
- Network filesystem edge cases (out of scope)
- Concurrent cleanup operations (not a use case)

**Acceptable risk:** These scenarios are rare and non-critical. Graceful degradation handles them.

## Summary

This quality strategy focuses on **confidence through pragmatic testing**:

- Unit tests cover binary discovery and cleanup logic
- Integration tests verify complete workflow
- Manual testing validates cross-platform behavior
- Failure scenarios explicitly tested
- Clear quality gates before verification

**Philosophy:** Ship confidently, not ceremoniously. Test what matters, mock what doesn't, and verify manually what's hard to automate.
