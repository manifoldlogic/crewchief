# Ticket: DINDFX-1001: Write failing tests for workspace path detection

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - tests executed and passing (or N/A if no tests)
- [x] **Verified** - by the verify-ticket agent

**Note on "Tests pass"**:
- For this ticket, tests MUST be executed and MUST FAIL as expected
- Success means: all 18 tests fail with "function not defined" errors
- We're proving the problem exists before implementing the solution

## Agents
- general-purpose
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary
Write comprehensive unit and integration tests for workspace path detection functionality before implementing the actual code. This follows test-driven development (TDD) principles - write failing tests first to prove we understand the problem, then implement the solution to make them pass.

## Background
The maproom-mcp setup fails in devcontainer environments because `WORKSPACE_HOST_PATH` is never set automatically. The system doesn't detect when it's running inside Docker or know how to retrieve the host path for volume mapping.

This ticket implements Phase 1 of the DINDFX project: creating a comprehensive test suite that:
1. Defines the expected behavior through tests
2. Proves the problem exists (functions don't exist yet, so tests fail)
3. Provides a clear specification for Phase 2 implementation
4. Ensures we have proper test coverage from day one

Reference: `.crewchief/projects/DINDFX_docker-workspace-path-detection/planning/plan.md` (Phase 1: Test Foundation)

## Acceptance Criteria
- [ ] Unit test file created: `packages/maproom-mcp/tests/utils/workspace-path-detection.test.ts`
- [ ] Integration test file created: `packages/maproom-mcp/tests/integration/workspace-path-detection.int.test.ts`
- [ ] All 15 unit tests written covering `isInsideDocker()`, `getWorkspaceHostPath()`, and `resolveWorkspacePath()`
- [ ] All 3 integration tests written covering `runSetup()` integration
- [ ] All tests use clear GIVEN/WHEN/THEN structure with proper Vitest syntax
- [ ] Mocking strategy implemented for `fs`, `execFileSync`, and `spawn`
- [ ] Running `pnpm test workspace-path-detection` confirms all 18 tests fail with expected "function not defined" errors
- [ ] Test output is captured and demonstrates the failing state

## Technical Requirements

### Test Framework
- Use Vitest (already configured in maproom-mcp)
- Import: `import { vi, describe, it, expect, beforeEach, afterEach } from 'vitest'`
- Follow existing test patterns from `packages/maproom-mcp/tests/provider-detection.test.ts`

### Test File Locations
- Unit tests: `packages/maproom-mcp/tests/utils/workspace-path-detection.test.ts`
- Integration tests: `packages/maproom-mcp/tests/integration/workspace-path-detection.int.test.ts`

### Unit Test Suites (15 tests total)

**`isInsideDocker()` Suite (5 tests):**
1. Returns true when `/.dockerenv` exists
2. Returns true when `/run/.containerenv` exists
3. Returns true when `/proc/1/cgroup` contains "docker"
4. Returns false when no Docker indicators present
5. Handles errors gracefully (returns false on exception)

**`getWorkspaceHostPath()` Suite (5 tests):**
1. Returns host path from `docker inspect` on success
2. Returns null when `docker inspect` fails
3. Returns null when hostname command fails
4. Returns null when no host path found in Docker inspect
5. Trims whitespace from returned path

**`resolveWorkspacePath()` Suite (5 tests):**
1. Returns user-provided `WORKSPACE_HOST_PATH` when set
2. Auto-detects and returns host path when inside Docker
3. Returns current directory when running on host (not in Docker)
4. Returns fallback path when detection fails
5. Logs diagnostic info when `DEBUG=true`

### Integration Test Suite (3 tests)

**`runSetup() integration` Suite:**
1. Sets `WORKSPACE_HOST_PATH` environment variable before spawning docker compose
2. Spawns docker compose with resolved workspace path
3. Handles failures gracefully without crashing

### Mocking Requirements

**Early Verification Checkpoint:**
Before writing all 18 tests, create one simple test that verifies CommonJS mocking works:
1. Write a basic test that mocks `execFileSync`
2. Verify Vitest can mock CommonJS `require()` statements from ESM tests
3. If mocking works: proceed with all tests as planned
4. If mocking fails: see contingency plan in Risk Mitigations section

**Note:** The functions being tested live in `bin/cli.cjs` (CommonJS using `require()`), but our tests are TypeScript/ESM. Vitest handles this seamlessly. See `quality-strategy.md` Section "Child process operations" for detailed mocking patterns.

**File System Mocks:**
```typescript
import * as fs from 'fs'
vi.mock('fs', () => ({
  existsSync: vi.fn(),
  readFileSync: vi.fn()
}))
```

**Child Process Mocks:**
```typescript
import { execFileSync, spawn } from 'child_process'
// This mocking pattern works even though bin/cli.cjs uses: const { execFileSync } = require('child_process')
vi.mock('child_process', () => ({
  execFileSync: vi.fn(),
  spawn: vi.fn()
}))
```

**Type-Safe Mock Usage:**
```typescript
const mockedExistsSync = vi.mocked(fs.existsSync)
const mockedExecFileSync = vi.mocked(execFileSync)
```

### Expected Function Signatures (from architecture.md)
```typescript
function isInsideDocker(): boolean
function getWorkspaceHostPath(): string | null
function resolveWorkspacePath(): string
```

## Implementation Notes

### Test Structure
Each test should follow GIVEN/WHEN/THEN pattern:
```typescript
it('should detect Docker via /.dockerenv', () => {
  // GIVEN: /.dockerenv file exists
  mockedExistsSync.mockImplementation((path) =>
    path === '/.dockerenv'
  )

  // WHEN: checking if inside Docker
  const result = isInsideDocker()

  // THEN: should return true
  expect(result).toBe(true)
})
```

### Mock Reset Strategy
- Use `beforeEach()` to clear all mocks before each test
- Use `afterEach()` to restore original implementations
- Keep tests isolated and independent

### Test Focus
- Test behavior, not implementation details
- Focus on the contract/interface
- Cover happy path, error cases, and edge cases
- Verify proper error handling

### Expected Outcome
All tests MUST fail when run because the functions don't exist yet. This is the desired outcome for Phase 1. The test output should show clear error messages indicating the functions are not defined.

### Reference Materials
- Test structure examples: `.crewchief/projects/DINDFX_docker-workspace-path-detection/planning/quality-strategy.md`
- Function signatures: `.crewchief/projects/DINDFX_docker-workspace-path-detection/planning/architecture.md`
- Mock patterns: `packages/maproom-mcp/tests/provider-detection.test.ts`

## Dependencies
- None (Phase 1 is the foundation for all subsequent phases)

## Risk Assessment
- **Risk**: Test file format incorrect (wrong extension or structure)
  - **Mitigation**: Use `.test.ts` extension, follow existing test patterns in codebase

- **Risk**: Mocking strategy unclear or incorrect
  - **Mitigation**: Examples provided in quality-strategy.md, reference provider-detection.test.ts

- **Risk**: CommonJS mocking from ESM tests doesn't work as expected
  - **Mitigation**: Early checkpoint test verifies mocking works before writing all tests
  - **Contingency Plan** (if mocking proves difficult):
    1. Extract functions to `src/utils/docker-detection.ts` (TypeScript/ESM)
    2. Have `bin/cli.cjs` import these utilities
    3. Test the TypeScript module directly (easier mocking)
    4. Estimated adjustment time: 1-2 hours
    5. Would require updating architecture.md and plan.md

- **Risk**: Tests too implementation-specific (brittle tests)
  - **Mitigation**: Focus on behavior and contracts, not internal implementation details

- **Risk**: Tests don't actually fail as expected
  - **Mitigation**: Run `pnpm test workspace-path-detection` and capture output to verify failure state

## Files/Packages Affected
**Files to Create:**
- `/workspace/packages/maproom-mcp/tests/utils/workspace-path-detection.test.ts` (new file)
- `/workspace/packages/maproom-mcp/tests/integration/workspace-path-detection.int.test.ts` (new file)

**Directories to Verify/Create:**
- `/workspace/packages/maproom-mcp/tests/utils/` (may need to create)
- `/workspace/packages/maproom-mcp/tests/integration/` (may need to create)

**No files modified** - this ticket only creates new test files.
