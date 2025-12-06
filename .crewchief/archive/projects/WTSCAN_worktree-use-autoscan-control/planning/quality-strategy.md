# Quality Strategy: Worktree Use Auto-Scan Control

## Testing Philosophy

**Core Principle**: Test for confidence, not coverage.

This project adds a simple config field and conditional logic. Our testing strategy focuses on:

1. **Config validation** - Ensure Zod schema accepts/rejects correct values
2. **Behavior verification** - Confirm scan runs/skips based on config
3. **Error resilience** - Verify config errors don't break worktree creation
4. **Regression prevention** - Ensure existing functionality still works

**Not Focusing On**:
- Line-by-line coverage metrics
- Testing framework internals
- Over-mocking simple conditionals

**Why**: The change is small and isolated. Pragmatic testing ensures it works without ceremony.

## Test Types

### Unit Tests

**Scope**: Config schema validation and conditional logic

**Files to Test**:
- `packages/cli/src/config/schema.ts` - Schema accepts boolean field
- `packages/cli/src/git/worktrees.ts` - Conditional scan logic (via integration tests)

**Tools**:
- Vitest (existing test framework)
- Zod validation (built-in)

**Coverage Target**: 100% of new conditional logic (the if statement and try-catch)

**Test Cases**:

1. **Config Schema Validation**
   - Valid boolean values (true, false)
   - Invalid values ("yes", 1, null) should fail validation
   - Undefined/missing field uses default (false)
   - Type inference works correctly

2. **Conditional Logic** (tested via integration)
   - Scan skipped when config is false
   - Scan skipped when config is undefined
   - Scan runs when config is true
   - Config error doesn't crash worktree creation

**Example Test Structure**:
```typescript
describe('WorktreeSchema', () => {
  it('accepts autoScanOnWorktreeUse as boolean', () => {
    const valid = WorktreeSchema.parse({ autoScanOnWorktreeUse: true })
    expect(valid.autoScanOnWorktreeUse).toBe(true)
  })

  it('defaults autoScanOnWorktreeUse to false', () => {
    const defaults = WorktreeSchema.parse({})
    expect(defaults.autoScanOnWorktreeUse).toBe(false)
  })

  it('rejects non-boolean values', () => {
    expect(() => WorktreeSchema.parse({ autoScanOnWorktreeUse: "yes" }))
      .toThrow()
  })
})
```

### Integration Tests

**Scope**: End-to-end worktree creation with config variations

**File**: `packages/cli/src/cli/__tests__/worktree-create.test.ts`

**Approach**: Use `vi.spyOn(WorktreeService.prototype, 'runMaproomScan')` to spy on the method and verify it's called/not called based on config

**Test Cases**:

1. **Default Behavior** (config undefined or false)
   ```typescript
   it('skips maproom scan by default', async () => {
     // Config has no worktree section
     await executeWorktreeCreate('feature-x')
     expect(WorktreeService.prototype.runMaproomScan).not.toHaveBeenCalled()
   })
   ```

2. **Explicit False**
   ```typescript
   it('skips scan when autoScanOnWorktreeUse is false', async () => {
     // Config explicitly sets false
     await executeWorktreeCreate('feature-x')
     expect(WorktreeService.prototype.runMaproomScan).not.toHaveBeenCalled()
   })
   ```

3. **Explicit True**
   ```typescript
   it('runs scan when autoScanOnWorktreeUse is true', async () => {
     // Config enables auto-scan
     await executeWorktreeCreate('feature-x')
     expect(WorktreeService.prototype.runMaproomScan)
       .toHaveBeenCalledWith('/path/to/worktrees/feature-x')
   })
   ```

4. **Error Handling**
   ```typescript
   it('handles config loading errors gracefully', async () => {
     vi.mocked(loadConfig).mockRejectedValue(new Error('Config failed'))

     // Should still create worktree
     await executeWorktreeCreate('feature-x')
     expect(WorktreeService.prototype.createWorktree).toHaveBeenCalled()
     expect(logger.warn).toHaveBeenCalledWith(
       expect.stringContaining('Failed to check auto-scan config')
     )
   })
   ```

5. **Regression Prevention**
   ```typescript
   it('still creates worktree successfully', async () => {
     // Existing test - ensure it still passes
     await executeWorktreeCreate('feature-x')
     expect(WorktreeService.prototype.createWorktree)
       .toHaveBeenCalledWith('feature-x', 'main', '/worktrees', false)
   })
   ```

**Why Integration Over Unit**:
- The conditional logic is simple (one if statement)
- Integration tests verify the entire flow works
- Mocking at the service level provides confidence
- Avoids testing implementation details

### End-to-End Tests

**Scope**: Not required for this change

**Rationale**:
- E2E tests for CLI tools require shell interaction and git setup
- Existing E2E tests cover worktree creation
- Our integration tests provide sufficient confidence
- Config loading is well-tested in other parts of codebase

**If E2E Were Required** (for reference):
```bash
# Test default behavior (no scan)
time crewchief worktree create test-feature
# Should complete in <1 second

# Test with auto-scan enabled
echo 'export default { worktree: { autoScanOnWorktreeUse: true } }' > crewchief.config.js
time crewchief worktree create test-feature-2
# Should take 5-30 seconds and show scan output
```

## Critical Paths

The following paths MUST be tested before shipping:

### 1. Default Fast Path
**Path**: User creates worktree without config → Worktree created instantly, no scan

**Test**: `skips maproom scan by default`

**Why Critical**: This is the new default behavior and primary value proposition

**Verification**:
- Worktree creation completes
- No maproom scan output
- `runMaproomScan()` not called

### 2. Opt-In Scan Path
**Path**: User enables auto-scan in config → Worktree created with scan

**Test**: `runs scan when autoScanOnWorktreeUse is true`

**Why Critical**: Users who want old behavior must have it work correctly

**Verification**:
- Worktree creation completes
- Maproom scan runs
- `runMaproomScan()` called with correct path

### 3. Error Resilience Path
**Path**: Config loading fails → Worktree still created, warning shown

**Test**: `handles config loading errors gracefully`

**Why Critical**: Config errors must never break core functionality

**Verification**:
- Worktree creation succeeds
- Warning logged to console
- User can still work

### 4. Regression Prevention Path
**Path**: Existing worktree operations still work

**Test**: All existing tests in `worktree-create.test.ts`

**Why Critical**: Must not break existing functionality

**Verification**:
- All existing tests pass
- No new test failures
- Coverage doesn't decrease

## Test Data Strategy

**Config Fixtures**:
```typescript
const mockConfigNoWorktree = {
  repository: { mainBranch: 'main', worktreeBasePath: '/worktrees' },
  // No worktree section
}

const mockConfigAutoScanOff = {
  repository: { mainBranch: 'main', worktreeBasePath: '/worktrees' },
  worktree: { autoScanOnWorktreeUse: false },
}

const mockConfigAutoScanOn = {
  repository: { mainBranch: 'main', worktreeBasePath: '/worktrees' },
  worktree: { autoScanOnWorktreeUse: true },
}
```

**Mocking Strategy**:
- Mock `loadConfig()` to return fixtures
- Use `vi.spyOn(WorktreeService.prototype, 'runMaproomScan')` to verify calls (cleaner than full mocking)
- Mock `WorktreeService.createWorktree()` for speed
- Use `vi.spyOn()` for logger to verify warnings

**Example Mock Setup**:
```typescript
const scanSpy = vi.spyOn(WorktreeService.prototype, 'runMaproomScan')
  .mockResolvedValue(undefined)

// Later in test:
expect(scanSpy).toHaveBeenCalledWith('/path/to/worktree')
// or
expect(scanSpy).not.toHaveBeenCalled()
```

**No Real Git Required**: All tests use mocks, no actual worktrees created.

## Quality Gates

### Before Commit
- [ ] All new tests pass
- [ ] All existing tests still pass (no regression)
- [ ] TypeScript compiles without errors
- [ ] ESLint passes (no new warnings)
- [ ] Test coverage includes new conditional logic

### Before Verification
- [ ] Unit tests pass (config schema validation)
- [ ] Integration tests pass (all 5 test cases)
- [ ] No console errors during test run
- [ ] Test execution time <10 seconds (should be instant)

### Before Release
- [ ] Full test suite passes (`pnpm test`)
- [ ] Manual smoke test: create worktree without config (fast)
- [ ] Manual smoke test: create worktree with config true (scans)
- [ ] Documentation reviewed by verify-ticket agent
- [ ] Breaking change clearly communicated

## Test Execution

**Run All Tests**:
```bash
cd packages/cli
pnpm test
```

**Run Specific Tests**:
```bash
pnpm test worktree-create.test.ts
```

**Run in Watch Mode** (during development):
```bash
pnpm test:watch worktree-create
```

**Coverage Report** (informational, not a gate):
```bash
pnpm test --coverage
```

## Manual Testing Checklist

**Assigned to**: `verify-ticket` agent as part of Phase 1 verification
**When**: Serves as a gate before Phase 2 documentation begins

Before marking Phase 1 complete, manually verify:

### Without Auto-Scan (Default)
- [ ] `crewchief worktree create test-1` completes in <1 second
- [ ] No maproom scan output shown
- [ ] Worktree exists and is functional
- [ ] `git worktree list` shows new worktree

### With Auto-Scan Enabled
- [ ] Create config: `{ worktree: { autoScanOnWorktreeUse: true } }`
- [ ] `crewchief worktree create test-2` shows scan output
- [ ] Scan completes successfully
- [ ] Worktree is indexed (verify with maproom search)

### Error Handling
- [ ] Break config syntax → Worktree still created
- [ ] Invalid config value → Clear error message
- [ ] Missing maproom binary → Worktree created, warning shown

### Backward Compatibility
- [ ] Existing config files without new field → Works
- [ ] All other worktree commands → Unchanged behavior
- [ ] Config validation → No new errors

## Regression Testing Strategy

**Existing Tests Must Pass**: All 7 tests in `worktree-create.test.ts` must continue passing.

**Key Existing Tests**:
1. `prints path to stdout after creation by default`
2. `spawns shell with --shell flag`
3. `passes --branch option to WorktreeService`
4. `passes --base-path option to WorktreeService`
5. `passes --no-copy-ignored option to WorktreeService`
6. `uses config defaults when options not provided`
7. `shows success message to stderr via logger`

**If Any Fail**: Stop, investigate, fix before proceeding.

## Test Maintenance

**Keep Tests Simple**:
- Avoid complex mocking chains
- Use clear, descriptive test names
- One assertion per test when possible
- Follow existing test patterns

**When to Update Tests**:
- Config schema changes → Update schema validation tests
- Conditional logic changes → Update integration tests
- Error handling changes → Update error tests
- New features → Add new tests

**When NOT to Update Tests**:
- Implementation details change (same behavior)
- Refactoring without behavior change
- Performance optimizations

## Definition of Quality

This project meets quality standards when:

1. **All Tests Pass**: New and existing tests green
2. **No Regressions**: Existing functionality unchanged
3. **Error Paths Tested**: Config errors handled gracefully
4. **Critical Paths Covered**: All 4 critical paths tested
5. **Manual Verification**: Manual testing checklist complete
6. **Documentation Accurate**: README examples work as written

**Not Required**:
- 100% line coverage
- Performance benchmarks
- Load testing
- Cross-platform testing (TypeScript/Node.js is portable)

**Pragmatic Approach**: Ship with confidence, not with ceremony.
