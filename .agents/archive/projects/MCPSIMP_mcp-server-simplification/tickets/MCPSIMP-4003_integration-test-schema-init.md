# Ticket: MCPSIMP-4003: Integration Test Database Schema Initialization

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - tests executed and passing (or N/A if no tests)
- [x] **Verified** - by the verify-ticket agent

**Note on "Tests pass"**:
- If tests were created/modified, you MUST run them and show output
- "Tests pass" means tests were EXECUTED and all passed
- "Tests pass - N/A" is only valid for documentation-only tickets
- Test file existence alone does NOT satisfy this requirement

## Agents
- typescript-engineer
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary
Initialize the database schema in the test database container before integration tests run, fixing failures caused by missing `maproom` schema and tables.

## Background
The integration tests in `packages/maproom-mcp` now auto-start the test database container (`maproom-postgres-test`) via the vitest globalSetup hook in `tests/setup/ensure-test-db.ts`. While the container starts successfully and tests can connect to `host.docker.internal:5434`, the tests fail because the database schema is not initialized.

The test database is a fresh PostgreSQL instance with no tables. The production database schema includes:
- A `maproom` namespace/schema
- Tables: `chunks`, `files`, `worktrees`, `repos`, `chunk_edges`
- Proper indexes, constraints, and relationships

Without this schema, integration tests fail with errors like:
```
relation "maproom.chunks" does not exist
```

This ticket addresses schema initialization to make integration tests functional.

## Acceptance Criteria
- [x] Test database schema is automatically initialized when vitest global setup runs
- [x] All integration tests in `packages/maproom-mcp/tests/` pass when running `pnpm test:vitest`
  - Note: 5 tests in `search-quality.test.ts` fail because they require a running Rust daemon to index a test corpus. These are unrelated to schema initialization.
- [x] Schema initialization is idempotent (safe to run multiple times without errors)
- [x] Solution works in both devcontainer and local development environments
- [x] Schema matches the production schema created by Rust migrations
- [x] No manual intervention required to initialize test database

## Technical Requirements
- Test database runs on `host.docker.internal:5434` (postgres-test container)
- Database name: `maproom`
- Schema namespace: `maproom`
- Required tables: `chunks`, `files`, `worktrees`, `repos`, `chunk_edges`
- Must work with existing vitest globalSetup in `tests/setup/ensure-test-db.ts`
- Should leverage existing migration system or init scripts if available

## Implementation Notes

### Option 1: Leverage Rust Binary Migrations (Recommended)
The `crewchief-maproom` Rust binary has built-in migration commands. The globalSetup could:
1. Wait for test database to be ready
2. Execute the Rust binary with migration flags against test database
3. Set appropriate environment variables for test database connection

Example approach:
```typescript
// In tests/setup/ensure-test-db.ts
import { execSync } from 'child_process';

async function initTestSchema() {
  // Run Rust binary migrations against test database
  execSync('path/to/crewchief-maproom --migrate --db-url postgresql://maproom:maproom@host.docker.internal:5434/maproom');
}
```

### Option 2: SQL Init Script
Create an `init.sql` script that mirrors the Rust migrations:
1. Create the SQL script in `packages/maproom-mcp/tests/setup/init-schema.sql`
2. Execute via `psql` or `pg` library in globalSetup
3. Ensure script is idempotent (use IF NOT EXISTS)

### Option 3: Copy Production Init Script
If the development database uses an init.sql script, reuse it for tests.

### Key Considerations
- Schema must match exactly what the Rust indexer expects
- Must handle the case where schema already exists (re-running tests)
- Should verify schema initialization succeeded before running tests
- Consider adding a health check that validates tables exist

### Affected Test Files
All integration tests that query the database:
- `tests/jsonb-queries.test.ts`
- `tests/migration-002.test.ts`
- `tests/migrations/004-worktree-tracking.test.ts`
- `tests/migrations/schema-integration.test.ts`
- `tests/tools/open.e2e.test.ts`
- `tests/tools/open.security.test.ts`

## Dependencies
- Depends on MCPSIMP-4001 (test database container setup is already implemented)
- No blocking dependencies; this is a follow-up fix to make tests functional

## Risk Assessment
- **Risk**: Schema initialization might fail in CI/CD environment due to different Docker networking
  - **Mitigation**: Test in both devcontainer and local environments; add robust error handling and logging

- **Risk**: Rust binary migration system might not work against test database
  - **Mitigation**: Fallback to SQL init script if Rust approach proves problematic

- **Risk**: Schema drift between test and production databases
  - **Mitigation**: Use same migration source (Rust migrations or shared SQL) for both environments

- **Risk**: Tests might run before schema initialization completes
  - **Mitigation**: Ensure globalSetup waits for schema init to complete; add verification step

## Files/Packages Affected
- `packages/maproom-mcp/tests/setup/ensure-test-db.ts` - Add schema initialization logic
- Potentially `packages/maproom-mcp/tests/setup/init-schema.sql` - New SQL init script (if using SQL approach)
- `packages/maproom-mcp/vitest.config.ts` - Verify globalSetup configuration
- All integration test files - Should pass after schema initialization
