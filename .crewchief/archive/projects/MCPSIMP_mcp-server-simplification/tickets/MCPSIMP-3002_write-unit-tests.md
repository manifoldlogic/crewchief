# Ticket: MCPSIMP-3002: Write resolveDatabase Unit Tests

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - tests executed and passing
- [ ] **Verified** - by the verify-ticket agent

## Agents
- general-purpose
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary
Write unit tests for the `resolveDatabase()` function in the new CLI entry point to verify all three database resolution paths work correctly.

## Background
The simplified CLI entry point includes a `resolveDatabase()` function that determines the database URL using a three-tier hierarchy. This is critical functionality that needs test coverage to ensure:
1. Explicit `MAPROOM_DATABASE_URL` always wins
2. `IN_DEVCONTAINER=true` triggers container hostname
3. Default fallback to localhost:5433

This implements Phase 3.2 of the MCP Server Simplification plan.

## Acceptance Criteria
- [ ] Test file created at `packages/maproom-mcp/tests/unit/resolve-database.test.ts`
- [ ] Test: uses `MAPROOM_DATABASE_URL` when explicitly set
- [ ] Test: uses container hostname when `IN_DEVCONTAINER=true`
- [ ] Test: defaults to `localhost:5433` when neither is set
- [ ] All tests pass when run with `pnpm test`

## Technical Requirements
Create test file with the following tests:

```typescript
// packages/maproom-mcp/tests/unit/resolve-database.test.ts
import { describe, test, expect, beforeEach, afterEach } from 'vitest'

describe('resolveDatabase', () => {
  const originalEnv = process.env

  beforeEach(() => {
    // Reset environment before each test
    process.env = { ...originalEnv }
    delete process.env.MAPROOM_DATABASE_URL
    delete process.env.IN_DEVCONTAINER
  })

  afterEach(() => {
    process.env = originalEnv
  })

  test('uses MAPROOM_DATABASE_URL when set', () => {
    process.env.MAPROOM_DATABASE_URL = 'postgresql://custom@host:5432/db'
    // Import or call resolveDatabase
    // Expect: 'postgresql://custom@host:5432/db'
  })

  test('uses container hostname when IN_DEVCONTAINER=true', () => {
    process.env.IN_DEVCONTAINER = 'true'
    // Import or call resolveDatabase
    // Expect: 'postgresql://maproom:maproom@maproom-postgres:5432/maproom'
  })

  test('defaults to localhost:5433', () => {
    // No env vars set
    // Import or call resolveDatabase
    // Expect: 'postgresql://maproom:maproom@localhost:5433/maproom'
  })

  test('MAPROOM_DATABASE_URL takes precedence over IN_DEVCONTAINER', () => {
    process.env.MAPROOM_DATABASE_URL = 'postgresql://explicit@host:5432/db'
    process.env.IN_DEVCONTAINER = 'true'
    // Import or call resolveDatabase
    // Expect: 'postgresql://explicit@host:5432/db'
  })
})
```

## Implementation Notes
- The `resolveDatabase` function is in `bin/cli.cjs` which is JavaScript
- You may need to:
  - Export the function from cli.cjs for testing
  - Or create a separate module that cli.cjs imports
  - Or test by actually running the CLI with different env vars
- If modifying cli.cjs to export the function, keep the main functionality unchanged
- Follow the existing test patterns in maproom-mcp package
- Run tests with `pnpm test` from `packages/maproom-mcp`

## Dependencies
- **MCPSIMP-1001** (Replace CLI Entry Point) - The function must exist before testing

## Risk Assessment
- **Risk**: cli.cjs doesn't export resolveDatabase for testing
  - **Mitigation**: Refactor to export, or test via integration (run CLI with different envs)
- **Risk**: Test framework not set up for this test location
  - **Mitigation**: Check vitest.config.ts for test file patterns; adjust if needed

## Files/Packages Affected
- `packages/maproom-mcp/tests/unit/resolve-database.test.ts` (create)
- `packages/maproom-mcp/bin/cli.cjs` (potentially modify to export function)
