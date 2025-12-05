# Ticket: [MRBIN-1003]: Unit Tests for Binary Resolution

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
- typescript-engineer
- test-runner
- verify-ticket
- commit-ticket

## Summary
Create comprehensive unit tests for the findMaproomBinary() utility to validate precedence order, platform handling, path validation, and error scenarios. Tests must achieve 90%+ coverage of the utility function.

## Background
The binary resolution utility is critical infrastructure that must work correctly across all platforms and scenarios. This ticket creates a comprehensive test suite that validates all precedence levels, platform-specific behavior, and edge cases.

As specified in quality-strategy.md, testing focuses on precedence order (highest risk), platform differences, and failure paths. Tests use mocks for fs and child_process to ensure deterministic behavior.

## Acceptance Criteria
- [ ] New test file created at packages/cli/tests/utils/maproom-binary.test.ts
- [ ] 6+ tests for precedence order (env > config > global > packaged)
- [ ] 2+ tests for platform handling (Windows .exe vs Unix)
- [ ] 2+ tests for path validation (relative, absolute, missing)
- [ ] 2+ tests for edge cases (undefined options, missing config)
- [ ] All tests pass when executed
- [ ] Test coverage >= 90% for maproom-binary.ts
- [ ] Tests use vi.mock for fs.existsSync and spawnSync
- [ ] Tests are deterministic and independent

## Technical Requirements
- Test framework: Vitest (existing framework)
- Mocking: vi.mock for fs and child_process modules
- Spying: vi.spyOn for console.warn (warning validation)
- Platform testing: Mock process.platform for Windows/Unix scenarios
- Environment testing: Mock process.env.CREWCHIEF_MAPROOM_BIN
- Coverage target: 90%+ for findMaproomBinary function

## Implementation Notes
Test structure as specified in quality-strategy.md:

```typescript
describe('findMaproomBinary', () => {
  // Precedence tests
  it('prioritizes CREWCHIEF_MAPROOM_BIN env var over all others')
  it('uses config path when env var not set')
  it('uses global install when env var and config not set')
  it('falls back to packaged binary when nothing else available')
  it('returns not-found when no binary exists')
  it('returns correct source information for debugging')

  // Platform tests
  it('uses .exe suffix on Windows')
  it('uses no suffix on Unix')

  // Path validation tests
  it('resolves relative config paths')
  it('handles absolute config paths')
  it('warns when config path does not exist')
  it('falls through to next priority when config path invalid')

  // Edge cases
  it('handles missing process.env.CREWCHIEF_MAPROOM_BIN gracefully')
  it('handles undefined options parameter')
  it('handles empty string config path')
})
```

Mock fs.existsSync to return true for known valid paths, false otherwise. Mock spawnSync to simulate global binary detection success/failure.

Use beforeEach/afterEach to set up and tear down environment variables and mocks to ensure test independence.

## Dependencies
- MRBIN-1002 (Utility must exist to test it)

## Risk Assessment
- **Risk**: Tests too brittle (break on implementation changes)
  - **Mitigation**: Test behavior, not implementation details; use minimal mocks
- **Risk**: Platform tests don't cover actual platform behavior
  - **Mitigation**: Mock platform detection clearly, validate with manual testing
- **Risk**: Tests miss edge cases
  - **Mitigation**: Follow test matrix from quality-strategy.md, review coverage report

## Files/Packages Affected
- packages/cli/tests/utils/maproom-binary.test.ts (NEW)

## Verification Notes
Verify that:
1. All tests pass when executed with pnpm test
2. Coverage report shows 90%+ coverage for maproom-binary.ts
3. Tests are independent (can run in any order)
4. Tests are deterministic (same result every time)
5. Mock usage is minimal and focused
6. Test descriptions clearly state what is being tested
7. All precedence levels have explicit test coverage
8. Both Windows and Unix platforms are tested
9. Warning emission is validated with spies
10. Tests run quickly (<1s total)
