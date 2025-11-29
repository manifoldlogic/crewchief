# Ticket: TESTISO-1002: Update vitest configuration for TEST_MAPROOM_DATABASE_URL

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - existing test suite executed and passing in both modes
- [x] **Verified** - by the verify-ticket agent

**Note on "Tests pass"**:
- If tests were created/modified, you MUST run them and show output
- "Tests pass" means tests were EXECUTED and all passed
- "Tests pass - N/A" is only valid for documentation-only tickets
- Test file existence alone does NOT satisfy this requirement

## Agents
- general-implementation
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary
Update vitest.config.ts to use TEST_MAPROOM_DATABASE_URL environment variable when set, with fallback to test database using container hostname, enabling test database isolation while maintaining backward compatibility.

## Background
With postgres-test service running (from TESTISO-1001), we need to configure the test framework to use it. Currently, vitest.config.ts hardcodes the dev database connection (`maproom-postgres:5432/maproom`). This ticket updates the configuration to respect TEST_MAPROOM_DATABASE_URL with fallback to the test database, enabling flexible database selection.

The test helper (`packages/maproom-mcp/tests/helpers/database.ts`) already supports this pattern:
```typescript
export function getDatabaseUrl(): string {
  const dbUrl = process.env.TEST_MAPROOM_DATABASE_URL || process.env.MAPROOM_DATABASE_URL
  // ...
}
```

We need to ensure vitest.config.ts sets the environment correctly so tests use the isolated test database by default.

This implements Phase 2 (Test Configuration) from the TESTISO project plan.

## Acceptance Criteria
- [x] vitest.config.ts updated to use TEST_MAPROOM_DATABASE_URL when set
- [x] Falls back to test database (maproom-postgres-test:5432/maproom_test) when TEST_MAPROOM_DATABASE_URL not present
- [x] Uses container hostname (maproom-postgres-test:5432) not localhost in default fallback
- [x] Configuration change validated by running existing test suite successfully
- [x] Tests pass with both TEST_MAPROOM_DATABASE_URL set and unset
- [x] Code comments explain hostname choice (container vs localhost)

## Technical Requirements

**File to Modify**: `packages/maproom-mcp/vitest.config.ts`

**Current Configuration** (line 8-10):
```typescript
env: {
  MAPROOM_DATABASE_URL: 'postgresql://maproom:maproom@maproom-postgres:5432/maproom'
}
```

**Updated Configuration**:
```typescript
env: {
  // Use TEST_MAPROOM_DATABASE_URL if set, fall back to default test database
  // NOTE: Uses container hostname (maproom-postgres-test) because tests run
  // on host but connect through Docker network. When TEST_MAPROOM_DATABASE_URL
  // is not set, tests should use the test database by default.
  MAPROOM_DATABASE_URL:
    process.env.TEST_MAPROOM_DATABASE_URL ||
    'postgresql://maproom:maproom@maproom-postgres-test:5432/maproom_test'
}
```

## Implementation Notes

### Critical: Container Hostname vs Localhost

Tests execute on the **host machine** (via `pnpm test`), but connect to databases via **Docker network**. Therefore:

- **vitest.config.ts**: Use container hostname `maproom-postgres-test:5432`
  - Tests run on host but can access Docker network
  - Container hostname is visible via Docker network
  - This is the fallback when TEST_MAPROOM_DATABASE_URL is not set

- **package.json scripts** (TESTISO-1003): Use `localhost:5434`
  - Environment variable set from host perspective
  - Host connects to mapped port

### Why Container Hostname in vitest.config.ts?

The fallback URL in vitest.config.ts uses the container hostname because when TEST_MAPROOM_DATABASE_URL is not set, the test process (running on host) will use this URL to connect through the Docker network to the container's internal port 5432.

### Environment Variable Priority

1. `TEST_MAPROOM_DATABASE_URL` (highest - test-specific override)
2. Fallback to test database with container hostname: `maproom-postgres-test:5432/maproom_test`

### Backward Compatibility

If TEST_MAPROOM_DATABASE_URL is not set, tests use test database by default (via fallback). This is intentional - we want tests to use the test database by default, not the dev database.

### Changes Required

1. Update `env.MAPROOM_DATABASE_URL` assignment (line 9)
2. Add multi-line comment explaining hostname choice (lines 8-11)
3. Change fallback from `maproom-postgres:5432/maproom` to `maproom-postgres-test:5432/maproom_test`

## Dependencies

**Depends on**:
- TESTISO-1001 (postgres-test service must be running and healthy)

**Blocks**:
- TESTISO-1003 (package.json scripts need this config)

## Risk Assessment

- **Risk**: Hostname confusion (localhost vs container name)
  - **Mitigation**: Clear documentation in code comments explaining when to use each
  - **Validation**: Test both with and without TEST_MAPROOM_DATABASE_URL set

- **Risk**: Tests fail with new configuration
  - **Mitigation**: Verify postgres-test is healthy before running tests via `docker ps`
  - **Recovery**: Check database connection string, verify container accessibility

- **Risk**: Breaking backward compatibility
  - **Mitigation**: Fallback pattern ensures tests work with or without TEST_MAPROOM_DATABASE_URL
  - **Validation**: Run tests in both modes to confirm

## Files/Packages Affected

**Modified**:
- `packages/maproom-mcp/vitest.config.ts` (lines 8-10)

**No changes needed**:
- `packages/maproom-mcp/tests/helpers/database.ts` (already supports the pattern)

## Validation Steps

After implementing, verify:

```bash
# Ensure postgres-test is running
docker ps | grep maproom-postgres-test

# Test with TEST_MAPROOM_DATABASE_URL set (localhost from host perspective)
TEST_MAPROOM_DATABASE_URL=postgresql://maproom:maproom@localhost:5434/maproom_test pnpm test

# Test without TEST_MAPROOM_DATABASE_URL (should use fallback with container hostname)
unset TEST_MAPROOM_DATABASE_URL
pnpm test

# Verify both scenarios pass
# No new tests needed - existing test suite validates configuration
```

## References

- Planning doc: `/workspace/.crewchief/projects/TESTISO_test-database-isolation/planning/plan.md` (Phase 2)
- Architecture: `/workspace/.crewchief/projects/TESTISO_test-database-isolation/planning/architecture.md` (Container vs Host Context section)
- Test helper: `packages/maproom-mcp/tests/helpers/database.ts` (shows existing getDatabaseUrl() pattern)
- Current config: `packages/maproom-mcp/vitest.config.ts` (line 9 needs update)

## Success Definition

Ticket complete when:
- vitest.config.ts uses TEST_MAPROOM_DATABASE_URL with fallback
- Fallback uses container hostname (maproom-postgres-test:5432) and test database name (maproom_test)
- Existing test suite passes with configuration
- Tests validated in both modes (with and without TEST_MAPROOM_DATABASE_URL)
- Code comments explain hostname choice and environment variable priority
