# Ticket: CFGVER-5002: Complete Integration Test Suite

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - related tests pass
- [ ] **Verified** - by the verify-ticket agent

## Agents
- integration-tester
- code-reviewer
- verify-ticket
- commit-ticket

## Summary
Ensure all critical paths covered by integration tests. This ticket consolidates integration testing from Phases 2 and 3, validating that the complete system works end-to-end with real file systems, real Docker operations, and realistic error scenarios.

## Background
Integration tests validate the complete system works end-to-end, not just isolated components. Unlike unit tests (which use memfs and mocks), integration tests use real file systems and real Docker operations to catch issues that only appear when components interact.

The quality strategy identifies critical paths that must be tested: first run config creation, version update with backup, user .env preservation, rollback on failure, Docker container management, and volume cleanup with filtering.

Reference: `quality-strategy.md` lines 86-152 for integration test scenarios.

## Acceptance Criteria
- [ ] All integration tests pass (from CFGVER-2902, CFGVER-3903)
- [ ] Critical path coverage complete:
  - [ ] First run config creation
  - [ ] Version update with backup
  - [ ] User .env preservation during update
  - [ ] Rollback on failure
  - [ ] Docker container management
  - [ ] Volume cleanup with filtering
  - [ ] Error handling scenarios
- [ ] Tests run in CI environment (with graceful skip for Docker if unavailable)
- [ ] Test fixtures documented
- [ ] Known limitations documented

## Technical Requirements

**Critical Path Coverage:**

1. **First Run Flow** (quality-strategy.md lines 143-152)
   - Empty cache directory
   - Creates all config files
   - Version file created with correct version
   - All file hashes match actual files
   - Containers can start

2. **Update Flow** (lines 154-167)
   - Old config with version 1.2.2
   - Run update to version 1.2.3
   - New files replace old files
   - Backup created with old config
   - Version file updated

3. **User .env Preservation** (lines 169-181)
   - Create user .env file with custom values
   - Run update
   - .env file contents unchanged

4. **Rollback Flow**
   - Simulated failure during update
   - Rollback restores original config
   - Containers can still start

5. **Docker Operations**
   - Stops running containers before update
   - Cleans up old volumes
   - Graceful skip if Docker not available

**Test Files:**
- `packages/maproom-mcp/tests/integration/update-flow.test.js`
- `packages/maproom-mcp/tests/integration/docker-operations.test.js`

**Test Environment:**
- Use real file system (not memfs)
- Use real Docker daemon (with graceful skip if unavailable)
- Cleanup after each test (remove temp directories, stop containers)
- Isolated temp directories per test

**CI Compatibility:**
Tests must handle CI environment differences:
- Docker may not be available → skip Docker tests with clear message
- File permissions may differ → test permission setting but allow variation
- Timing may vary → use appropriate timeouts

## Implementation Notes

**Review Existing Tests:**
1. Review: `packages/maproom-mcp/tests/integration/update-flow.test.js`
2. Review: `packages/maproom-mcp/tests/integration/docker-operations.test.js`

**Add Missing Scenarios:**
Identify gaps in current integration tests:
- Edge cases not covered
- Error scenarios not tested
- Timing-dependent failures

**Test Isolation:**
Each test must:
```javascript
beforeEach(() => {
  // Create temporary cache directory
  testCacheDir = path.join(os.tmpdir(), `maproom-test-${Date.now()}`);
  fs.mkdirSync(testCacheDir, { recursive: true });
});

afterEach(() => {
  // Cleanup: Remove temporary cache directory
  fs.rmSync(testCacheDir, { recursive: true, force: true });
});
```

**Docker Test Handling:**
```javascript
test('stops containers before update', async () => {
  // Check if Docker is available
  const dockerAvailable = await checkDockerAvailable();
  if (!dockerAvailable) {
    console.log('Skipping Docker test: Docker not available');
    return;
  }

  // Test Docker operations
});
```

**Reference Implementation:**
See `quality-strategy.md` lines 125-181 for complete test examples.

## Dependencies
- **CFGVER-0001** - Vitest testing infrastructure must be installed first (CRITICAL)
- CFGVER-2902 (update flow integration tests)
- CFGVER-3903 (Docker integration tests)
- All implementation tickets complete (Phase 1-4)

## Risk Assessment
- **Risk**: CI environment lacks Docker
  - **Mitigation**: Graceful skip with clear message, tests still valuable locally

- **Risk**: Timing-dependent failures (containers not stopped in time)
  - **Mitigation**: Use appropriate timeouts, retry logic for flaky operations

- **Risk**: Test cleanup failures leave artifacts
  - **Mitigation**: Use `force: true` in cleanup, temp directories with unique names

## Files/Packages Affected
- **Review**: `packages/maproom-mcp/tests/integration/update-flow.test.js`
- **Review**: `packages/maproom-mcp/tests/integration/docker-operations.test.js`
- **Modify**: Add missing test scenarios
- **Create**: Additional integration test files if needed for organization
