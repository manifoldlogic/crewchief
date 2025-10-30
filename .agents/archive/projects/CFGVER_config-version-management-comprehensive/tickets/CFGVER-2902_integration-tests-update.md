# Ticket: CFGVER-2902: Create integration tests for complete update process

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - related tests pass
- [ ] **Verified** - by the verify-ticket agent

## Agents
- integration-tester
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary
Create integration tests that validate the end-to-end update flow: backup → update → rollback. These tests provide confidence that the complete process works correctly and safely handles failures.

## Background
Phase 2 implements the critical update path that users will rely on. Unit tests validate individual functions, but integration tests validate the complete workflow:
- First run creates configs correctly
- Updates replace configs and create backups
- User customizations are preserved
- Rollback works when failures occur
- Cleanup removes old backups

These tests use real file system operations (not mocks) to catch integration issues that unit tests miss.

Reference: `quality-strategy.md` lines 86-152 for integration test structure and scenarios.

## Acceptance Criteria
- [ ] Test: "First run creates all config files correctly" - validates initial setup
- [ ] Test: "Version update replaces configs and creates backup" - validates update flow
- [ ] Test: "User .env preserved during update" - validates customization preservation
- [ ] Test: "Rollback works after simulated failure" - validates recovery mechanism
- [ ] Test: "Cleanup removes old backups" - validates cleanup logic
- [ ] All tests use real file system (not memfs)
- [ ] Each test creates isolated temporary cache directory
- [ ] Cleanup runs after each test (teardown)

## Technical Requirements
- Test file location: `packages/maproom-mcp/tests/integration/update-flow.test.ts`
- Use Node.js `fs` module (real file system, not mocks)
- Use Node.js `os.tmpdir()` or similar for temporary directories
- Create unique temp directory per test: `${tmpdir}/maproom-test-${Date.now()}/`
- Set `process.env.HOME` or similar to point to temp directory during tests
- Clean up temp directories in `afterEach()` hook
- Use Vitest or Jest test framework (match existing test setup)

## Implementation Notes
**Test File Structure:**
```javascript
import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import fs from 'fs';
import path from 'path';
import os from 'os';
import {
  backupConfigs,
  copyNewConfigs,
  rollbackConfigs,
  cleanupOldBackups
} from '../../src/config-manager.ts';

describe('Update Flow Integration Tests', () => {
  let testCacheDir;

  beforeEach(() => {
    // Create isolated temp directory for this test
    testCacheDir = path.join(os.tmpdir(), `maproom-test-${Date.now()}`);
    fs.mkdirSync(testCacheDir, { recursive: true, mode: 0o700 });

    // Override cache directory for test
    process.env.MAPROOM_CACHE_DIR = testCacheDir;
  });

  afterEach(() => {
    // Clean up temp directory
    if (fs.existsSync(testCacheDir)) {
      fs.rmSync(testCacheDir, { recursive: true, force: true });
    }
  });

  // Tests go here
});
```

**Test Scenarios:**

### Test 1: First Run Creates All Config Files
From `quality-strategy.md` lines 92-101:
- **Setup**: Empty cache directory
- **Act**: Run `copyNewConfigs('1.2.3')`
- **Assert**:
  * All files exist (docker-compose.yml, init.sql, Dockerfile.mcp-server, .maproom-version)
  * Version file contains correct package version
  * File permissions are 0o600
  * Directory permissions are 0o700

### Test 2: Version Update
From `quality-strategy.md` lines 102-113:
- **Setup**:
  * Create old config (version 1.2.2)
  * Modify docker-compose.yml (add comment to detect change)
- **Act**:
  * Run `backupConfigs()`
  * Run `copyNewConfigs('1.2.3')`
- **Assert**:
  * New version in .maproom-version
  * docker-compose.yml updated (old comment gone)
  * Backup directory exists with old config
  * Backup contains old docker-compose.yml with comment

### Test 3: Preserve User .env
From `quality-strategy.md` lines 114-123:
- **Setup**:
  * Create config with user .env file (custom content)
- **Act**:
  * Run `copyNewConfigs('1.2.4')`
- **Assert**:
  * .env file unchanged (custom content preserved)
  * Other files updated

### Test 4: Rollback After Failure
From `quality-strategy.md` lines 124-135:
- **Setup**:
  * Create config (version 1.2.3)
  * Run `backupConfigs()` to create backup
  * Simulate failure by corrupting config files
- **Act**:
  * Run `rollbackConfigs(backupDir)`
- **Assert**:
  * Original config restored
  * Version file matches backup
  * All files have correct content

### Test 5: Cleanup Old Backups
From `quality-strategy.md` lines 136-152:
- **Setup**:
  * Create 10 backup directories with different timestamps
- **Act**:
  * Run `cleanupOldBackups()`
- **Assert**:
  * Only 5 most recent backups remain
  * Oldest 5 backups deleted
  * Backups sorted by timestamp correctly

**Helper Functions:**
```javascript
// Create fake config files for testing
function createTestConfig(cacheDir, version, customContent = {}) {
  const dockerCompose = customContent.dockerCompose || 'version: "3.8"';
  const initSql = customContent.initSql || 'CREATE TABLE test();';

  fs.writeFileSync(path.join(cacheDir, 'docker-compose.yml'), dockerCompose);
  fs.writeFileSync(path.join(cacheDir, 'init.sql'), initSql);
  // ... etc
}

// Create fake backup directory
function createTestBackup(cacheDir, timestamp) {
  const backupDir = path.join(cacheDir, 'backups', timestamp);
  fs.mkdirSync(backupDir, { recursive: true, mode: 0o700 });
  // ... copy files
  return backupDir;
}
```

**Quality Strategy Reference:**
All test scenarios from `quality-strategy.md` lines 86-152:
- Integration tests section provides complete test descriptions
- Each test scenario has setup, action, and assertion details
- Tests cover happy path and error cases

## Dependencies
- **CFGVER-0001** - Vitest testing infrastructure must be installed first (CRITICAL)
- CFGVER-2001 (backup function)
- CFGVER-2002 (update function)
- CFGVER-2003 (rollback function)
- CFGVER-2004 (cleanup function)

## Risk Assessment
- **Risk**: Flaky tests - temp directory conflicts between parallel tests
  - **Mitigation**: Use timestamp + random number for temp directory names
  - **Severity**: Medium (CI failures)

- **Risk**: Cleanup failures - temp directories not removed after test failures
  - **Mitigation**: Use `afterEach()` with `force: true` to ensure cleanup
  - **Severity**: Low (disk space accumulation on test machine)

- **Risk**: Platform differences - permissions behave differently on Windows
  - **Mitigation**: Skip permission tests on Windows or adjust expectations
  - **Severity**: Low (Windows not primary target)

## Files/Packages Affected
- **Create**: `packages/maproom-mcp/tests/integration/update-flow.test.ts`
- **Read**: Functions from `packages/maproom-mcp/src/config-manager.js`
- **Create/Delete**: Temporary test directories in `os.tmpdir()`
