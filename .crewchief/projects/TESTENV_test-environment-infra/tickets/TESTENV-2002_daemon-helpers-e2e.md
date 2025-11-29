# Ticket: TESTENV-2002: Create daemon test helpers and configure E2E test suite

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - tests executed and passing (or N/A if no tests)
- [ ] **Verified** - by the verify-ticket agent

**Note on "Tests pass"**: Run E2E tests with daemon available to verify helpers work.

## Agents
- typescript-engineer
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary
Create TypeScript helper functions for daemon availability detection and health checking, then configure a separate E2E test suite that requires the real daemon for indexing tests.

## Background
With the daemon available via Docker Compose (TESTENV-2001), we need TypeScript utilities to detect daemon availability and wait for health checks. This enables E2E tests that verify real indexing behavior, separate from the fixture-based integration tests.

Reference: [plan.md](../planning/plan.md) - Phase 2, Deliverables 2-3: "E2E Test Helpers" and "E2E Test Suite"
Reference: [architecture.md](../planning/architecture.md) - "E2E Test Integration" section

## Acceptance Criteria
- [ ] `tests/helpers/daemon.ts` created with helper functions
- [ ] `isDaemonAvailable()` detects `MAPROOM_DAEMON_URL` environment variable
- [ ] `getDaemonUrl()` returns URL or throws if not available
- [ ] `waitForDaemon()` polls health endpoint until healthy
- [ ] E2E test file created at `tests/e2e/real-indexing.test.ts`
- [ ] E2E tests skip automatically when daemon not available
- [ ] E2E tests pass when daemon is running

## Technical Requirements

### Daemon Helper Functions
Create `packages/maproom-mcp/tests/helpers/daemon.ts`:

```typescript
/**
 * Daemon test helpers for E2E testing.
 *
 * Usage:
 * - Set MAPROOM_DAEMON_URL environment variable to enable E2E tests
 * - Local: MAPROOM_DAEMON_URL=http://localhost:3000
 * - CI: MAPROOM_DAEMON_URL=http://maproom-daemon:3000
 */

/**
 * Check if daemon is available based on environment variable.
 * Used for skipIf conditions in E2E tests.
 */
export function isDaemonAvailable(): boolean {
  return process.env.MAPROOM_DAEMON_URL !== undefined
}

/**
 * Get daemon URL, throwing if not available.
 * Use after checking isDaemonAvailable().
 */
export function getDaemonUrl(): string {
  const url = process.env.MAPROOM_DAEMON_URL
  if (!url) {
    throw new Error(
      'MAPROOM_DAEMON_URL not set. ' +
      'To run E2E tests: docker compose --profile e2e up -d && ' +
      'MAPROOM_DAEMON_URL=http://localhost:3000 pnpm test'
    )
  }
  return url
}

/**
 * Wait for daemon to become healthy before running tests.
 * Called at E2E test suite setup.
 */
export async function waitForDaemon(timeoutMs = 30000): Promise<void> {
  const url = getDaemonUrl()
  const healthUrl = `${url}/health`
  const startTime = Date.now()
  const pollInterval = 1000

  while (Date.now() - startTime < timeoutMs) {
    try {
      const response = await fetch(healthUrl)
      if (response.ok) {
        console.log(`✅ Daemon healthy at ${url}`)
        return
      }
    } catch (error) {
      // Daemon not ready yet, continue polling
    }
    await new Promise(resolve => setTimeout(resolve, pollInterval))
  }

  throw new Error(
    `Daemon at ${url} did not become healthy after ${timeoutMs}ms. ` +
    `Check: docker logs maproom-daemon`
  )
}

/**
 * Utility to sleep for a specified duration.
 */
export function sleep(ms: number): Promise<void> {
  return new Promise(resolve => setTimeout(resolve, ms))
}
```

### E2E Test Suite
Create `packages/maproom-mcp/tests/e2e/real-indexing.test.ts`:

```typescript
import { describe, it, expect, beforeAll } from 'vitest'
import { isDaemonAvailable, waitForDaemon, getDaemonUrl } from '../helpers/daemon'
import { getTestClient } from '../helpers/database'

/**
 * E2E tests that require the real maproom daemon.
 *
 * These tests are SKIPPED by default. To run:
 *   docker compose --profile e2e up -d
 *   MAPROOM_DAEMON_URL=http://localhost:3000 pnpm test
 */
describe.skipIf(!isDaemonAvailable())('E2E: Real Indexing', () => {
  beforeAll(async () => {
    await waitForDaemon()
  })

  it('daemon responds to health check', async () => {
    const url = getDaemonUrl()
    const response = await fetch(`${url}/health`)
    expect(response.ok).toBe(true)
  })

  it('daemon can index a file', async () => {
    const url = getDaemonUrl()
    // TODO: Implement real indexing test
    // This would trigger indexing via daemon API and verify chunks are created
  })

  it('daemon indexes produce searchable chunks', async () => {
    const client = await getTestClient()
    // TODO: Verify that daemon-indexed chunks appear in database
  })
})
```

### Environment Variable Configuration

| Environment | `MAPROOM_DAEMON_URL` | Behavior |
|-------------|---------------------|----------|
| Local (normal) | *not set* | E2E tests skip |
| Local (e2e) | `http://localhost:3000` | E2E tests run |
| CI (normal) | *not set* | E2E tests skip |
| CI (e2e job) | `http://maproom-daemon:3000` | E2E tests run |

## Implementation Notes

1. **Skip by default** - E2E tests must skip when `MAPROOM_DAEMON_URL` is not set. This ensures normal test runs aren't affected.

2. **Timeout handling** - `waitForDaemon()` should have reasonable timeout (30s) with clear error messages.

3. **Health check URL** - Daemon exposes `/health` endpoint per existing Dockerfile configuration.

4. **Test isolation** - E2E tests should clean up after themselves or use separate test data.

5. **Error messages** - Include helpful instructions for developers who encounter skipped tests.

6. **Vitest config** - May need to add e2e test pattern to vitest config if not already included:
   ```typescript
   // vitest.config.ts
   include: ['tests/**/*.test.ts']
   ```

## Dependencies
- TESTENV-1006 (Phase 1 complete)
- TESTENV-2001 (daemon service in Docker Compose)

## Risk Assessment
- **Risk**: E2E tests accidentally run in CI without daemon
  - **Mitigation**: `skipIf` ensures tests are skipped when daemon unavailable
- **Risk**: Health check flaky
  - **Mitigation**: Configurable timeout, multiple retries in `waitForDaemon()`
- **Risk**: E2E tests slow down normal test runs
  - **Mitigation**: E2E tests skip by default; only run when explicitly enabled

## Files/Packages Affected
- `packages/maproom-mcp/tests/helpers/daemon.ts` (NEW)
- `packages/maproom-mcp/tests/e2e/real-indexing.test.ts` (NEW)
- `packages/maproom-mcp/tests/e2e/` directory (NEW)
