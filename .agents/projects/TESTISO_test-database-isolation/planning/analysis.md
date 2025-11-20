# Analysis: Test Database Isolation

## Problem Definition

Currently, development and test environments share the same PostgreSQL database instance. This creates several issues:

1. **Data Contamination**: Tests may see or modify data from development workflows
2. **State Leakage**: Failed tests can leave artifacts that affect subsequent test runs
3. **Concurrent Execution**: Cannot develop and run tests simultaneously without interference
4. **Production Parity**: Production deployments use isolated databases, but tests don't reflect this

The user explicitly wants:
- **Primary environment**: Set up like a normal user (published Docker image, standard setup)
- **Test environment**: Isolated database on different port for running tests

## Current State

### Docker Compose Configuration

**`packages/maproom-mcp/config/docker-compose.yml`:**
- Single `postgres` service on port `5433` (mapped from internal `5432`)
- Database: `maproom`
- Container: `maproom-postgres`
- Volume: `maproom-data`
- MCP service pulls published image from Docker Hub

**`packages/maproom-mcp/config/docker-compose.test.yml`:**
- **Misleading name**: This file is for *building from source*, NOT for test database isolation
- Overrides `maproom-mcp` service to build locally instead of pulling from Docker Hub
- Does NOT provide test database isolation

### Test Configuration

**`packages/maproom-mcp/vitest.config.ts`:**
```typescript
env: {
  MAPROOM_DATABASE_URL: 'postgresql://maproom:maproom@maproom-postgres:5432/maproom'
}
```
- Hardcoded to development database
- No way to override for test isolation

**`packages/maproom-mcp/package.json`:**
```json
"test:vitest": "MAPROOM_DATABASE_URL=postgresql://maproom:maproom@maproom-postgres:5432/maproom vitest run",
"test:integration": "MAPROOM_DATABASE_URL=postgresql://maproom:maproom@maproom-postgres:5432/maproom vitest run tests/integration/"
```
- All test scripts point to development database
- No test-specific database URL

**`packages/maproom-mcp/tests/helpers/database.ts`:**
```typescript
export function getDatabaseUrl(): string {
  const dbUrl = process.env.TEST_MAPROOM_DATABASE_URL || process.env.MAPROOM_DATABASE_URL
  // ...
}
```
- **Already supports TEST_MAPROOM_DATABASE_URL** - excellent!
- Falls back to MAPROOM_DATABASE_URL if TEST_MAPROOM_DATABASE_URL not set
- This file needs no changes

### Gaps Identified

1. **No test-specific PostgreSQL service**: Only one postgres instance
2. **No TEST_MAPROOM_DATABASE_URL propagation**: Configs hardcode MAPROOM_DATABASE_URL
3. **No CI/CD isolation**: GitHub Actions likely use development database
4. **No documentation**: Developers don't know how to set up isolated testing

## Industry Solutions

### Approach 1: Separate Docker Compose Projects
- Dev: `docker compose up` (one project)
- Test: `docker compose -f docker-compose.test-db.yml up` (separate project)
- **Pros**: Clean separation, independent lifecycles
- **Cons**: Developers must remember to start both, cognitive overhead

### Approach 2: Single Compose with Multiple Services
- Single `docker-compose.yml` with both `postgres` and `postgres-test`
- One `docker compose up` starts everything
- **Pros**: Developer ergonomics, one command starts all infrastructure
- **Cons**: Always runs both databases (minimal cost)

### Approach 3: Environment-Specific Overrides
- Base: `docker-compose.yml`
- Test: `docker-compose.test.yml` (actual database override, not build override)
- Dev: `docker-compose.override.yml`
- **Pros**: Flexible, follows Docker Compose conventions
- **Cons**: More files to manage, compose -f flag required

## Recommended Solution: Approach 2

**Single compose file with both databases** for these reasons:

1. **Developer Experience**: `docker compose up` just works
2. **Resource Cost**: Negligible (postgres containers are lightweight)
3. **Mental Model**: "Infrastructure is always ready for both dev and test"
4. **Discoverability**: Everything visible in one file
5. **Backward Compatible**: Existing dev database unchanged

## Research Findings

### PostgreSQL Isolation Patterns

**Key insight**: Same schema, different database instances
- Both databases use `init.sql` for schema initialization
- Separate volumes prevent data sharing
- Different ports (5433 dev, 5434 test) prevent accidental connections

### Environment Variable Hierarchy

Best practice discovered in test helpers:
```typescript
TEST_MAPROOM_DATABASE_URL || MAPROOM_DATABASE_URL
```

This pattern:
- Explicitly prioritizes test database when set
- Falls back gracefully to dev database for backward compatibility
- Self-documenting (TEST_ prefix makes intent clear)

### CI/CD Considerations

GitHub Actions workflows need:
- Service containers for databases OR
- Docker Compose for local-like environment
- TEST_MAPROOM_DATABASE_URL env var in workflow

## Success Criteria

Isolation verified when:
1. **Parallel execution**: Can run `pnpm dev` (using dev DB) and `pnpm test` (using test DB) simultaneously
2. **Data independence**: Test data in postgres-test (5434) not visible to dev database (5433)
3. **CI reliability**: GitHub Actions tests pass using test database
4. **Zero configuration**: `docker compose up && pnpm test` works out of the box
