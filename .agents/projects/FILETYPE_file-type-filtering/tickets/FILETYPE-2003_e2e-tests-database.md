# Ticket: FILETYPE-2003: Create E2E Tests with Database

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - tests executed and passing (or N/A if no tests)
- [ ] **Verified** - by the verify-ticket agent

## Agents
- typescript-test-engineer
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary
Create 5 end-to-end tests with real database queries to verify actual file_type filtering behavior in production-like conditions.

## Background
E2E tests validate the complete workflow from MCP request through SQL execution to result filtering. These tests use a real PostgreSQL database with known test data to ensure the feature works correctly beyond unit and integration test mocks.

**Reference:**
- quality-strategy.md - "E2E Tests: Real Database Queries (5 tests)" section (lines 400-536)
- quality-strategy.md - "Test File Organization" - NEW FILE in filters/ directory (lines 79-88)

## Acceptance Criteria
- [ ] All 5 E2E tests pass
- [ ] Single extension filter returns only matching files
- [ ] Multi-extension filter returns union of all specified types
- [ ] Case insensitive matching works correctly
- [ ] Empty filter returns all files (graceful fallback)
- [ ] Tests run in <2 seconds with test database

## Technical Requirements

**Location:** `packages/maproom-mcp/tests/filters/file-type.e2e.test.ts` (NEW FILE)

**Database setup required:**
```typescript
import { describe, it, expect, beforeAll, afterAll } from 'vitest'
import { Client } from 'pg'
import { handleSearch } from '../src/index.js'

let testDb: Client

beforeAll(async () => {
  // Setup test database with known data
  testDb = new Client({
    connectionString: process.env.TEST_DATABASE_URL ||
      'postgresql://maproom:maproom@localhost:5432/maproom_test'
  })
  await testDb.connect()

  // Insert test data with known file types
  await testDb.query(`
    INSERT INTO maproom.files (relpath, worktree_id, last_modified, ...)
    VALUES
      ('src/auth.ts', 1, NOW(), ...),
      ('src/auth.spec.ts', 1, NOW(), ...),
      ('src/api.tsx', 1, NOW(), ...),
      ('src/utils.js', 1, NOW(), ...),
      ('README.md', 1, NOW(), ...),
      ('package.json', 1, NOW(), ...)
  `)
})

afterAll(async () => {
  await testDb.query('TRUNCATE maproom.files CASCADE')
  await testDb.end()
})
```

**Test suite structure:**
```typescript
describe('File Type Filter - E2E Tests', () => {
  // Single extension (P0)
  it('returns only TypeScript files for file_type=ts', async () => {
    const result = await handleSearch({
      repo: 'test-repo',
      query: 'auth',
      filters: { file_type: 'ts' }
    })

    expect(result.hits).toBeDefined()
    expect(result.hits.every(hit => hit.relpath.endsWith('.ts'))).toBe(true)
    expect(result.hits.some(hit => hit.relpath === 'src/auth.ts')).toBe(true)
    expect(result.hits.every(hit => !hit.relpath.endsWith('.tsx'))).toBe(true)
    expect(result.hits.every(hit => !hit.relpath.endsWith('.md'))).toBe(true)
  })

  // Multi-extension (P0)
  it('returns union of multiple file types', async () => {
    const result = await handleSearch({
      repo: 'test-repo',
      query: 'auth',
      filters: { file_type: 'ts,tsx,js' }
    })

    const extensions = result.hits.map(hit =>
      hit.relpath.split('.').pop()
    )

    expect(extensions).toContain('ts')
    expect(extensions).toContain('tsx')
    expect(extensions).toContain('js')
    expect(extensions).not.toContain('md')
    expect(extensions).not.toContain('json')
  })

  // Case insensitive (P0)
  it('handles uppercase extensions same as lowercase', async () => {
    const lower = await handleSearch({
      repo: 'test-repo',
      query: 'auth',
      filters: { file_type: 'ts' }
    })

    const upper = await handleSearch({
      repo: 'test-repo',
      query: 'auth',
      filters: { file_type: 'TS' }
    })

    expect(lower.hits.length).toBe(upper.hits.length)
    expect(lower.hits.map(h => h.chunk_id).sort()).toEqual(
      upper.hits.map(h => h.chunk_id).sort()
    )
  })

  // Empty filter (P0)
  it('returns all file types when file_type is empty', async () => {
    const result = await handleSearch({
      repo: 'test-repo',
      query: 'auth',
      filters: { file_type: '' }
    })

    const extensions = new Set(result.hits.map(hit =>
      hit.relpath.split('.').pop()
    ))

    // Should include various types (not filtered)
    expect(extensions.size).toBeGreaterThan(1)
  })

  // Performance (P2)
  it('completes search with filter in <200ms', async () => {
    const start = Date.now()

    await handleSearch({
      repo: 'test-repo',
      query: 'auth',
      filters: { file_type: 'ts,tsx,js' }
    })

    const duration = Date.now() - start
    expect(duration).toBeLessThan(200)
  })
})
```

## Implementation Notes

**Test database setup:**
- Use TEST_DATABASE_URL environment variable
- Create test database schema if needed
- Insert known test data with specific file types
- Clean up after tests (TRUNCATE tables)

**File organization:**
- Same `tests/filters/` directory as integration tests
- `.e2e.test.ts` suffix distinguishes from integration tests
- Can be excluded from watch mode (slow tests)

**What these tests verify:**
1. Feature works end-to-end with real database
2. File filtering actually filters (not just SQL generation)
3. Case normalization works in PostgreSQL LIKE queries
4. Empty filter gracefully falls back to all files
5. Performance acceptable (<200ms)

**Test execution:**
```bash
# Run only E2E tests
pnpm test filters/*.e2e.test.ts

# Run with test database
TEST_DATABASE_URL=postgresql://maproom:maproom@localhost:5432/maproom_test pnpm test file-type.e2e
```

**Database test data requirements:**
- At least one .ts file
- At least one .tsx file
- At least one .js file
- At least one .md file
- At least one .json file
- Files should contain searchable content matching test queries

## Dependencies
- **FILETYPE-1002** (parseFileTypeFilter implemented)
- **FILETYPE-1003** (buildFilterClauses updated)
- **FILETYPE-1004** (handleSearch validation added)

## Risk Assessment
- **Risk**: Test database not available in CI
  - **Mitigation:** Use Docker Compose or GitHub Actions services for PostgreSQL

- **Risk**: Tests flaky due to database state
  - **Mitigation:** Clean setup/teardown in beforeAll/afterAll

- **Risk**: Tests slow (>5 seconds)
  - **Mitigation:** Use small test dataset, exclude from watch mode

## Files/Packages Affected
- `packages/maproom-mcp/tests/filters/file-type.e2e.test.ts` (NEW FILE)
- Test database (maproom_test schema)
