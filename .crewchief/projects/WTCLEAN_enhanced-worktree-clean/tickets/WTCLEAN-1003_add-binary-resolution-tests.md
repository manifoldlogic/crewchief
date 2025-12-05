# Ticket: [WTCLEAN-1003]: Add Unit Tests for Binary Resolution

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
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary
Create comprehensive unit tests for the binary discovery utility, covering all fallback strategies, platform variations, and error cases.

## Background
The binary discovery utility (WTCLEAN-1001) uses multiple fallback strategies to find the maproom binary. These tests ensure the discovery logic works correctly across all platforms and scenarios.

This ticket implements Phase 1, Deliverable 3 from the plan: Unit tests for binary resolution.

## Acceptance Criteria
- [ ] Test file `packages/cli/src/utils/maproom-binary.test.ts` created
- [ ] Test: Finds binary from `CREWCHIEF_MAPROOM_BIN` env var
- [ ] Test: Finds packaged binary in `bin/` directory
- [ ] Test: Finds dev build in `target/release/`
- [ ] Test: Falls back to command name for PATH lookup
- [ ] Test: Returns `null` when binary not found (all strategies fail)
- [ ] Test: Handles Windows `.exe` extension correctly
- [ ] Test: All platform combinations covered (darwin/linux/win32 × arm64/x64)
- [ ] All tests pass when executed
- [ ] Test coverage for `findMaproomBinary()` is at least 90%

## Technical Requirements
- Create test file: `packages/cli/src/utils/maproom-binary.test.ts`
- Use Vitest as test framework (existing CLI test setup)
- Mock `fs.existsSync` to control file system state
- Mock `process.env` to control environment variables
- Mock `process.platform` and `process.arch` for cross-platform tests
- Mock `__dirname` to control packaged binary path
- Use `describe` blocks to group related tests
- Use descriptive test names following pattern: `it('finds binary from X')`
- Clean up mocks in `afterEach` to prevent test pollution
- Assert return values match expected paths
- Assert `null` returned when appropriate

## Implementation Notes
Follow the quality strategy test structure:

```typescript
describe('findMaproomBinary', () => {
  // Test environment variable strategy
  it('finds binary from CREWCHIEF_MAPROOM_BIN env var', () => {
    // Mock env var and fs.existsSync
    // Call findMaproomBinary()
    // Assert returns env var path
  })

  // Test packaged binary strategy
  it('finds packaged binary in bin/ directory', () => {
    // Mock process.platform, process.arch
    // Mock fs.existsSync for packaged path
    // Call findMaproomBinary()
    // Assert returns packaged binary path
  })

  // Test dev build strategy
  it('finds dev build in target/release', () => {
    // Mock fs.existsSync for dev paths
    // Call findMaproomBinary()
    // Assert returns resolved dev path
  })

  // Test PATH fallback strategy
  it('falls back to command name for PATH lookup', () => {
    // Mock all fs.existsSync to return false
    // Call findMaproomBinary()
    // Assert returns 'crewchief-maproom'
  })

  // Test failure case
  it('returns null when binary not found', () => {
    // Mock all strategies to fail
    // Set up to return null (not command name)
    // Call findMaproomBinary()
    // Assert returns null
  })

  // Test Windows-specific behavior
  it('handles Windows .exe extension correctly', () => {
    // Mock process.platform = 'win32'
    // Verify binary path includes .exe
    // Assert correct path returned
  })

  // Test all platform combinations
  it.each([
    ['darwin', 'arm64'],
    ['darwin', 'x64'],
    ['linux', 'x64'],
    ['linux', 'arm64'],
    ['win32', 'x64'],
  ])('handles %s-%s platform combination', (platform, arch) => {
    // Mock process.platform and process.arch
    // Mock packaged binary exists
    // Call findMaproomBinary()
    // Assert correct platform-specific path returned
  })
})

describe('cleanMaproomRecords', () => {
  it('calls db cleanup-stale --confirm', () => {
    // Mock findMaproomBinary
    // Mock spawnSync
    // Call cleanMaproomRecords()
    // Assert spawnSync called with correct args
  })

  it('handles exit code 0 (success)', async () => {
    // Mock successful spawn
    // Call cleanMaproomRecords()
    // Assert no error thrown
  })

  it('handles exit code 2 (no stale worktrees)', async () => {
    // Mock spawn with exit code 2
    // Call cleanMaproomRecords()
    // Assert no error thrown (2 is success)
  })

  it('throws on exit code 1 (error)', async () => {
    // Mock spawn with exit code 1
    // Assert cleanMaproomRecords() throws
    // Assert error message includes stderr
  })

  it('throws when binary not found', async () => {
    // Mock findMaproomBinary to return null
    // Assert cleanMaproomRecords() throws
    // Assert error message mentions binary not found
  })
})
```

**Mocking strategy:**
- Mock `fs.existsSync` to control which paths "exist"
- Mock `process.env` to control environment variables
- Mock `process.platform` and `process.arch` for platform tests
- Mock `spawnSync` to avoid calling real binary
- Use Vitest's `vi.mock()` and `vi.spyOn()` utilities

**Coverage goals:**
- All 4 fallback strategies tested
- All common platform combinations tested
- Windows `.exe` handling verified
- Error cases (binary not found) tested
- Target: 90% coverage of `findMaproomBinary()`

## Dependencies
- **WTCLEAN-1001** (Binary discovery utility) - MUST be completed first
- **WTCLEAN-1002** (Cleanup helper function) - Include tests for this too

## Risk Assessment
- **Risk**: Mocks don't accurately reflect real behavior
  - **Mitigation**: Combine with manual testing on real platforms
- **Risk**: Platform-specific tests only pass on one platform
  - **Mitigation**: Mock `process.platform` and `process.arch` to test all combinations
- **Risk**: Tests too brittle (break on implementation changes)
  - **Mitigation**: Test behavior/outputs, not implementation details

## Files/Packages Affected
- `packages/cli/src/utils/maproom-binary.test.ts` (new file)
- `packages/cli/package.json` (may need test script updates)

## Verification Notes
Verify-ticket agent should check:
- [ ] Test file exists at correct path
- [ ] All tests listed in acceptance criteria are present
- [ ] Tests actually executed (not just file created)
- [ ] All tests pass (green checkmarks in test output)
- [ ] Test coverage report shows >90% for `maproom-binary.ts`
- [ ] No skipped or pending tests (all tests run)
- [ ] Tests use proper mocking (don't depend on real file system)
- [ ] Tests clean up mocks properly (no cross-test pollution)
