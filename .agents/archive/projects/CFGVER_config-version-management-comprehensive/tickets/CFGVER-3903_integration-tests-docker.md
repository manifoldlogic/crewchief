# Ticket: CFGVER-3903: Create integration tests for Docker container management

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - related tests pass
- [ ] **Verified** - by the verify-ticket agent

## Agents
- integration-tester
- code-reviewer
- test-runner
- verify-ticket
- commit-ticket

## Summary
Create comprehensive integration tests for Docker container stop, volume cleanup, and error handling. Tests must work with real Docker daemon to verify actual behavior, with graceful skip when Docker unavailable for CI flexibility.

## Background
Docker operations are complex and platform-specific. Unit tests with mocks don't catch real-world issues like permission errors, timeout behavior, or platform-specific Docker quirks. Integration tests with real Docker are essential to verify container management works correctly.

Tests must be skippable when Docker unavailable (CI environments, developer machines without Docker) while still providing comprehensive validation when Docker is present.

Reference: Quality strategy document for integration test approach and Docker testing best practices.

## Acceptance Criteria
- [ ] Test suite runs with real Docker daemon when available
- [ ] Tests skip gracefully with clear message when Docker not available
- [ ] Test: Container stop succeeds when Docker running
- [ ] Test: Volume cleanup removes only Maproom-labeled volumes
- [ ] Test: Docker not running handled gracefully (mocked error)
- [ ] Test: Error messages are clear and actionable
- [ ] Tests create and cleanup all test resources
- [ ] Tests run in CI with Docker available

## Technical Requirements
- **Test File:** `packages/maproom-mcp/tests/integration/docker-operations.test.ts`
- **Test Framework:** Vitest
- **Requirements:**
  - Check Docker availability in `beforeAll`
  - Skip entire suite if Docker not found
  - Create unique labels for test resources: `com.crewchief.maproom.test=true`
  - Cleanup ALL test resources in `afterEach` and `afterAll`
  - Use environment variable: `CI_SKIP_DOCKER=1` to force skip in CI

## Implementation Notes
**Test Suite Structure:**

```javascript
import { describe, test, expect, beforeAll, afterAll, afterEach } from 'vitest';
import { execFile } from 'child_process';
import { promisify } from 'util';
import path from 'path';
import fs from 'fs';
import { stopContainers, cleanupOldVolumes, checkDockerAvailable } from '../src/config-manager.js';

const execFileAsync = promisify(execFile);

describe('Docker Operations Integration Tests', () => {
  let dockerAvailable = false;
  let testComposeFile;
  let testCacheDir;

  beforeAll(async () => {
    // Skip tests if CI_SKIP_DOCKER set
    if (process.env.CI_SKIP_DOCKER === '1') {
      console.log('Skipping Docker tests (CI_SKIP_DOCKER=1)');
      return;
    }

    // Check if Docker available
    const result = await checkDockerAvailable();
    dockerAvailable = result.available;

    if (!dockerAvailable) {
      console.log(`Skipping Docker tests: ${result.reason}`);
      return;
    }

    // Setup test environment
    testCacheDir = path.join('/tmp', `maproom-test-${Date.now()}`);
    fs.mkdirSync(testCacheDir, { recursive: true });

    // Create test docker-compose.yml
    testComposeFile = path.join(testCacheDir, 'docker-compose.yml');
    fs.writeFileSync(testComposeFile, `
version: '3.8'
services:
  test-postgres:
    image: postgres:15-alpine
    environment:
      POSTGRES_PASSWORD: test
    labels:
      - com.crewchief.maproom.test=true
volumes:
  test-pgdata:
    labels:
      - com.crewchief.maproom=true
      - com.crewchief.maproom.test=true
`);
  });

  afterAll(async () => {
    if (!dockerAvailable) return;

    // Cleanup test resources
    try {
      await execFileAsync('docker', [
        'compose',
        '-f', testComposeFile,
        'down',
        '-v'
      ], { cwd: testCacheDir });

      // Remove any leftover test volumes
      await execFileAsync('docker', [
        'volume', 'prune', '-f',
        '--filter', 'label=com.crewchief.maproom.test=true'
      ]);

      // Remove test directory
      fs.rmSync(testCacheDir, { recursive: true, force: true });
    } catch (error) {
      console.error('Cleanup error:', error);
    }
  });

  afterEach(async () => {
    if (!dockerAvailable) return;

    // Ensure containers stopped after each test
    try {
      await execFileAsync('docker', [
        'compose',
        '-f', testComposeFile,
        'down'
      ], { cwd: testCacheDir });
    } catch (error) {
      // Ignore errors
    }
  });

  describe('Container Stop', () => {
    test('stops containers successfully when running', async () => {
      if (!dockerAvailable) return;

      // Start test containers
      await execFileAsync('docker', [
        'compose',
        '-f', testComposeFile,
        'up', '-d'
      ], { cwd: testCacheDir });

      // Wait for containers to start
      await new Promise(resolve => setTimeout(resolve, 2000));

      // Stop containers
      const result = await stopContainers();

      expect(result.success).toBe(true);
      expect(result.skipped).toBeUndefined();

      // Verify containers stopped
      const { stdout } = await execFileAsync('docker', [
        'compose',
        '-f', testComposeFile,
        'ps', '-q'
      ], { cwd: testCacheDir });

      expect(stdout.trim()).toBe('');
    });

    test('handles no containers gracefully', async () => {
      if (!dockerAvailable) return;

      // Ensure no containers running
      await execFileAsync('docker', [
        'compose',
        '-f', testComposeFile,
        'down'
      ], { cwd: testCacheDir });

      // Stop should succeed even with no containers
      const result = await stopContainers();

      expect(result.success).toBe(true);
    });

    test('respects timeout for stuck containers', async () => {
      if (!dockerAvailable) return;

      // This test would need a container that ignores SIGTERM
      // For now, just verify timeout is configured
      // Real implementation would create a container with trap signal handler
    });
  });

  describe('Volume Cleanup', () => {
    test('removes only Maproom-labeled volumes', async () => {
      if (!dockerAvailable) return;

      // Create test volume with label
      await execFileAsync('docker', [
        'volume', 'create',
        '--label', 'com.crewchief.maproom=true',
        '--label', 'com.crewchief.maproom.test=true',
        'test-maproom-volume'
      ]);

      // Create volume without label (should not be removed)
      await execFileAsync('docker', [
        'volume', 'create',
        'test-other-volume'
      ]);

      // Cleanup Maproom volumes
      const result = await cleanupOldVolumes();

      expect(result.success).toBe(true);

      // Verify Maproom volume removed
      try {
        await execFileAsync('docker', ['volume', 'inspect', 'test-maproom-volume']);
        expect.fail('Maproom volume should have been removed');
      } catch (error) {
        expect(error.message).toContain('No such volume');
      }

      // Verify other volume still exists
      const { stdout } = await execFileAsync('docker', ['volume', 'inspect', 'test-other-volume']);
      expect(stdout).toContain('test-other-volume');

      // Cleanup
      await execFileAsync('docker', ['volume', 'rm', 'test-other-volume']);
    });

    test('handles no volumes to cleanup gracefully', async () => {
      if (!dockerAvailable) return;

      const result = await cleanupOldVolumes();

      expect(result.success).toBe(true);
      expect(result.reclaimed).toMatch(/0B|0 B/);
    });
  });

  describe('Docker Error Handling', () => {
    test('detects Docker availability correctly', async () => {
      const result = await checkDockerAvailable();

      if (dockerAvailable) {
        expect(result.available).toBe(true);
      } else {
        expect(result.available).toBe(false);
        expect(result.reason).toBeDefined();
        expect(result.suggestion).toBeDefined();
      }
    });

    test('provides actionable error messages', async () => {
      // This would test error message formatting
      // Real implementation would mock execFile errors
      const errorCases = [
        {
          error: { code: 'ENOENT' },
          expectedReason: 'Docker not installed',
          expectedSuggestion: 'https://docker.com'
        },
        {
          error: { code: 'EACCES', stderr: 'permission denied' },
          expectedReason: 'Docker permission denied',
          expectedSuggestion: 'usermod -aG docker'
        },
        {
          error: { stderr: 'Cannot connect to the Docker daemon' },
          expectedReason: 'Docker daemon not running',
          expectedSuggestion: 'systemctl start docker'
        }
      ];

      // Test parseDockerError function with each case
      // (Would need to export parseDockerError or test via public API)
    });

    test('caches Docker availability check', async () => {
      if (!dockerAvailable) return;

      const start = Date.now();
      await checkDockerAvailable();
      const firstCheck = Date.now() - start;

      const start2 = Date.now();
      await checkDockerAvailable();
      const secondCheck = Date.now() - start2;

      // Second check should be much faster (cached)
      expect(secondCheck).toBeLessThan(firstCheck / 2);
    });
  });

  describe('Integration: Full Update Flow', () => {
    test('stops containers and cleans volumes in sequence', async () => {
      if (!dockerAvailable) return;

      // Start containers
      await execFileAsync('docker', [
        'compose',
        '-f', testComposeFile,
        'up', '-d'
      ], { cwd: testCacheDir });

      await new Promise(resolve => setTimeout(resolve, 2000));

      // Full flow: stop then cleanup
      const stopResult = await stopContainers();
      expect(stopResult.success).toBe(true);

      const cleanupResult = await cleanupOldVolumes();
      expect(cleanupResult.success).toBe(true);

      // Verify clean state
      const { stdout } = await execFileAsync('docker', [
        'compose',
        '-f', testComposeFile,
        'ps', '-q'
      ], { cwd: testCacheDir });

      expect(stdout.trim()).toBe('');
    });
  });
});
```

**CI Configuration:**

Add to `.github/workflows/test.yml`:
```yaml
- name: Run integration tests
  run: pnpm test:integration
  env:
    CI_SKIP_DOCKER: ${{ matrix.os == 'windows-latest' && '1' || '0' }}
```

**Test Execution:**

```bash
# Run all tests including Docker integration
pnpm test

# Run only Docker integration tests
pnpm test tests/integration/docker-operations.test.js

# Skip Docker tests
CI_SKIP_DOCKER=1 pnpm test

# Run with verbose output
pnpm test --reporter=verbose tests/integration/docker-operations.test.js
```

## Dependencies
- **CFGVER-0001** - Vitest testing infrastructure must be installed first (CRITICAL)
- CFGVER-3001 (container stop implementation)
- CFGVER-3002 (volume cleanup implementation)
- CFGVER-3003 (error handling implementation)

## Risk Assessment
- **Risk**: Tests modify user's Docker environment
  - **Mitigation**: Use unique labels, cleanup all test resources, never use production resource names
  - **Severity**: High (could affect user's containers)

- **Risk**: Test pollution (leftover containers/volumes)
  - **Mitigation**: Comprehensive cleanup in afterEach and afterAll, use unique test labels
  - **Severity**: Medium (disk space usage)

- **Risk**: CI fails when Docker not available
  - **Mitigation**: Graceful skip with clear message, CI_SKIP_DOCKER environment variable
  - **Severity**: Low (expected in some CI environments)

- **Risk**: Tests are slow (waiting for containers)
  - **Mitigation**: Use alpine images, minimize wait times, run selectively
  - **Severity**: Low (acceptable for integration tests)

## Files/Packages Affected
- **Create**: `packages/maproom-mcp/tests/integration/docker-operations.test.ts`
- **Modify**: `packages/maproom-mcp/package.json` (add test:integration script if needed)
- **Modify**: `.github/workflows/test.yml` (CI configuration for Docker tests)
