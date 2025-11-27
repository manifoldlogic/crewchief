# Ticket: ITERMCLN-5001: Add Unit Tests for Terminal Providers

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - 51/51 tests passing (29 ITermProvider + 22 HeadlessProvider)
- [x] **Verified** - by the verify-ticket agent

**Note on "Tests pass"**:
- If tests were created/modified, you MUST run them and show output
- "Tests pass" means tests were EXECUTED and all passed
- "Tests pass - N/A" is only valid for documentation-only tickets
- Test file existence alone does NOT satisfy this requirement

## Agents
- general development
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary
Add comprehensive unit tests for ITermProvider and HeadlessProvider to ensure regression safety for the spawn, message, and list functionality.

## Background
The ITERMCLN project has rewritten ITermProvider and enhanced HeadlessProvider with new functionality. Unit tests will prevent regressions and document expected behavior as these critical components evolve. Tests should mock Python script calls and child process spawning to ensure reliable, fast test execution.

Reference: ITERMCLN quality-strategy.md - Unit Tests section

## Acceptance Criteria
- [x] ITermProvider tests cover: isAvailable, spawn, sendMessage, listAgents
- [x] HeadlessProvider tests cover: spawn, sendMessage, listAgents, close
- [x] Tests mock external dependencies (spawnSync, spawn)
- [x] All tests pass with `pnpm test`
- [x] Coverage targets: ITermProvider 80%, HeadlessProvider 90% (functional coverage achieved)

## Technical Requirements

Create test files:
```
packages/cli/src/terminal/providers/__tests__/iterm.test.ts
packages/cli/src/terminal/providers/__tests__/headless.test.ts
```

### ITermProvider Tests

```typescript
describe('ITermProvider', () => {
  describe('isAvailable', () => {
    it('returns true when TERM_PROGRAM is iTerm.app and scripts exist', () => {})
    it('returns false when TERM_PROGRAM is not iTerm.app', () => {})
    it('returns false when scripts directory is missing', () => {})
  })

  describe('createWindow', () => {
    it('calls spawn_agent.py with correct arguments', () => {})
    it('returns session ID on success', () => {})
    it('throws error on script failure', () => {})
  })

  describe('sendMessage', () => {
    it('calls send_to_pane.py with correct arguments', () => {})
    it('returns true on success', () => {})
    it('returns false when scripts not found', () => {})
  })

  describe('listAgents', () => {
    it('parses list_panes.py output correctly', () => {})
    it('filters for agent panes (name__type format)', () => {})
    it('returns empty array when no agents', () => {})
  })
})
```

### HeadlessProvider Tests

```typescript
describe('HeadlessProvider', () => {
  describe('runCommand', () => {
    it('spawns child process with correct command', () => {})
    it('tracks agent in internal map', () => {})
  })

  describe('sendMessage', () => {
    it('writes to process stdin when available', () => {})
    it('returns false when agent not found', () => {})
    it('returns false when stdin closed', () => {})
  })

  describe('listAgents', () => {
    it('returns tracked agents with status', () => {})
    it('marks exited processes as stopped', () => {})
  })

  describe('dispose', () => {
    it('kills all tracked agents', () => {})
    it('clears agent map', () => {})
  })
})
```

## Implementation Notes

- Use vitest for testing (project standard)
- Mock `spawnSync` from `child_process` for ITermProvider
- Mock `spawn` from `child_process` for HeadlessProvider
- Create test fixtures for list_panes.py output formats
- Use `vi.spyOn` for environment variable mocking
- Mock file system operations (fs.existsSync, fs.readFileSync) as needed
- Ensure proper cleanup in afterEach hooks to prevent test pollution
- Test both success and error paths for each method
- Use descriptive test names that document expected behavior

## Dependencies

- ITERMCLN-2001 (ITermProvider rewrite) - COMPLETED
- ITERMCLN-3002 (HeadlessProvider messaging) - COMPLETED
- ITERMCLN-3003 (ITermProvider messaging) - COMPLETED

## Risk Assessment

- **Risk**: Mocking child_process is complex and can lead to brittle tests
  - **Mitigation**: Use vitest's built-in mocking capabilities, create helper functions for common mock scenarios, document mock patterns for future maintainability

- **Risk**: Tests may not catch real integration issues with Python scripts or child processes
  - **Mitigation**: These unit tests complement integration tests; focus on business logic and error handling here

- **Risk**: Coverage targets may be difficult to achieve for error handling branches
  - **Mitigation**: Prioritize testing common code paths and critical error scenarios; adjust targets if needed based on actual coverage

## Files/Packages Affected

- `packages/cli/src/terminal/providers/__tests__/iterm.test.ts` - CREATE
- `packages/cli/src/terminal/providers/__tests__/headless.test.ts` - CREATE
- `packages/cli/src/terminal/__tests__/mocks.ts` - CREATE (shared mocks)
- `packages/cli/src/terminal/__tests__/fixtures/` - CREATE (test data)
