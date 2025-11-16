# Ticket: VSMAP-4001: Add integration tests for core workflows

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
- process-management-specialist
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary
Create integration tests for Docker startup, process spawning, and status bar updates. Cover happy path and error scenarios to ensure production readiness.

## Background
This begins Phase 4 (Polish & Testing) of the VSMAP plan. While we have unit tests for individual modules, we need integration tests that verify the full workflows work together: extension activation → Docker startup → process spawning → event handling → UI updates. These tests catch integration bugs and provide confidence for release.

Reference: VSMAP_PLAN.md Phase 4 "Polish & Testing - Integration Tests"

## Acceptance Criteria
- [ ] Test: Extension activates successfully in test environment
- [ ] Test: Docker services start and become healthy
- [ ] Test: Watch processes spawn and run without crashing
- [ ] Test: Status bar updates correctly on events
- [ ] Test: Process crash triggers restart attempt
- [ ] Test coverage >50% overall (measure with c8 or nyc)
- [ ] All integration tests pass in CI environment

## Technical Requirements
- Use `@vscode/test-electron` for VSCode integration testing
- Mock Rust binary output for predictable test scenarios
- Use real Docker for service tests (requires Docker in test env)
- Spy on status bar updates using sinon or jest.spyOn
- Test process lifecycle: spawn → run → kill → verify cleanup
- Create test fixtures: sample NDJSON output, mock binary scripts
- Set test timeout to 30s for Docker startup tests

## Implementation Notes
Test structure using @vscode/test-electron:

```typescript
// src/test/integration/activation.test.ts
import * as vscode from 'vscode';
import { runTests } from '@vscode/test-electron';

suite('Extension Activation', () => {
  test('activates and starts Docker services', async () => {
    const ext = vscode.extensions.getExtension('maproom.vscode-maproom');
    await ext?.activate();

    // Wait for Docker services
    await waitForDockerHealth(10000);

    // Verify status bar shows "Starting..."
    // then "Watching"
  });
});
```

Mock binary for testing:
Create a mock script that emits predictable NDJSON:
```bash
#!/bin/bash
# test/fixtures/mock-binary.sh
echo '{"type":"progress","percent":50,"files":100}'
sleep 0.1
echo '{"type":"complete","files":100,"elapsed":1000}'
```

Configure spawner to use mock in tests:
```typescript
if (process.env.NODE_ENV === 'test') {
  binaryPath = path.join(__dirname, '../test/fixtures/mock-binary.sh');
}
```

Status bar testing:
```typescript
test('status bar updates on progress events', async () => {
  const statusBarSpy = sinon.spy(statusBar, 'update');

  // Trigger scan
  await scanWorkspace(context, workspacePath);

  // Verify status bar updated with progress
  expect(statusBarSpy).toHaveBeenCalledWith(
    expect.objectContaining({ files: 100 })
  );
});
```

Crash recovery testing:
```typescript
test('restarts process after crash', async () => {
  const crashingBinary = path.join(__dirname, '../fixtures/crash-binary.sh');

  // Spawn process that crashes immediately
  const process = spawner.spawn(crashingBinary);

  // Wait for restart attempt
  await waitForRestart(5000);

  // Verify restart was attempted
  expect(recovery.attemptCount).toBeGreaterThan(0);
});
```

Docker integration tests:
```typescript
test('Docker services start and become healthy', async () => {
  const docker = new DockerManager(output);
  await docker.ensureServices();

  const health = await docker.checkHealth();
  expect(health.postgres).toBe('healthy');
});
```

Coverage configuration (package.json):
```json
{
  "scripts": {
    "test:coverage": "c8 npm test",
    "test:integration": "node ./out/test/runTests.js"
  },
  "c8": {
    "reporter": ["text", "html"],
    "exclude": ["**/test/**"]
  }
}
```

## Dependencies
- All Phase 1-3 tickets must be complete (VSMAP-1001 through VSMAP-3002)
- Docker available in test environment
- @vscode/test-electron installed

## Risk Assessment
- **Risk**: Docker tests may be flaky in CI environment
  - **Mitigation**: Use retry logic, longer timeouts, skip if Docker unavailable
- **Risk**: Integration tests may be slow (>10s per test)
  - **Mitigation**: Use parallel test execution, optimize Docker startup
- **Risk**: Mock binaries may not accurately represent real binary behavior
  - **Mitigation**: Also include tests with real binary in controlled scenarios

## Files/Packages Affected
- `src/test/integration/activation.test.ts` (new file, ~100 lines)
- `src/test/integration/docker.test.ts` (new file, ~80 lines)
- `src/test/integration/processes.test.ts` (new file, ~120 lines)
- `test/fixtures/mock-binary.sh` (new fixture script)
- `test/fixtures/crash-binary.sh` (new fixture script)
- `package.json` (add test:integration script, c8 config)
- `.github/workflows/test.yml` (update to run integration tests)
