# Ticket: [MRBIN-2001]: Test Coverage for Config Parameter Usage

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - all tests passing
- [x] **Verified** - by the verify-ticket agent

**Note on "Tests pass"**:
- If tests were created/modified, you MUST run them and show output
- "Tests pass" means tests were EXECUTED and all passed
- "Tests pass - N/A" is only valid for documentation-only tickets
- Test file existence alone does NOT satisfy this requirement

## Agents
- unit-test-specialist
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary
Add 2-3 new test cases to the existing `clean-maproom-records.test.ts` file to verify that `cleanMaproomRecords()` correctly handles the new optional config parameter, loads config when not provided, and gracefully handles config load failures.

## Background
The existing test file (`packages/cli/tests/unit/clean-maproom-records.test.ts`) already contains 26 comprehensive test cases covering binary resolution, error handling, and edge cases. After MRBIN-1001 adds config parameter support to `cleanMaproomRecords()`, we need specific tests to verify this new functionality works correctly.

This ticket ensures the config parameter integration is thoroughly tested without duplicating existing binary resolution tests.

## Acceptance Criteria
- [x] Test case added: Config parameter provided → function uses it
- [x] Test case added: Config parameter not provided → function loads config internally
- [x] Test case added: Config load fails → function handles gracefully and falls back to env/packaged
- [x] All new tests pass
- [x] All 25 existing tests continue to pass (note: 1 test was updated to match MRBIN-1001 changes)
- [x] Test coverage maintained at 90%+ for `cleanMaproomRecords()` function
- [x] Mocking works correctly (config loading, binary resolution)

## Technical Requirements
- Add new test cases to `packages/cli/tests/unit/clean-maproom-records.test.ts`
- Mock `loadConfig()` function to return test config or throw errors
- Mock `findMaproomBinary()` to verify correct parameters passed
- Use Vitest mocking patterns consistent with existing tests in the file
- Each test should verify both the config loading behavior AND the binary resolution call
- Ensure tests are isolated and don't affect each other

## Implementation Notes

**Test 1: Config parameter provided**
```typescript
it('should use provided config parameter', async () => {
  const mockConfig = {
    repository: {
      maproomBinaryPath: '/custom/maproom'
    }
  } as CrewChiefConfig

  const findBinarySpy = vi.spyOn(maproomBinary, 'findMaproomBinary')
  // ... setup mocks for daemon, spawnSync, etc ...

  await cleanMaproomRecords(mockConfig)

  expect(findBinarySpy).toHaveBeenCalledWith({
    configPath: '/custom/maproom'
  })
})
```

**Test 2: Config parameter not provided (loads internally)**
```typescript
it('should load config when not provided', async () => {
  const mockConfig = {
    repository: {
      maproomBinaryPath: '/loaded/maproom'
    }
  } as CrewChiefConfig

  vi.mocked(loadConfig).mockResolvedValue(mockConfig)
  const findBinarySpy = vi.spyOn(maproomBinary, 'findMaproomBinary')
  // ... setup mocks ...

  await cleanMaproomRecords() // No config parameter

  expect(loadConfig).toHaveBeenCalled()
  expect(findBinarySpy).toHaveBeenCalledWith({
    configPath: '/loaded/maproom'
  })
})
```

**Test 3: Config load fails (graceful fallback)**
```typescript
it('should handle config load failure gracefully', async () => {
  vi.mocked(loadConfig).mockRejectedValue(new Error('Config not found'))
  const findBinarySpy = vi.spyOn(maproomBinary, 'findMaproomBinary')
  // ... setup mocks ...

  await cleanMaproomRecords() // No config parameter

  expect(loadConfig).toHaveBeenCalled()
  expect(findBinarySpy).toHaveBeenCalledWith({
    configPath: undefined
  })
  // Should not throw, should continue with fallback resolution
})
```

**Existing test structure to follow:**
- Review lines 1-50 of existing test file for import patterns
- Follow existing mock setup patterns (daemon, spawnSync, fs)
- Use the same describe/it structure
- Clean up mocks in beforeEach/afterEach hooks

## Dependencies
- **MRBIN-1001**: Must be complete - the config parameter functionality must exist before tests can be written

## Risk Assessment
- **Risk**: Tests don't accurately reflect real behavior
  - **Mitigation**: Use same mocking patterns as existing 26 tests; verify tests fail when code is broken
- **Risk**: Mocking issues cause false positives
  - **Mitigation**: Verify mocks are called with exact expected parameters; check spy call counts
- **Risk**: New tests break existing tests
  - **Mitigation**: Isolate mocks properly; ensure all tests pass together
- **Risk**: Missing edge cases
  - **Mitigation**: Cover all three code paths (config provided, loaded, failed)

## Files/Packages Affected
- `packages/cli/tests/unit/clean-maproom-records.test.ts` (add 2-3 new test cases)
- No changes to source files (testing MRBIN-1001 changes)

## Verification Notes
Verify that:
1. All three new test cases exist and are properly structured
2. Each test uses appropriate mocking (loadConfig, findMaproomBinary)
3. Tests verify both config loading AND binary resolution call parameters
4. All new tests pass when run with `pnpm test`
5. All 26 existing tests still pass (no regressions)
6. Total test count increases by 2-3
7. Tests fail when expected behavior is broken (not false positives)
8. Code coverage remains at 90%+ for the modified function

## Planning References
- Plan: `.crewchief/projects/MRBIN_maproom-binary-configuration/planning/plan.md` (Phase 2)
- Quality Strategy: `.crewchief/projects/MRBIN_maproom-binary-configuration/planning/quality-strategy.md` (Testing Approach section)
