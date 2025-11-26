# Ticket: MCPDB-1003: SQLite Test Helpers

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - tests executed and passing (build passes)
- [x] **Verified** - by the verify-ticket agent

## Agents
- integration-tester
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary
Create separate SQLite test helpers in `tests/helpers/sqlite.ts` for managing test fixtures, enabling SQLite-based integration tests without modifying existing PostgreSQL helpers.

## Background
The existing test helpers (`tests/helpers/database.ts`) are PostgreSQL-specific. For SQLite tests:
- Use a SEPARATE helper file (not abstraction layer)
- Work with pre-indexed fixture from MAPCLI
- Copy fixture to temp location for test isolation

**Plan Reference:** Phase 3 - Test Infrastructure (plan.md, quality-strategy.md)

## Acceptance Criteria
- [x] `tests/helpers/sqlite.ts` created with fixture management functions
- [x] `createTestSqliteDatabase()` copies fixture to temp location and returns path
- [x] `cleanupTestSqliteDatabase()` removes temp database file
- [x] `getSqliteFixturePath()` returns path to source fixture
- [x] Helper throws descriptive error if fixture doesn't exist
- [x] Existing PostgreSQL helpers (`helpers/database.ts`) unchanged
- [x] Helper functions are fully typed with TypeScript

## Technical Requirements

### File Location
`packages/maproom-mcp/tests/helpers/sqlite.ts`

### Fixture Location
`crates/maproom/tests/fixtures/pre-indexed-maproom.db` (verified exists, 19MB)

### Helper Implementation
```typescript
/**
 * SQLite test utilities (SEPARATE from database.ts)
 * Does NOT modify or interact with PostgreSQL helpers
 */
import { copyFileSync, unlinkSync, existsSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'

const __dirname = path.dirname(fileURLToPath(import.meta.url))

// Relative path from packages/maproom-mcp/tests/helpers to fixture
const FIXTURE_SOURCE = resolve(
  __dirname,
  '../../../../crates/maproom/tests/fixtures/pre-indexed-maproom.db'
)

/**
 * Create a copy of the SQLite test fixture for isolated testing
 * @returns Path to the temporary test database
 */
export function createTestSqliteDatabase(): string {
  if (!existsSync(FIXTURE_SOURCE)) {
    throw new Error(
      `SQLite fixture not found: ${FIXTURE_SOURCE}\n` +
      `Run: cargo test --features sqlite --test create_sqlite_fixture -- --ignored`
    )
  }

  const testDbPath = join(tmpdir(), `maproom-test-${Date.now()}-${Math.random().toString(36).slice(2)}.db`)
  copyFileSync(FIXTURE_SOURCE, testDbPath)
  return testDbPath
}

/**
 * Clean up a temporary test database
 * @param path - Path to the test database to remove
 */
export function cleanupTestSqliteDatabase(path: string): void {
  try {
    if (existsSync(path)) {
      unlinkSync(path)
    }
  } catch {
    // Ignore cleanup errors (file may already be deleted)
  }
}

/**
 * Get the path to the source SQLite fixture
 * Useful for read-only tests that don't need isolation
 */
export function getSqliteFixturePath(): string {
  if (!existsSync(FIXTURE_SOURCE)) {
    throw new Error(
      `SQLite fixture not found: ${FIXTURE_SOURCE}\n` +
      `Run: cargo test --features sqlite --test create_sqlite_fixture -- --ignored`
    )
  }
  return FIXTURE_SOURCE
}

/**
 * Get SQLite database URL for testing
 * @param dbPath - Optional path to database (uses fixture if not provided)
 * @returns SQLite URL suitable for MAPROOM_DATABASE_URL
 */
export function getSqliteTestUrl(dbPath?: string): string {
  const path = dbPath || getSqliteFixturePath()
  return `sqlite://${path}`
}
```

### Key Design Decisions
1. **Separate file**: No modifications to `database.ts`
2. **Fixture-based**: Uses pre-indexed data, no runtime indexing
3. **Temp copies**: Each test gets isolated database copy
4. **Error guidance**: Clear message if fixture missing with regeneration command

## Implementation Notes

### Why Separate Files (Not Abstraction)
Per quality-strategy.md:
- No complex abstraction layer
- Each backend tested independently
- No risk of PostgreSQL helper changes breaking SQLite tests
- Simpler to understand and maintain

### Fixture Contents
The `pre-indexed-maproom.db` fixture contains:
- Indexed code from the maproom crate test corpus
- FTS5 full-text search data
- Ready-to-query test data

### Path Resolution
The path resolution uses `__dirname` relative navigation:
```
packages/maproom-mcp/tests/helpers/sqlite.ts
    → ../../../../
    → crates/maproom/tests/fixtures/pre-indexed-maproom.db
```

### Test Usage Pattern
```typescript
import { createTestSqliteDatabase, cleanupTestSqliteDatabase, getSqliteTestUrl } from '../helpers/sqlite.js'

describe('MCP Tools with SQLite', () => {
  let testDbPath: string

  beforeAll(() => {
    testDbPath = createTestSqliteDatabase()
    process.env.MAPROOM_DATABASE_URL = getSqliteTestUrl(testDbPath)
  })

  afterAll(() => {
    cleanupTestSqliteDatabase(testDbPath)
  })

  // Tests here...
})
```

## Dependencies
- **MCPDB-1001**: URL parsing (for `sqlite://` URL format understanding)
- **External**: Pre-indexed SQLite fixture must exist at `crates/maproom/tests/fixtures/pre-indexed-maproom.db`

## Risk Assessment
- **Risk**: Fixture doesn't exist in CI
  - **Mitigation**: CI job ensures fixture exists or generates it; error message includes regeneration command
- **Risk**: Path resolution breaks on Windows
  - **Mitigation**: Use `path.join` and `path.resolve` for cross-platform compatibility

## Files/Packages Affected
- `packages/maproom-mcp/tests/helpers/sqlite.ts` (create)
- `packages/maproom-mcp/tests/helpers/database.ts` (NO changes)
