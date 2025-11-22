# Implementation Plan: Test Database Isolation

## Project Goal

Set up isolated test database infrastructure that runs alongside development database without interference, enabling true test isolation while maintaining developer ergonomics.

## Success Criteria

- ✅ Two PostgreSQL containers running simultaneously (dev: 5433, test: 5434)
- ✅ Tests use TEST_MAPROOM_DATABASE_URL, dev uses MAPROOM_DATABASE_URL
- ✅ Test data isolated from dev data (separate volumes)
- ✅ `docker compose up && pnpm test` works out of the box
- ✅ CI tests pass using isolated test database
- ✅ Backward compatible (existing tests work without TEST_MAPROOM_DATABASE_URL)

## Implementation Phases

### Phase 1: Docker Infrastructure

**Goal**: Add postgres-test service to Docker Compose

**Files to Modify**:
- `packages/maproom-mcp/config/docker-compose.yml`

**Changes**:
```yaml
# Add new service (similar to existing postgres)
postgres-test:
  image: pgvector/pgvector:pg16
  container_name: maproom-postgres-test
  environment:
    POSTGRES_DB: maproom_test
    POSTGRES_USER: maproom
    POSTGRES_PASSWORD: maproom
  ports:
    - "5434:5432"  # Different host port
  volumes:
    - maproom-test-data:/var/lib/postgresql/data  # Separate volume
    # NOTE: init.sql mount is disabled (matches current dev setup)
    # Schema will be initialized manually after container starts
    # - ./init.sql:/docker-entrypoint-initdb.d/init.sql
  healthcheck:
    test: ["CMD-SHELL", "pg_isready -U maproom -d maproom_test"]
    interval: 10s
    timeout: 5s
    retries: 5
  restart: unless-stopped
  networks:
    - maproom-network

# Add new volume
volumes:
  maproom-test-data:
```

**Schema Initialization** (manual step after container starts):
```bash
# Option A: Execute init.sql manually
docker exec maproom-postgres-test psql -U maproom -d maproom_test < packages/maproom-mcp/config/init.sql

# Option B: Run existing migration system (if available)
# [Document migration command based on project's migration approach]
```

**Note**: init.sql mount is disabled to match current dev database setup. Current docker-compose.yml has this note:
```
# Note: init.sql mount disabled in dev container due to Docker-in-Docker limitations
# Schema will be initialized via migrations or manual SQL execution
```

**Validation**:
```bash
docker compose up -d
docker compose ps | grep postgres-test  # Should show healthy
docker exec maproom-postgres-test psql -U maproom -d maproom_test -c "\l"  # Should show maproom_test database
```

**Acceptance Criteria**:
- [ ] postgres-test service starts successfully
- [ ] postgres-test shows as healthy in `docker compose ps`
- [ ] maproom_test database created with correct schema (via manual initialization)
- [ ] Test database accessible on localhost:5434
- [ ] Separate volume created (maproom-test-data)
- [ ] Schema initialization procedure documented (manual SQL execution or migration)

---

### Phase 2: Test Configuration

**Goal**: Update test configurations to use TEST_MAPROOM_DATABASE_URL

**Files to Modify**:
- `packages/maproom-mcp/vitest.config.ts`
- `packages/maproom-mcp/package.json`

**Changes**:

**vitest.config.ts**:
```typescript
export default defineConfig({
  test: {
    // ... other config
    env: {
      // Use TEST_MAPROOM_DATABASE_URL if set, fall back to MAPROOM_DATABASE_URL
      // NOTE: Uses container hostname (maproom-postgres-test) because tests run
      // on host but connect through Docker network
      MAPROOM_DATABASE_URL:
        process.env.TEST_MAPROOM_DATABASE_URL ||
        'postgresql://maproom:maproom@maproom-postgres-test:5432/maproom_test'
    }
  }
})
```

**Hostname Explanation**:
- Tests execute on **host machine** (via `pnpm test`)
- Tests connect to databases via **Docker network** (host can access Docker network)
- Use **container hostname** `maproom-postgres-test:5432` (visible via Docker network)
- Do NOT use `localhost:5434` in vitest.config.ts (that's for package.json scripts)

**package.json** (update test scripts):
```json
{
  "scripts": {
    "test:vitest": "TEST_MAPROOM_DATABASE_URL=postgresql://maproom:maproom@localhost:5434/maproom_test vitest run",
    "test:integration": "TEST_MAPROOM_DATABASE_URL=postgresql://maproom:maproom@localhost:5434/maproom_test vitest run tests/integration/"
  }
}
```

**Note**: `tests/helpers/database.ts` already supports this pattern - no changes needed!

**Validation**:
```bash
# Test with TEST_MAPROOM_DATABASE_URL (should use test DB)
TEST_MAPROOM_DATABASE_URL=postgresql://maproom:maproom@localhost:5434/maproom_test pnpm test:vitest

# Test without TEST_MAPROOM_DATABASE_URL (should fall back to dev DB)
unset TEST_MAPROOM_DATABASE_URL
pnpm test:vitest
```

**Acceptance Criteria**:
- [ ] vitest.config.ts uses TEST_MAPROOM_DATABASE_URL when set
- [ ] vitest.config.ts falls back to MAPROOM_DATABASE_URL
- [ ] vitest.config.ts uses container hostname (maproom-postgres-test:5432)
- [ ] package.json test scripts set TEST_MAPROOM_DATABASE_URL with localhost:5434
- [ ] Tests pass with TEST_MAPROOM_DATABASE_URL (manual validation via running test suite)
- [ ] Tests pass without TEST_MAPROOM_DATABASE_URL (backward compatibility)

---

### Phase 3: Manual Validation

**Goal**: Verify test and dev databases are truly isolated

**Deliverable**: Manual validation script

**Script**: `/workspace/scripts/validate-test-isolation.sh` (project root)
```bash
#!/bin/bash
set -e

echo "Validating test database isolation..."

# Start infrastructure
echo "Starting Docker Compose..."
docker compose up -d

# Wait for health
echo "Waiting for databases to be healthy..."
timeout 30 bash -c 'until docker compose ps | grep postgres-test | grep healthy; do sleep 1; done'

# Run tests
echo "Running tests with TEST_MAPROOM_DATABASE_URL..."
TEST_MAPROOM_DATABASE_URL=postgresql://maproom:maproom@localhost:5434/maproom_test pnpm test:integration

# Verify isolation
echo "Checking database isolation..."
DEV_COUNT=$(docker exec maproom-postgres psql -U maproom -d maproom -t -c "SELECT COUNT(*) FROM maproom.chunks" | xargs)
TEST_COUNT=$(docker exec maproom-postgres-test psql -U maproom -d maproom_test -t -c "SELECT COUNT(*) FROM maproom.chunks" | xargs)

echo "Dev database chunks: $DEV_COUNT"
echo "Test database chunks: $TEST_COUNT"

# Validate
if [ "$DEV_COUNT" != "$TEST_COUNT" ]; then
  echo "✅ Databases are isolated"
else
  echo "⚠️  Databases may not be isolated (counts match)"
  echo "This could be normal if both are empty or have same test data"
fi

echo "Validation complete"
```

**Acceptance Criteria**:
- [ ] Script exits successfully
- [ ] Both databases show as healthy
- [ ] Tests run successfully using test database
- [ ] Database counts can be checked independently

---

### Phase 4: CI/CD Integration

**Goal**: Create GitHub Actions workflow with test database

**Files to Create**:
- `.github/workflows/test.yml` (new file - does not exist yet)

**Changes**:
```yaml
name: Tests
on:
  push:
    branches: [main]
  pull_request:

jobs:
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

      - name: Setup Node.js
        uses: actions/setup-node@v4
        with:
          node-version: '20'

      - name: Install pnpm
        uses: pnpm/action-setup@v2
        with:
          version: 8

      - name: Install dependencies
        run: pnpm install

      - name: Run tests
        run: pnpm test
```

**Acceptance Criteria**:
- [ ] CI workflow uses postgres-test service
- [ ] TEST_MAPROOM_DATABASE_URL set in CI environment
- [ ] Tests pass in CI using test database
- [ ] Workflow logs show correct database connection

---

### Phase 5: Documentation

**Goal**: Document new test database setup for developers

**Files to Create/Modify**:
- `packages/maproom-mcp/README.md` (update)
- `docs/development/TEST_DATABASE_SETUP.md` (new)

**README.md Updates**:
```markdown
## Database Setup

### Development Database
- Port: 5433
- Database: maproom
- Connection: `postgresql://maproom:maproom@localhost:5433/maproom`

### Test Database
- Port: 5434
- Database: maproom_test
- Connection: `postgresql://maproom:maproom@localhost:5434/maproom_test`

### Starting Databases
```bash
# Start both dev and test databases
docker compose up -d

# Check health
docker compose ps
```

### Running Tests
```bash
# Tests automatically use test database
pnpm test

# Override database for tests
TEST_MAPROOM_DATABASE_URL=postgresql://custom:url pnpm test
```

**TEST_DATABASE_SETUP.md Content**:
- Why we have separate databases
- How the configuration works (TEST_MAPROOM_DATABASE_URL priority)
- Troubleshooting common issues
- How to reset test database
- How to run tests against dev database (if needed)
- Volume management commands:
  ```bash
  # Reset test database completely
  docker compose down
  docker volume rm maproom-test-data
  docker compose up -d
  # Then run manual schema initialization
  ```

**Acceptance Criteria**:
- [ ] README updated with database information
- [ ] Test setup guide created
- [ ] Troubleshooting section included
- [ ] Examples provided for common workflows

---

## Ticket Breakdown

### TESTISO-1001: Add postgres-test service to Docker Compose
**Phase**: 1 - Docker Infrastructure
**Files**: `packages/maproom-mcp/config/docker-compose.yml`
**Description**: Add postgres-test service with separate port (5434), volume, and database name
**Suggested Agent**: docker-engineer
**Testing**: Manual validation that service starts and is healthy

### TESTISO-1002: Update vitest configuration for TEST_MAPROOM_DATABASE_URL
**Phase**: 2 - Test Configuration
**Files**: `packages/maproom-mcp/vitest.config.ts`
**Description**: Update env config to use TEST_MAPROOM_DATABASE_URL with fallback to MAPROOM_DATABASE_URL
**Suggested Agent**: General implementation
**Testing**: Manual validation via running existing test suite

### TESTISO-1003: Update package.json test scripts
**Phase**: 2 - Test Configuration
**Files**: `packages/maproom-mcp/package.json`
**Description**: Update test scripts to set TEST_MAPROOM_DATABASE_URL pointing to localhost:5434
**Suggested Agent**: General implementation
**Testing**: Run tests with updated scripts

### TESTISO-1004: Create manual validation script
**Phase**: 3 - Manual Validation
**Files**: `/workspace/scripts/validate-test-isolation.sh` (new)
**Description**: Create bash script to validate database isolation
**Suggested Agent**: General implementation
**Testing**: Execute script and verify output

### TESTISO-1005: Create GitHub Actions test workflow
**Phase**: 4 - CI/CD Integration
**Files**: `.github/workflows/test.yml` (new)
**Description**: Create new GitHub Actions workflow with postgres-test service and TEST_MAPROOM_DATABASE_URL
**Suggested Agent**: github-actions-specialist
**Testing**: Verify CI tests pass with new configuration

### TESTISO-1006: Update documentation
**Phase**: 5 - Documentation
**Files**: `packages/maproom-mcp/README.md`, `docs/development/TEST_DATABASE_SETUP.md`
**Description**: Document test database setup and usage
**Suggested Agent**: General implementation
**Testing**: Manual review of documentation clarity

---

## Dependencies

```
TESTISO-1001 (Docker)
    ↓
TESTISO-1002 (vitest)
    ↓
TESTISO-1003 (package.json)
    ↓
TESTISO-1004 (validation)
    ↓
TESTISO-1005 (CI)
    ↓
TESTISO-1006 (docs)
```

**Explanation**:
- Must have postgres-test running before updating configurations
- Must have configurations updated before validation
- Should validate manually before updating CI
- Documentation should reflect final working state

---

## Rollback Plan

If database isolation causes issues:

1. **Immediate Rollback**: Unset TEST_MAPROOM_DATABASE_URL in scripts
   ```json
   "test:vitest": "vitest run"  // Falls back to MAPROOM_DATABASE_URL
   ```

2. **Remove postgres-test**: Comment out service in docker-compose.yml
   ```yaml
   # postgres-test:  # Disabled - using dev database for tests
   ```

3. **Revert CI**: Remove postgres-test service from workflow

The architecture is designed for easy rollback - all changes are additive and backward compatible.

---

## Timeline Estimate

**Phase 1**: 1 ticket, ~30 minutes
**Phase 2**: 2 tickets, ~1 hour
**Phase 3**: 1 ticket, ~30 minutes
**Phase 4**: 1 ticket, ~45 minutes
**Phase 5**: 1 ticket, ~1 hour

**Total**: 6 tickets, ~3.75 hours

**Note**: This is infrastructure work - actual time may vary based on:
- Docker environment issues
- Existing data in databases
- CI/CD configuration complexity

---

## Risk Mitigation

**Risk**: Port 5434 already in use
- **Mitigation**: Check with `lsof -i :5434` before starting
- **Recovery**: Use different port, update configs

**Risk**: Schema drift between databases
- **Mitigation**: Both use same init.sql
- **Recovery**: Recreate test volume

**Risk**: Tests fail with test database but pass with dev
- **Mitigation**: Manual validation phase catches this
- **Recovery**: Investigate schema/data differences

**Risk**: CI flakiness with service containers
- **Mitigation**: Health checks ensure readiness
- **Recovery**: Increase timeout values

---

## Post-Implementation Validation

After all tickets complete:

1. ✅ Run full test suite: `pnpm test`
2. ✅ Verify dev database unchanged: Check row counts
3. ✅ Run tests in parallel with dev workflow: `pnpm dev` + `pnpm test`
4. ✅ Check CI passes: Trigger GitHub Actions workflow
5. ✅ Validate backward compatibility: `unset TEST_MAPROOM_DATABASE_URL && pnpm test`
6. ✅ Verify isolation: Run validation script

---

## Long-Term Maintenance

**Monthly**:
- Review test database size (clean up if needed)
- Check Docker image updates (pgvector/pgvector)

**Per Test Run**:
- Tests should clean up after themselves
- Consider beforeEach/afterEach hooks for test data

**Documentation**:
- Keep TEST_DATABASE_SETUP.md updated with new patterns
- Add troubleshooting entries as issues arise

---

## Future Enhancements (Not in Scope)

- Multiple test databases for parallel test execution
- Database seeding for E2E tests
- Test database reset between test suites
- Performance testing database with different config
- Testcontainers for per-test isolation

These can be added later if needed, following the same TEST_*_DATABASE_URL pattern.
