# Ticket: TESTENV-1004: Integrate fixtures into test setup

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - tests executed and passing (or N/A if no tests)
- [ ] **Verified** - by the verify-ticket agent

**Note on "Tests pass"**: Run `pnpm test` in packages/maproom-mcp to verify fixture loading works.

## Agents
- typescript-engineer
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary
Update the Vitest globalSetup (`ensure-test-db.ts`) to automatically load test fixtures after schema initialization, enabling tests to run against pre-indexed data.

## Background
The test database setup currently initializes the schema but doesn't populate data. Tests that need indexed data fail because there's no test corpus. This ticket adds fixture loading to the globalSetup so all tests have access to deterministic, pre-indexed data.

Reference: [plan.md](../planning/plan.md) - Phase 1, Deliverable 4: "Enhanced Test Setup"

## Acceptance Criteria
- [ ] `ensure-test-db.ts` loads fixtures after schema initialization
- [ ] Fixture loading is idempotent (safe to run multiple times)
- [ ] Fixture loading completes in <50ms
- [ ] `isTestCorpusLoaded()` function detects if fixtures are already present
- [ ] `loadTestFixtures()` function loads fixtures via psql
- [ ] Verification query confirms expected data exists
- [ ] Tests can be run without manual fixture loading

## Technical Requirements

### Updated ensure-test-db.ts Structure
```typescript
// packages/maproom-mcp/tests/setup/ensure-test-db.ts

import { execSync } from 'child_process'
import { readFileSync, existsSync } from 'fs'
import { resolve } from 'path'

const CONTAINER_NAME = 'postgres-test-maproom'
const SCHEMA_FILE = resolve(__dirname, 'init-schema.sql')
const FIXTURE_FILE = resolve(__dirname, 'test-fixtures.sql')

export async function setup(): Promise<void> {
  // Step 1: Ensure container is running (existing)
  ensureContainerRunning()

  // Step 2: Initialize schema (existing)
  if (!isSchemaInitialized()) {
    initializeSchema()
  }

  // Step 3: Load fixtures (NEW)
  if (!isTestCorpusLoaded()) {
    loadTestFixtures()
  }

  // Step 4: Verify (NEW)
  verifyTestCorpus()
}

function isTestCorpusLoaded(): boolean {
  try {
    const result = execSync(
      `docker exec ${CONTAINER_NAME} psql -U maproom -d maproom_test -t -c "SELECT COUNT(*) FROM maproom.repos WHERE name = 'test-corpus'"`,
      { encoding: 'utf-8', stdio: ['pipe', 'pipe', 'pipe'] }
    )
    return parseInt(result.trim(), 10) > 0
  } catch {
    return false
  }
}

function loadTestFixtures(): void {
  if (!existsSync(FIXTURE_FILE)) {
    console.warn('⚠️  Fixture file not found, skipping fixture load')
    return
  }

  console.log('📦 Loading test fixtures...')
  const startTime = Date.now()

  const fixtureSQL = readFileSync(FIXTURE_FILE, 'utf-8')
  execSync(
    `docker exec -i ${CONTAINER_NAME} psql -U maproom -d maproom_test`,
    { input: fixtureSQL, stdio: ['pipe', 'pipe', 'pipe'], encoding: 'utf-8' }
  )

  const loadTime = Date.now() - startTime
  console.log(`✅ Fixtures loaded in ${loadTime}ms`)
}

function verifyTestCorpus(): void {
  const result = execSync(
    `docker exec ${CONTAINER_NAME} psql -U maproom -d maproom_test -t -c "SELECT COUNT(*) FROM maproom.chunks"`,
    { encoding: 'utf-8', stdio: ['pipe', 'pipe', 'pipe'] }
  )
  const chunkCount = parseInt(result.trim(), 10)

  if (chunkCount < 50) {
    console.warn(`⚠️  Low chunk count: ${chunkCount}. Expected ~100 chunks.`)
  } else {
    console.log(`✅ Test corpus verified: ${chunkCount} chunks`)
  }
}
```

### Database Helper Updates
Also update `tests/helpers/database.ts` if needed to export fixture-related utilities:
```typescript
// Add to database.ts
export async function reloadFixtures(): Promise<void> {
  // For tests that need fresh fixtures
}

export async function getTestCorpusChunkCount(): Promise<number> {
  // Utility for verification
}
```

## Implementation Notes

1. **Preserve existing functionality** - Don't break current schema initialization logic

2. **Make fixture loading optional** - If fixture file doesn't exist, warn but don't fail (allows incremental development)

3. **Log timing** - Measure and log fixture load time to catch performance regressions

4. **Use docker exec** - Consistent with existing approach; don't introduce new dependencies

5. **Handle CI vs Local**:
   - Local: Uses docker compose container
   - CI: Uses service container (may need different container name)

6. **Idempotency strategy**:
   - Check if test-corpus repo exists
   - If exists, skip loading (fixtures already present)
   - If not, load fixtures

## Dependencies
- TESTENV-1001 (test corpus files)
- TESTENV-1002 (fixture generation script)
- TESTENV-1003 (generated fixtures file)

## Risk Assessment
- **Risk**: Breaking existing tests during refactor
  - **Mitigation**: Run full test suite before and after changes
- **Risk**: Different container names in CI vs local
  - **Mitigation**: Use environment variable for container name
- **Risk**: Fixture load slow in CI
  - **Mitigation**: Target <50ms load time; keep fixtures small

## Files/Packages Affected
- `packages/maproom-mcp/tests/setup/ensure-test-db.ts` (MODIFY)
- `packages/maproom-mcp/tests/helpers/database.ts` (MODIFY - optional)
