# Test Database Setup Guide

**Last Updated**: 2025-11-20

Comprehensive guide to Maproom's dual-database architecture for development and testing.

## Table of Contents

- [Overview](#overview)
- [Why Separate Databases?](#why-separate-databases)
- [Configuration](#configuration)
- [Common Workflows](#common-workflows)
- [Troubleshooting](#troubleshooting)
- [Volume Management](#volume-management)
- [CI/CD Configuration](#cicd-configuration)
- [Architecture Details](#architecture-details)
- [Validation Script](#validation-script)

---

## Overview

Maproom uses **two separate PostgreSQL instances** to isolate development and test data:

| Database | Port | Container Name | Database Name | Purpose | Startup |
|----------|------|----------------|---------------|---------|---------|
| **Development** | 5433 | `maproom-postgres` | `maproom` | Manual work, CLI commands, MCP operations | **Automatic** (via setup) |
| **Test** | 5434 | `maproom-postgres-test` | `maproom_test` | Automated tests only (vitest, integration tests) | **Manual** (opt-in) |

**Key difference**: The development database starts automatically when you run `setup` (via `depends_on` in docker-compose.yml). The test database is **opt-in** and must be started manually - regular maproom users don't need it running.

Both databases share these characteristics:
- Run the same `pgvector/pgvector:pg16` image
- Use identical configuration (shared_buffers, connections, etc.)
- Have separate Docker volumes for complete data isolation
- Initialize from the same schema (`config/init.sql`)

---

## Why Separate Databases?

### The Problem

Before test database isolation, development and tests shared a single database instance, causing:

1. **Data Contamination** - Test data polluted dev database, dev data polluted test results
2. **State Leakage** - Failed tests left dirty data affecting subsequent test runs
3. **Cannot Develop and Test Simultaneously** - Running tests while developing would corrupt both environments
4. **Lack of Production Parity** - Tests didn't run in isolated environment like production

### The Solution

Dual-database architecture provides:

- **Complete Isolation** - Test and dev data never mix
- **Parallel Workflows** - Develop in one terminal, test in another
- **Repeatable Tests** - Test database can be reset without affecting dev work
- **Production Parity** - Tests run in isolated environment matching production

---

## Configuration

### Environment Variables

The test database connection is controlled by the `TEST_MAPROOM_DATABASE_URL` environment variable:

**Priority**:
1. `TEST_MAPROOM_DATABASE_URL` (explicit override) - Use this value if set
2. `vitest.config.ts` default - Falls back to `maproom-postgres-test:5432`

**When to Set**:
- **In devcontainer**: ALWAYS set to `host.docker.internal:5434` (see [devcontainer adaptation](#devcontainer-adaptation))
- **In CI**: Usually set to `localhost:5434` (GitHub Actions service containers)
- **On host**: Optional, defaults to container hostname

### Hostname Resolution

Connection strings vary depending on **where the code runs**:

| Context | Hostname | Port | Example Connection String |
|---------|----------|------|---------------------------|
| **vitest.config.ts** | `maproom-postgres-test` | 5432 | `postgresql://maproom:maproom@maproom-postgres-test:5432/maproom_test` |
| **package.json scripts** | `host.docker.internal` | 5434 | `postgresql://maproom:maproom@host.docker.internal:5434/maproom_test` |
| **GitHub Actions** | `localhost` | 5434 | `postgresql://maproom:maproom@localhost:5434/maproom_test` |
| **Direct host connection** | `localhost` | 5434 | `postgresql://maproom:maproom@localhost:5434/maproom_test` |

### Devcontainer Adaptation

**CRITICAL**: When running tests inside the devcontainer, you CANNOT use `localhost:5434` because:
- The devcontainer is itself a Docker container
- `localhost` inside the devcontainer refers to the devcontainer, not the host
- The test database runs on the host's Docker daemon

**Solution**: Use `host.docker.internal:5434` to reach the host from inside the devcontainer:

```bash
# In package.json (devcontainer-compatible)
TEST_MAPROOM_DATABASE_URL=postgresql://maproom:maproom@host.docker.internal:5434/maproom_test
```

This was discovered and fixed in **TESTISO-1003**.

---

## Common Workflows

### Starting Test Database

The test database is **opt-in** and must be started manually before running tests. Regular maproom users don't need this step.

**Start test database** (developers/CI only):

```bash
cd packages/maproom-mcp/config  # or ~/.maproom-mcp
docker compose up -d postgres-test

# Wait for healthy status
docker compose ps | grep postgres-test
```

**Initialize schema** (first time only):

```bash
# From repository root
docker exec -i maproom-postgres-test psql -U maproom -d maproom_test < packages/maproom-mcp/config/init.sql

# Or from config directory
docker exec -i maproom-postgres-test psql -U maproom -d maproom_test < init.sql
```

**Verify connection**:

```bash
docker exec maproom-postgres-test psql -U maproom -d maproom_test -c "\dt maproom.*"
```

### Running Tests

**Prerequisites**: Test database must be running (see [Starting Test Database](#starting-test-database) above).

**Basic test run** (uses test database automatically):

```bash
cd packages/maproom-mcp
pnpm test
```

**Integration tests only**:

```bash
pnpm test:integration
```

**All tests** (connection + blob-sha + vitest):

```bash
pnpm test:all
```

The `package.json` scripts automatically set `TEST_MAPROOM_DATABASE_URL` for devcontainer compatibility.

### Resetting Test Database

**Complete reset** (deletes all data and volume):

```bash
# Stop test database
cd packages/maproom-mcp/config  # or ~/.maproom-mcp
docker compose stop postgres-test

# Remove volume
docker volume rm config_maproom-test-data

# Restart and reinitialize
docker compose up -d postgres-test

# Wait for healthy status
docker compose ps | grep postgres-test

# Reinitialize schema
docker exec -i maproom-postgres-test psql -U maproom -d maproom_test < config/init.sql
```

**Quick reset** (keeps volume, truncates tables):

```bash
docker exec maproom-postgres-test psql -U maproom -d maproom_test -c "TRUNCATE TABLE maproom.chunks, maproom.files, maproom.code_embeddings CASCADE"
```

### Running Tests Against Dev Database

Sometimes you want to test against the dev database (e.g., debugging with real data):

```bash
# Override to use dev database
TEST_MAPROOM_DATABASE_URL=postgresql://maproom:maproom@localhost:5433/maproom pnpm test:integration
```

**Warning**: This will modify your dev database. Consider backing up first.

---

## Troubleshooting

### Connection Refused (devcontainer localhost issues)

**Error**:
```
Error: connect ECONNREFUSED 127.0.0.1:5434
```

**Diagnosis**: You're running tests inside the devcontainer and using `localhost:5434`.

**Solution**: Use `host.docker.internal:5434` instead:

```bash
TEST_MAPROOM_DATABASE_URL=postgresql://maproom:maproom@host.docker.internal:5434/maproom_test pnpm test
```

**Why**: Inside the devcontainer, `localhost` refers to the container itself, not the host where the test database runs.

### Relation Does Not Exist

**Error**:
```
ERROR: relation "maproom.chunks" does not exist
```

**Diagnosis**: Test database schema not initialized.

**Solution**: Initialize schema manually:

```bash
docker exec -i maproom-postgres-test psql -U maproom -d maproom_test < packages/maproom-mcp/config/init.sql
```

**Verification**:
```bash
docker exec maproom-postgres-test psql -U maproom -d maproom_test -c "\dt maproom.*"
```

Expected output: List of 12 tables in `maproom` schema.

### Port Conflicts

**Error**:
```
Error: bind: address already in use (0.0.0.0:5434)
```

**Diagnosis**: Another process is using port 5434.

**Solution**: Find and stop the conflicting process:

```bash
# Find process using port 5434
lsof -i :5434

# Stop the process or choose different port
# To use different port, edit docker-compose.yml:
# ports:
#   - "5435:5432"  # Changed from 5434
```

### CI Failures

**Error**: Tests pass locally but fail in CI.

**Common Causes**:
1. **Missing `TEST_MAPROOM_DATABASE_URL`** - CI doesn't set it correctly
2. **Service container not healthy** - Test database not ready
3. **Wrong hostname** - Using `host.docker.internal` instead of `localhost` in CI

**Solution**: Check GitHub Actions configuration (see [CI/CD Configuration](#cicd-configuration)).

### Test Database Stale/Corrupted

**Symptoms**:
- Tests fail with data already exists errors
- Unexpected foreign key violations
- Stale data from previous test runs

**Solution**: Full reset with volume removal:

```bash
cd packages/maproom-mcp/config
docker compose down postgres-test
docker volume rm config_maproom-test-data
docker compose up -d postgres-test
docker exec -i maproom-postgres-test psql -U maproom -d maproom_test < config/init.sql
```

---

## Volume Management

### List Volumes

```bash
docker volume ls | grep maproom
```

Expected output:
```
config_maproom-data          # Dev database volume
config_maproom-test-data     # Test database volume
```

### Inspect Volume

```bash
# Test database volume
docker volume inspect config_maproom-test-data

# Shows mount point (usually /var/lib/docker/volumes/config_maproom-test-data/_data)
```

### Remove Volume

**Warning**: This deletes ALL data in the test database permanently.

```bash
# Stop container first
docker compose -f packages/maproom-mcp/config/docker-compose.yml stop postgres-test

# Remove volume
docker volume rm config_maproom-test-data

# Restart and reinitialize
docker compose -f packages/maproom-mcp/config/docker-compose.yml up -d postgres-test
docker exec -i maproom-postgres-test psql -U maproom -d maproom_test < packages/maproom-mcp/config/init.sql
```

### Volume Size

Check how much disk space volumes are using:

```bash
# Test database volume size
docker system df -v | grep maproom-test-data

# All maproom volumes
docker system df -v | grep maproom
```

---

## CI/CD Configuration

### GitHub Actions

The test database runs as a **service container** in GitHub Actions workflows:

**Example** (from `.github/workflows/test.yml`):

```yaml
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
          --health-cmd "pg_isready -U maproom -d maproom_test"
          --health-interval 10s
          --health-timeout 5s
          --health-retries 5

    steps:
      - name: Initialize test database schema
        run: |
          docker exec $(docker ps -q -f "ancestor=pgvector/pgvector:pg16") \
            psql -U maproom -d maproom_test < packages/maproom-mcp/config/init.sql

      - name: Run tests
        env:
          TEST_MAPROOM_DATABASE_URL: postgresql://maproom:maproom@localhost:5434/maproom_test
        run: pnpm test
```

**Key Points**:
- Use `localhost:5434` in CI (not `host.docker.internal`)
- Service containers run on the same Docker network as the job container
- Wait for health check before running tests
- Initialize schema manually in a setup step

### Local CI Simulation

Test the exact CI environment locally:

```bash
# Start test database matching CI config
docker run -d \
  --name ci-postgres-test \
  -e POSTGRES_DB=maproom_test \
  -e POSTGRES_USER=maproom \
  -e POSTGRES_PASSWORD=maproom \
  -p 5434:5432 \
  pgvector/pgvector:pg16

# Wait for startup
sleep 5

# Initialize schema
docker exec ci-postgres-test psql -U maproom -d maproom_test < packages/maproom-mcp/config/init.sql

# Run tests
TEST_MAPROOM_DATABASE_URL=postgresql://maproom:maproom@localhost:5434/maproom_test pnpm test

# Cleanup
docker stop ci-postgres-test
docker rm ci-postgres-test
```

---

## Architecture Details

### Design Documentation

Complete architecture and design rationale in project planning documents:

- **Analysis**: `/workspace/.agents/projects/TESTISO_test-database-isolation/planning/analysis.md`
- **Architecture**: `/workspace/.agents/projects/TESTISO_test-database-isolation/planning/architecture.md`
- **Implementation Plan**: `/workspace/.agents/projects/TESTISO_test-database-isolation/planning/plan.md`
- **Quality Strategy**: `/workspace/.agents/projects/TESTISO_test-database-isolation/planning/quality-strategy.md`

### Implementation Tickets

The dual-database system was implemented in phases:

- **TESTISO-1001**: Add `postgres-test` service to docker-compose.yml
- **TESTISO-1002**: Update vitest config for test database
- **TESTISO-1003**: Update package.json scripts (devcontainer adaptation)
- **TESTISO-1004**: Create validation script
- **TESTISO-1005**: Update CI configuration
- **TESTISO-1006**: Update documentation (this guide)

### Key Design Decisions

1. **Manual schema initialization** - Matches dev database pattern, avoids Docker-in-Docker complexity
2. **Separate volumes** - Guarantees data isolation, prevents accidental cross-contamination
3. **Identical configuration** - Both databases use same PostgreSQL settings for production parity
4. **Sequential test execution** - `vitest.config.ts` uses single thread to avoid race conditions
5. **Environment variable override** - `TEST_MAPROOM_DATABASE_URL` allows flexible connection strings

---

## Validation Script

### Overview

The project includes a comprehensive validation script to verify test database isolation:

**Location**: `/workspace/scripts/validate-test-isolation.sh`

**What it checks**:
1. Docker Compose infrastructure is running
2. Both databases are healthy
3. Integration tests can connect to test database
4. Test and dev databases have different data (proving isolation)

### Running Validation

```bash
# From anywhere in the repository
./scripts/validate-test-isolation.sh
```

**Expected output** (when isolated):
```
=========================================
Test Database Isolation Validation
=========================================

Step 1: Ensuring Docker Compose infrastructure is running...
✅ Docker Compose services already running

Step 2: Waiting for databases to be healthy (timeout: 30s)...
✅ Dev database (maproom-postgres) is healthy
✅ Test database (maproom-postgres-test) is healthy

Step 3: Running integration tests against test database...
✅ Integration tests passed

Step 4: Querying databases for chunk counts...
Dev database (maproom):       142 chunks
Test database (maproom_test): 0 chunks

Step 5: Validating database isolation...
✅ Databases are ISOLATED (different chunk counts)
   Dev and test databases contain different data, confirming isolation.
```

### Interpreting Results

**Pass scenarios**:
- Different chunk counts (databases have different data)
- Both zero (both empty, but separate instances)

**Fail scenarios**:
- Same non-zero chunk count (possible volume sharing or coincidence)
- Connection errors (infrastructure not running)
- Test failures (configuration issue)

### Troubleshooting Validation Failures

If validation fails:

1. **Check volumes are separate**:
   ```bash
   docker volume ls | grep maproom
   # Should show both config_maproom-data AND config_maproom-test-data
   ```

2. **Verify containers are running**:
   ```bash
   docker compose -f packages/maproom-mcp/config/docker-compose.yml ps
   ```

3. **Check container logs**:
   ```bash
   docker logs maproom-postgres-test
   ```

4. **Reset test database** (see [Resetting Test Database](#resetting-test-database))

---

## Additional Resources

- **Main README**: `/workspace/packages/maproom-mcp/README.md`
- **Database Architecture**: `/workspace/docs/architecture/DATABASE_ARCHITECTURE.md`
- **Docker Compose Config**: `/workspace/packages/maproom-mcp/config/docker-compose.yml`
- **Schema Definition**: `/workspace/packages/maproom-mcp/config/init.sql`
- **Vitest Config**: `/workspace/packages/maproom-mcp/vitest.config.ts`
- **Package Scripts**: `/workspace/packages/maproom-mcp/package.json`

---

## Quick Reference

### Connection Strings

```bash
# Development Database
postgresql://maproom:maproom@localhost:5433/maproom

# Test Database (host)
postgresql://maproom:maproom@localhost:5434/maproom_test

# Test Database (devcontainer)
postgresql://maproom:maproom@host.docker.internal:5434/maproom_test

# Test Database (container-to-container)
postgresql://maproom:maproom@maproom-postgres-test:5432/maproom_test
```

### Common Commands

```bash
# Start both databases
cd packages/maproom-mcp/config
docker compose up -d

# Check database status
docker compose ps

# Initialize test schema
docker exec -i maproom-postgres-test psql -U maproom -d maproom_test < config/init.sql

# Run tests
cd packages/maproom-mcp
pnpm test

# Reset test database
docker compose stop postgres-test
docker volume rm config_maproom-test-data
docker compose up -d postgres-test
docker exec -i maproom-postgres-test psql -U maproom -d maproom_test < config/init.sql

# Validate isolation
./scripts/validate-test-isolation.sh
```

### Port Allocation

- **5433**: Development database (maproom-postgres)
- **5434**: Test database (maproom-postgres-test)
- **5432**: Internal PostgreSQL port (both containers)

### Volume Names

- `config_maproom-data`: Development database volume
- `config_maproom-test-data`: Test database volume

---

**Questions or issues?** File a bug report with the `test-database` label in the CrewChief repository.
