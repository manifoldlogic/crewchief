# Ticket: TESTFIX-1006: Fix VSCode Extension Tests

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
- vscode-extension-specialist
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary
Fix all 16 failing tests in the VSCode extension package (`packages/vscode-maproom`). Issues are primarily in orchestrator and integration tests related to process spawning and timeouts.

## Background
The VSCode extension has 16 failing tests out of 352 total. Failures are concentrated in `src/process/orchestrator.test.ts` and `src/test/integration.test.ts`. These tests involve spawning the maproom binary and waiting for responses, which may have timing or mock issues. This is Phase 4 of the TESTFIX project - TypeScript test fixes.

## Acceptance Criteria
- [ ] `pnpm test` in packages/vscode-maproom passes with 0 failures
- [ ] Orchestrator tests pass (process spawn handling fixed)
- [ ] Integration tests pass (end-to-end workflow verified)
- [ ] Test timeouts are appropriate (not too short, not excessively long)
- [ ] Mock infrastructure properly simulates extension environment

## Technical Requirements

**Process Spawn Issues:**
- Verify mock setup for child process spawning
- Ensure proper event handling (stdout, stderr, close)
- Handle async operations correctly with proper await/promise handling

**Timeout Handling:**
- Review timeout values in tests
- Add appropriate timeouts for async operations
- Use vitest's timeout configuration if needed

**VSCode Extension Context:**
- Ensure vscode mocks are properly configured
- Verify ExtensionContext mock provides required methods
- Check StatusBarItem and other UI element mocks

## Implementation Notes
1. Run `pnpm test` to get exact failure list
2. Analyze failure patterns:
   - Timeout failures: increase timeouts or fix async handling
   - Mock failures: update mock setup
   - Assertion failures: update expectations
3. Fix orchestrator.test.ts first (likely has most failures)
4. Fix integration.test.ts second
5. Verify with multiple test runs to catch flakiness

## Dependencies
- TESTFIX-1001 (clean environment)
- TESTFIX-1002 (baseline documented)

## Risk Assessment
- **Risk**: Tests depend on binary availability
  - **Mitigation**: Ensure mocks properly simulate binary behavior; document any binary requirements

- **Risk**: VSCode extension tests may require specific test runner
  - **Mitigation**: Check if `@vscode/test-electron` is needed; update test configuration

- **Risk**: Integration tests may be inherently flaky
  - **Mitigation**: Add retries or increase timeouts; document known flaky tests

## Files/Packages Affected
- `packages/vscode-maproom/src/process/orchestrator.test.ts`
- `packages/vscode-maproom/src/test/integration.test.ts`
- `packages/vscode-maproom/src/test/` (mock setup files)
- `packages/vscode-maproom/vitest.config.ts` (if timeout changes needed)
