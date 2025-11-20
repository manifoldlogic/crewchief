# Quality Strategy: Test Database Isolation

## Testing Philosophy

**Goal**: Ship with confidence that test and dev databases are truly isolated, without over-testing infrastructure code.

**Approach**: Targeted validation of critical paths, not ceremonial coverage.

## Critical Paths

### Path 1: Database Service Startup
**Risk**: postgres-test fails to start or becomes unhealthy

**Test Strategy**:
- Docker health checks (built-in)
- Manual verification: `docker ps` shows both databases healthy
- CI validation: Workflow fails fast if postgres-test unhealthy

**Why No Unit Test**: Docker Compose handles this - we verify, not duplicate

### Path 2: Port Isolation
**Risk**: Tests accidentally connect to dev database

**Test Strategy**:
```bash
# Manual validation script
TEST_MAPROOM_DATABASE_URL=postgresql://maproom:maproom@localhost:5434/maproom_test pnpm test:integration

# Verify test data only in postgres-test:
docker exec maproom-postgres-test psql -U maproom -d maproom_test -c "SELECT COUNT(*) FROM maproom.chunks"
docker exec maproom-postgres psql -U maproom -d maproom -c "SELECT COUNT(*) FROM maproom.chunks"
```

**Expected**:
- Test database has test data
- Dev database unchanged

**Why Manual**: One-time verification, not regression risk

### Path 3: Configuration Propagation
**Risk**: TEST_MAPROOM_DATABASE_URL not reaching test code

**Test Strategy**:
- Existing test suite validates this implicitly
- If tests connect to wrong database, they fail
- No new tests needed - existing tests become validation

**Why No New Tests**: Self-validating system

### Path 4: CI/CD Integration
**Risk**: GitHub Actions still uses dev database

**Test Strategy**:
- Workflow logs show TEST_MAPROOM_DATABASE_URL value
- Tests pass in CI (implicit validation)
- Failed runs clearly indicate database connection issues

**Why No Special Tests**: CI is the test

## Test Coverage Targets

### What We Test

**Infrastructure Validation** (Scripts):
- Database health checks
- Port accessibility
- Volume isolation

**Configuration Correctness** (Smoke Tests):
- vitest.config.ts uses TEST_MAPROOM_DATABASE_URL
- package.json scripts set TEST_MAPROOM_DATABASE_URL
- Test helpers respect TEST_MAPROOM_DATABASE_URL priority

### What We Don't Test

**Docker Compose**: Assumes Docker works (not our responsibility)

**PostgreSQL**: Assumes database works (not our responsibility)

**Backward Compatibility**: If TEST_MAPROOM_DATABASE_URL not set, falls back to MAPROOM_DATABASE_URL (existing tests validate this)

## MVP Testing Milestones

### Milestone 1: Local Validation
**Deliverable**: Manual validation script

```bash
#!/bin/bash
# validate-test-isolation.sh

echo "Starting validation..."

# Step 1: Start infrastructure
docker compose up -d
docker compose ps

# Step 2: Wait for health
timeout 30 bash -c 'until docker compose ps | grep postgres-test | grep healthy; do sleep 1; done'

# Step 3: Run tests with TEST_MAPROOM_DATABASE_URL
TEST_MAPROOM_DATABASE_URL=postgresql://maproom:maproom@localhost:5434/maproom_test pnpm test:integration

# Step 4: Verify isolation
DEV_COUNT=$(docker exec maproom-postgres psql -U maproom -d maproom -t -c "SELECT COUNT(*) FROM maproom.chunks")
TEST_COUNT=$(docker exec maproom-postgres-test psql -U maproom -d maproom_test -t -c "SELECT COUNT(*) FROM maproom.chunks")

echo "Dev database chunks: $DEV_COUNT"
echo "Test database chunks: $TEST_COUNT"

# Step 5: Validate different
if [ "$DEV_COUNT" != "$TEST_COUNT" ]; then
  echo "✅ Databases are isolated"
else
  echo "⚠️  Databases may not be isolated (counts match)"
fi
```

**Success Criteria**: Script exits 0, logs show isolation

### Milestone 2: Configuration Verification (OPTIONAL for MVP)
**Deliverable**: Smoke test suite

**Note**: This milestone is OPTIONAL for MVP. Milestone 1 (manual validation) provides sufficient confidence. The existing test suite implicitly validates configuration correctness - if tests pass, configuration is working.

```typescript
// tests/smoke/database-isolation.test.ts
import { describe, it, expect } from 'vitest'
import { getDatabaseUrl } from '../helpers/database'

describe('Database Isolation Configuration', () => {
  it('uses TEST_MAPROOM_DATABASE_URL when set', () => {
    const originalEnv = process.env.TEST_MAPROOM_DATABASE_URL
    process.env.TEST_MAPROOM_DATABASE_URL = 'postgresql://test-override:5434/test'

    const url = getDatabaseUrl()

    expect(url).toContain('5434')
    expect(url).toContain('test')

    process.env.TEST_MAPROOM_DATABASE_URL = originalEnv
  })

  it('falls back to MAPROOM_DATABASE_URL', () => {
    const originalTest = process.env.TEST_MAPROOM_DATABASE_URL
    const originalMaproom = process.env.MAPROOM_DATABASE_URL

    delete process.env.TEST_MAPROOM_DATABASE_URL
    process.env.MAPROOM_DATABASE_URL = 'postgresql://maproom:5433/maproom'

    const url = getDatabaseUrl()

    expect(url).toContain('5433')
    expect(url).toContain('maproom')

    process.env.TEST_MAPROOM_DATABASE_URL = originalTest
    process.env.MAPROOM_DATABASE_URL = originalMaproom
  })
})
```

**Success Criteria**: Smoke tests pass

### Milestone 3: CI Integration
**Deliverable**: GitHub Actions workflow test job

```yaml
test:
  runs-on: ubuntu-latest
  services:
    postgres-test:
      image: pgvector/pgvector:pg16
      env:
        POSTGRES_DB: maproom_test
        POSTGRES_USER: maproom
        POSTGRES_PASSWORD: maproom
      ports:
        - 5434:5432
      options: >-
        --health-cmd pg_isready
        --health-interval 10s
        --health-timeout 5s
        --health-retries 5
  env:
    TEST_MAPROOM_DATABASE_URL: postgresql://maproom:maproom@localhost:5434/maproom_test
  steps:
    - uses: actions/checkout@v4
    - run: pnpm test
```

**Success Criteria**: CI tests pass using TEST_MAPROOM_DATABASE_URL

## Integration Testing Strategy

### What Integration Tests Validate

**Existing Integration Tests** (no changes needed):
- Search functionality
- Indexing workflow
- MCP tool operations
- Migration execution

**Why**: These tests will automatically use TEST_MAPROOM_DATABASE_URL once configured. They become implicit validators of database isolation.

### What We Add

**Database Fixture Management**:
```typescript
// tests/helpers/test-fixtures.ts
export async function resetTestDatabase(client: Client): Promise<void> {
  // Drop and recreate maproom_test database
  await client.query('DROP DATABASE IF EXISTS maproom_test')
  await client.query('CREATE DATABASE maproom_test')
  // Run migrations
}
```

**Purpose**: Clean slate for each test run without affecting dev database

## Risk Mitigation Through Testing

### Risk: Breaking Existing Tests
**Mitigation**: Fallback to MAPROOM_DATABASE_URL preserves current behavior

**Validation**:
```bash
# Tests still pass without TEST_MAPROOM_DATABASE_URL set
unset TEST_MAPROOM_DATABASE_URL
pnpm test
```

### Risk: CI Flakiness
**Mitigation**: Service health checks in GitHub Actions

**Validation**: Workflow logs show health check passing before tests run

### Risk: Port Conflicts
**Mitigation**: Clear error messages if ports unavailable

**Validation**:
```bash
# Start something on 5434
nc -l 5434 &
PID=$!

# Try to start compose - should fail with clear message
docker compose up

# Cleanup
kill $PID
```

## Quality Gates

### Before Merge
1. ✅ Manual validation script passes
2. ✅ Smoke tests pass
3. ✅ Existing test suite passes with TEST_MAPROOM_DATABASE_URL
4. ✅ Existing test suite passes without TEST_MAPROOM_DATABASE_URL (backward compat)
5. ✅ Documentation updated

### Before Production Use
1. ✅ CI workflow integrated and passing
2. ✅ Developer guide published
3. ✅ Troubleshooting guide available

## Long-term Quality Maintenance

### Regression Prevention
- Existing test suite becomes regression suite for isolation
- If isolation breaks, tests fail (self-validating)
- No special "isolation tests" needed beyond smoke tests

### Monitoring
- CI should track test database connection count
- Alert if tests connect to dev database (5433) instead of test (5434)

### Evolution
- If adding more test databases (e.g., E2E), follow same TEST_*_DATABASE_URL pattern
- Smoke tests extend to cover new databases

## Definition of Done

**Test Isolation Delivered When**:
1. ✅ Two postgres containers running (postgres, postgres-test)
2. ✅ Tests connect to port 5434, dev to port 5433
3. ✅ Test data not visible in dev database
4. ✅ Dev data not visible in test database
5. ✅ `docker compose up && pnpm test` works out of the box
6. ✅ CI tests pass using test database
7. ✅ Documentation explains how to use both databases

**Not Required for Done**:
- ❌ 100% code coverage of infrastructure (over-testing)
- ❌ Unit tests for Docker Compose syntax (trust tooling)
- ❌ Performance benchmarks (not a performance feature)
- ❌ Chaos testing (YAGNI for development databases)
