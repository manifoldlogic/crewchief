# Ticket: [WTSCAN-1003]: Add Integration Tests for Auto-Scan Behavior

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
- typescript-dev
- test-runner
- verify-ticket
- commit-ticket

## Summary
Add comprehensive integration tests to verify auto-scan behavior in all scenarios: default (off), explicitly disabled, explicitly enabled, and error handling.

## Background
WTSCAN-1001 and WTSCAN-1002 implemented the config field and conditional logic. This ticket adds the test coverage to ensure the feature works correctly in all cases and doesn't break existing functionality.

This completes Phase 1 by providing confidence that the implementation meets all acceptance criteria and handles edge cases gracefully.

## Acceptance Criteria
- [ ] Test: Scan skipped by default (no worktree config section)
- [ ] Test: Scan skipped when `autoScanOnWorktreeUse: false`
- [ ] Test: Scan runs when `autoScanOnWorktreeUse: true`
- [ ] Test: Config loading errors don't break worktree creation
- [ ] Test: Scan is called with correct worktree path
- [ ] All new tests pass
- [ ] All existing tests still pass (no regressions)
- [ ] Test coverage includes all critical paths

## Technical Requirements
- Add test suite to `packages/cli/src/cli/__tests__/worktree-create.test.ts`
- Use `vi.spyOn(WorktreeService.prototype, 'runMaproomScan')` to verify calls
- Mock `loadConfig()` to return different config fixtures
- Follow existing test patterns in the file
- Use descriptive test names (describe/it blocks)
- Verify both positive (scan runs) and negative (scan skipped) cases

## Implementation Notes
**Test File**: `packages/cli/src/cli/__tests__/worktree-create.test.ts`

Add a new describe block for auto-scan tests:

```typescript
describe('auto-scan behavior', () => {
  let scanSpy: SpyInstance

  beforeEach(() => {
    // Spy on runMaproomScan to verify if it's called
    scanSpy = vi.spyOn(WorktreeService.prototype, 'runMaproomScan')
      .mockResolvedValue(undefined)
  })

  afterEach(() => {
    scanSpy.mockRestore()
  })

  it('skips maproom scan by default (no worktree config)', async () => {
    const mockConfig = {
      repository: { mainBranch: 'main', worktreeBasePath: '/worktrees' },
      // No worktree section
    }
    vi.mocked(loadConfig).mockResolvedValue(mockConfig as any)

    await executeWorktreeCreate('feature-x')

    expect(scanSpy).not.toHaveBeenCalled()
  })

  it('skips maproom scan when autoScanOnWorktreeUse is false', async () => {
    const mockConfig = {
      repository: { mainBranch: 'main', worktreeBasePath: '/worktrees' },
      worktree: { autoScanOnWorktreeUse: false },
    }
    vi.mocked(loadConfig).mockResolvedValue(mockConfig as any)

    await executeWorktreeCreate('feature-x')

    expect(scanSpy).not.toHaveBeenCalled()
  })

  it('runs maproom scan when autoScanOnWorktreeUse is true', async () => {
    const mockConfig = {
      repository: { mainBranch: 'main', worktreeBasePath: '/worktrees' },
      worktree: { autoScanOnWorktreeUse: true },
    }
    vi.mocked(loadConfig).mockResolvedValue(mockConfig as any)

    await executeWorktreeCreate('feature-x')

    expect(scanSpy).toHaveBeenCalledOnce()
    expect(scanSpy).toHaveBeenCalledWith(expect.stringContaining('feature-x'))
  })

  it('handles config loading errors gracefully', async () => {
    vi.mocked(loadConfig).mockRejectedValue(new Error('Config read failed'))

    // Should still create worktree successfully
    await executeWorktreeCreate('feature-x')

    expect(WorktreeService.prototype.createWorktree).toHaveBeenCalled()
    expect(scanSpy).not.toHaveBeenCalled()
  })
})
```

**Test Strategy**:
- **Positive tests**: Verify scan runs when enabled
- **Negative tests**: Verify scan skipped when disabled or default
- **Error tests**: Verify resilience when config fails
- **Regression tests**: Ensure existing tests still pass

**Mocking Approach**:
- Use `vi.spyOn()` instead of full mock replacement (cleaner)
- Mock `loadConfig()` to return fixtures
- Spy on `runMaproomScan()` to verify calls without executing
- Use `beforeEach`/`afterEach` for setup/cleanup

## Dependencies
- **Prerequisite**: WTSCAN-1001 (config schema)
- **Prerequisite**: WTSCAN-1002 (conditional logic)
- **External**: Vitest testing framework (already in use)
- **Pattern**: Follow existing tests in `worktree-create.test.ts`

## Risk Assessment
- **Risk**: Mock setup is complex or fragile
  - **Mitigation**: Follow existing mock patterns in the test file. Use `vi.spyOn()` which is simpler than full mocks.
- **Risk**: Tests don't actually verify behavior
  - **Mitigation**: Use specific assertions (`toHaveBeenCalledOnce`, `not.toHaveBeenCalled`). Verify test runner shows clear pass/fail.
- **Risk**: Existing tests break
  - **Mitigation**: Run full test suite. Any failures must be investigated and fixed.

## Files/Packages Affected
- `packages/cli/src/cli/__tests__/worktree-create.test.ts` - Add new test suite

## Verification Notes
**test-runner agent MUST**:
1. Execute tests and show output
2. Verify all new tests pass
3. Verify all existing tests still pass
4. Confirm no console errors during test execution

**verify-ticket agent should check**:
1. Test file contains new describe block for "auto-scan behavior"
2. Tests cover all 4 critical scenarios (default, false, true, error)
3. Tests use `vi.spyOn()` for cleaner mocking
4. Test assertions are specific and meaningful
5. Test names are descriptive
6. test-runner output shows green (all tests passing)
7. No regressions in existing test suite

**Critical Paths Verified**:
- Default fast path (no config → no scan)
- Opt-in scan path (config true → scan runs)
- Error resilience (config error → worktree created, no scan)
- Regression prevention (existing tests → still pass)
