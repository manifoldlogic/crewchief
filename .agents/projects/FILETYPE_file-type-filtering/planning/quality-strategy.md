# Quality Strategy: File Type Filtering

**Project:** FILETYPE - File Type Filtering
**Date:** 2025-11-19
**Philosophy:** Pragmatic testing focused on confidence, not ceremony

---

## Testing Philosophy

**Ship with confidence, not 100% coverage.** Tests should prevent rework, not become rework themselves.

### Core Principles

1. **Test critical paths first** - The 90% use cases that users will actually hit
2. **Integration over isolation** - Verify the feature works end-to-end
3. **MVP testing** - Cover essentials now, expand later if bugs emerge
4. **Fast feedback** - Tests run in <5 seconds, iterate quickly

**Anti-pattern to avoid:** Writing tests for every theoretical edge case before proving the feature works.

---

## Test Pyramid

```
         ┌─────────────┐
         │   E2E (5)   │  Real database, full workflow
         │             │  "Search returns only .ts files"
         ├─────────────┤
         │ Integration │  SQL generation, filter logic
         │    (10)     │  "buildFilterClauses produces correct SQL"
         ├─────────────┤
         │   Unit      │  Pure functions, parsing
         │   (15)      │  "parseFileTypeFilter handles edge cases"
         └─────────────┘
```

**Ratio:** 3:2:1 (Unit:Integration:E2E)
**Total:** ~30 tests
**Run time:** <5 seconds

---

## Test File Organization

This section specifies **exact file locations** for all tests to resolve the test organization ambiguity identified in the project review.

### File Structure

```
packages/maproom-mcp/tests/
├── search_tool.test.ts          # ← EXTEND (add parseFileTypeFilter unit tests)
├── filters/                     # ← NEW DIRECTORY
│   ├── file-type.int.test.ts   # ← NEW FILE (integration tests)
│   └── file-type.e2e.test.ts   # ← NEW FILE (E2E tests)
└── ... (existing test files)
```

### Test Type → File Mapping

**1. Unit Tests (15 tests)**
- **Location:** `packages/maproom-mcp/tests/search_tool.test.ts`
- **Action:** EXTEND existing file (add new describe block)
- **Rationale:**
  - Keep all search tool parameter tests together
  - parseFileTypeFilter is called from search tool
  - Existing file already has filter handling tests
  - No need for separate file for simple pure function tests

**2. Integration Tests (10 tests)**
- **Location:** `packages/maproom-mcp/tests/filters/file-type.int.test.ts` (NEW FILE)
- **Action:** CREATE new file in new `filters/` directory
- **Rationale:**
  - Isolate database-dependent SQL generation tests
  - Create reusable pattern for future filter tests
  - Keep integration tests separate from unit tests
  - Allows running unit vs integration tests separately

**3. E2E Tests (5 tests)**
- **Location:** `packages/maproom-mcp/tests/filters/file-type.e2e.test.ts` (NEW FILE)
- **Action:** CREATE new file in `filters/` directory
- **Rationale:**
  - Full workflow tests require database setup
  - E2E tests may be slow (skip in watch mode)
  - Clear separation from faster integration tests
  - Follows existing pattern (e.g., `tools/open.e2e.test.ts`)

### Implementation Guidance

**When adding unit tests to search_tool.test.ts:**
```typescript
// Add after existing describe blocks
describe('parseFileTypeFilter - File Type Parsing', () => {
  // 15 unit test cases here
  it('should parse single extension', () => { ... })
  it('should parse comma-separated extensions', () => { ... })
  // ... etc
})
```

**When creating filters/ directory:**
```bash
mkdir -p packages/maproom-mcp/tests/filters
```

**File naming convention:**
- Pattern: `{feature}.{test-type}.test.ts`
- Examples:
  - `file-type.int.test.ts` - Integration tests
  - `file-type.e2e.test.ts` - E2E tests
  - `file-type.perf.test.ts` - Performance tests (future)

### Vitest Configuration

**Run specific test types:**
```bash
# Unit tests only (fast)
pnpm test search_tool.test.ts

# Integration tests only
pnpm test filters/*.int.test.ts

# E2E tests only (slow)
pnpm test filters/*.e2e.test.ts

# All file-type tests
pnpm test file-type
```

**Watch mode (excludes E2E):**
```bash
pnpm test:watch --testPathIgnorePatterns=e2e
```

### Test Organization Benefits

1. **Clear separation:** Unit (fast) vs Integration (medium) vs E2E (slow)
2. **Selective execution:** Run only needed test tier during development
3. **Reusable pattern:** Future filters can follow same structure
4. **Backward compatible:** Existing tests in search_tool.test.ts unchanged
5. **Discoverable:** New `filters/` directory clearly signals filter-specific tests

---

## Critical Path Testing

### User Journey Priority

**P0 (Must work):**
1. Single extension filter: `{file_type: "ts"}` → only .ts files
2. Multi-extension filter: `{file_type: "ts,tsx,js"}` → .ts OR .tsx OR .js
3. Case insensitive: `{file_type: "TS"}` → same as "ts"
4. Empty filter ignored: `{file_type: ""}` → no error, all files

**P1 (Should work):**
5. Whitespace tolerance: `{file_type: " ts , tsx "}` → ["ts", "tsx"]
6. Dot handling: `{file_type: ".ts,.tsx"}` → ["ts", "tsx"]
7. Filter combination: file_type + recency_threshold
8. Error on too many extensions (>20)

**P2 (Nice to have):**
9. Helpful hint on empty result
10. Performance acceptable (<20% overhead)

---

## Test Suite Breakdown

### 1. Unit Tests: `parseFileTypeFilter` (15 tests)

**Location:** `packages/maproom-mcp/tests/search_tool.test.ts`

**Coverage:**

```typescript
describe('parseFileTypeFilter - Unit Tests', () => {
  // Basic functionality (P0)
  it('parses single extension', () => {
    expect(parseFileTypeFilter('ts')).toEqual(['ts'])
  })

  it('parses multiple extensions', () => {
    expect(parseFileTypeFilter('ts,tsx,js')).toEqual(['ts', 'tsx', 'js'])
  })

  // Case normalization (P0)
  it('normalizes to lowercase', () => {
    expect(parseFileTypeFilter('TS,TSX')).toEqual(['ts', 'tsx'])
  })

  it('handles mixed case', () => {
    expect(parseFileTypeFilter('Ts,TSX,js')).toEqual(['ts', 'tsx', 'js'])
  })

  // Whitespace handling (P1)
  it('trims whitespace', () => {
    expect(parseFileTypeFilter('  ts  ,  tsx  ')).toEqual(['ts', 'tsx'])
  })

  it('handles spaces around commas', () => {
    expect(parseFileTypeFilter('ts , tsx , js')).toEqual(['ts', 'tsx', 'js'])
  })

  // Dot handling (P1)
  it('strips leading dots', () => {
    expect(parseFileTypeFilter('.ts,.tsx')).toEqual(['ts', 'tsx'])
  })

  it('handles mixed dot/no-dot', () => {
    expect(parseFileTypeFilter('.ts,tsx,.js')).toEqual(['ts', 'tsx', 'js'])
  })

  // Empty input (P0)
  it('returns empty array for empty string', () => {
    expect(parseFileTypeFilter('')).toEqual([])
  })

  it('returns empty array for whitespace only', () => {
    expect(parseFileTypeFilter('   ')).toEqual([])
  })

  it('returns empty array for commas only', () => {
    expect(parseFileTypeFilter(',,,')).toEqual([])
  })

  // Trailing/leading comma (P1)
  it('ignores trailing comma', () => {
    expect(parseFileTypeFilter('ts,tsx,')).toEqual(['ts', 'tsx'])
  })

  it('ignores leading comma', () => {
    expect(parseFileTypeFilter(',ts,tsx')).toEqual(['ts', 'tsx'])
  })

  // Complex combinations
  it('handles all edge cases at once', () => {
    expect(parseFileTypeFilter('  .TS , tsx,  , .JS  ,')).toEqual(['ts', 'tsx', 'js'])
  })

  // Limit validation (P1)
  it('handles exactly 20 extensions', () => {
    const twentyExt = Array(20).fill('ts').join(',')
    expect(parseFileTypeFilter(twentyExt).length).toBe(20)
  })
})
```

**Why these tests:**
- Cover all branches in parseFileTypeFilter
- Verify normalization logic (case, dots, whitespace)
- Edge cases users will accidentally hit (trailing comma, spaces)
- No theoretical cases that won't happen in practice

**Time to run:** <1 second

---

### 2. Integration Tests: SQL Generation (10 tests)

**Location:** `packages/maproom-mcp/tests/search_tool.test.ts`

**Coverage:**

```typescript
describe('buildFilterClauses - Integration Tests', () => {
  // Single extension SQL (P0)
  it('generates correct SQL for single extension', () => {
    const args: any[] = [1] // repoId
    const filters = { file_type: 'ts' }
    const clauses = buildFilterClauses(filters, 'all', args)

    expect(clauses).toContain("f.relpath LIKE $2")
    expect(args).toContain('%.ts')
    expect(args.length).toBe(2)
  })

  // Multi-extension SQL (P0)
  it('generates correct OR clause for multiple extensions', () => {
    const args: any[] = [1]
    const filters = { file_type: 'ts,tsx,js' }
    const clauses = buildFilterClauses(filters, 'all', args)

    expect(clauses).toContain('(f.relpath LIKE')
    expect(clauses).toContain(' OR ')
    expect(args).toContain('%.ts')
    expect(args).toContain('%.tsx')
    expect(args).toContain('%.js')
    expect(args.length).toBe(4) // repoId + 3 extensions
  })

  // Parameterization (P0)
  it('uses parameterized queries (SQL injection safe)', () => {
    const args: any[] = [1]
    const filters = { file_type: "ts'; DROP TABLE files; --" }
    const clauses = buildFilterClauses(filters, 'all', args)

    // Should NOT contain the malicious string directly in SQL
    expect(clauses).not.toContain("DROP TABLE")
    // Should use parameter placeholders
    expect(clauses).toContain("$")
  })

  // Empty filter (P0)
  it('handles empty file_type gracefully', () => {
    const args: any[] = [1]
    const filters = { file_type: '' }
    const clausesBefore = args.length
    const clauses = buildFilterClauses(filters, 'all', args)

    // No filter added, args unchanged
    expect(args.length).toBe(clausesBefore)
    expect(clauses).not.toContain('f.relpath LIKE')
  })

  // Filter combination (P1)
  it('combines file_type with recency_threshold', () => {
    const args: any[] = [1]
    const filters = {
      file_type: 'ts',
      recency_threshold: '7 days'
    }
    const clauses = buildFilterClauses(filters, 'all', args)

    expect(clauses).toContain('f.relpath LIKE')
    expect(clauses).toContain('f.last_modified >')
    expect(args.length).toBe(3) // repoId, file_type, recency
  })

  it('combines file_type with worktree_id', () => {
    const args: any[] = [1]
    const filters = {
      file_type: 'ts',
      worktree_id: 42
    }
    const clauses = buildFilterClauses(filters, 'all', args)

    expect(clauses).toContain('f.relpath LIKE')
    expect(clauses).toContain('worktree_id')
  })

  // Legacy filter coexistence (P1)
  it('works with legacy filter parameter', () => {
    const args: any[] = [1]
    const filters = { file_type: 'ts' }
    const clauses = buildFilterClauses(filters, 'code', args)

    // Both filters applied
    expect(clauses).toContain('f.relpath LIKE') // file_type
    expect(clauses).toContain("NOT LIKE '%.md'") // legacy "code" filter
  })

  // Case handling (P0)
  it('normalizes case before SQL generation', () => {
    const args: any[] = [1]
    const filters = { file_type: 'TS,TSX' }
    const clauses = buildFilterClauses(filters, 'all', args)

    expect(args).toContain('%.ts') // Lowercased
    expect(args).toContain('%.tsx')
    expect(args).not.toContain('%.TS') // Not uppercase
  })

  // Limit enforcement (P1)
  it('rejects >20 extensions with error', () => {
    const args: any[] = [1]
    const twentyOne = Array(21).fill('ts').join(',')
    const filters = { file_type: twentyOne }

    // Should throw or return error indicator
    // (Implementation detail - could be exception or error object)
    expect(() => buildFilterClauses(filters, 'all', args)).toThrow(/too many/i)
    // OR
    const clauses = buildFilterClauses(filters, 'all', args)
    expect(clauses).toContain('ERROR') // or some error marker
  })

  // Whitespace/dot handling (P1)
  it('handles dots and whitespace before SQL', () => {
    const args: any[] = [1]
    const filters = { file_type: '  .ts , .tsx  ' }
    const clauses = buildFilterClauses(filters, 'all', args)

    expect(args).toContain('%.ts')
    expect(args).toContain('%.tsx')
    expect(args).not.toContain('%..ts') // No double dot
  })
})
```

**Why these tests:**
- Verify SQL generated correctly (most critical for correctness)
- Ensure parameterization (security)
- Test filter combinations (real-world usage)
- Catch SQL injection attempts

**Time to run:** <2 seconds

---

### 3. E2E Tests: Real Database Queries (5 tests)

**Location:** `packages/maproom-mcp/tests/search_tool.integration.test.ts` (new file)

**Setup:**
```typescript
import { describe, it, expect, beforeAll, afterAll } from 'vitest'
import { Client } from 'pg'

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
    INSERT INTO maproom.files (relpath, worktree_id, ...)
    VALUES
      ('src/auth.ts', 1, ...),
      ('src/auth.spec.ts', 1, ...),
      ('src/api.tsx', 1, ...),
      ('src/utils.js', 1, ...),
      ('README.md', 1, ...),
      ('package.json', 1, ...)
  `)
})

afterAll(async () => {
  await testDb.query('TRUNCATE maproom.files CASCADE')
  await testDb.end()
})
```

**Coverage:**

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
    expect(lower.hits.map(h => h.chunk_id)).toEqual(
      upper.hits.map(h => h.chunk_id)
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

**Why these tests:**
- Verify feature works end-to-end with real database
- Catch issues unit/integration tests miss (data types, null handling, etc.)
- Performance regression detection

**Time to run:** <2 seconds (with test database)

---

## Risk-Based Testing

### High Risk (Must test)

**SQL injection:**
- Integration test with malicious input
- Verify parameterized queries used

**Multi-extension logic:**
- E2E test verifies OR clause works
- Integration test checks SQL structure

**Case sensitivity:**
- Unit test validates normalization
- E2E test confirms database behavior

### Medium Risk (Should test)

**Filter combination:**
- Integration test with multiple filters
- Ensures no conflicts between filters

**Performance degradation:**
- E2E performance test with timer
- Acceptable if <200ms total

### Low Risk (Can skip for MVP)

**Theoretical edge cases:**
- 1000-character extension name
- Unicode characters in extension
- Binary file extensions

**Rationale:** Users won't hit these. Add later if bugs reported.

---

## Testing Workflow

### Development TDD Cycle

```
1. Write failing unit test
   ↓
2. Implement parseFileTypeFilter
   ↓
3. Unit tests pass
   ↓
4. Write failing integration test
   ↓
5. Implement buildFilterClauses
   ↓
6. Integration tests pass
   ↓
7. Write failing E2E test
   ↓
8. Verify end-to-end flow
   ↓
9. All tests pass → Ship
```

**Time per cycle:** ~30 minutes

---

### CI Pipeline

```yaml
# .github/workflows/test-file-type-filter.yml
name: File Type Filter Tests

on: [push, pull_request]

jobs:
  test:
    runs-on: ubuntu-latest
    services:
      postgres:
        image: ankane/pgvector:latest
        env:
          POSTGRES_PASSWORD: maproom
        options: >-
          --health-cmd pg_isready
          --health-interval 10s
    steps:
      - name: Unit tests
        run: pnpm test:unit -- --grep "parseFileTypeFilter"

      - name: Integration tests
        run: pnpm test:integration -- --grep "buildFilterClauses"

      - name: E2E tests
        run: pnpm test:e2e -- --grep "File Type Filter"
        env:
          TEST_DATABASE_URL: postgresql://postgres:maproom@localhost:5432/maproom_test
```

**Total CI time:** <10 seconds

---

## Acceptance Criteria

### Definition of Done

**Feature is complete when:**

✅ **All tests pass:**
- 15 unit tests (parseFileTypeFilter)
- 10 integration tests (buildFilterClauses)
- 5 E2E tests (handleSearch)

✅ **No regressions:**
- Existing search tests still pass
- Other filters (recency_threshold, repo_id) unaffected

✅ **Performance acceptable:**
- E2E test completes in <200ms
- No significant overhead vs non-filtered search

✅ **Documentation updated:**
- Tool description includes file_type examples
- Error messages are helpful

✅ **Code review passed:**
- No obvious bugs
- Follows existing code style
- Parameterized queries used

---

### Test Coverage Goals

**Not chasing 100%** - targeting meaningful coverage:

```
parseFileTypeFilter:    100% (pure function, easy to test)
buildFilterClauses:     85%  (core logic + edge cases)
handleSearch:           70%  (integration point, many paths)
```

**Overall project:** ~80% coverage (confidence without ceremony)

---

## Testing Tools

### Vitest Configuration

```typescript
// vitest.config.ts
export default {
  test: {
    include: ['tests/**/*.test.ts'],
    coverage: {
      provider: 'v8',
      include: ['src/index.ts'],
      exclude: ['tests/**'],
      thresholds: {
        lines: 80,
        functions: 80,
        branches: 75,
      }
    },
    setupFiles: ['./tests/setup.ts']
  }
}
```

### Database Setup

```typescript
// tests/setup.ts
import { Client } from 'pg'

export async function setupTestDb() {
  const client = new Client({
    connectionString: 'postgresql://maproom:maproom@localhost:5432/maproom_test'
  })
  await client.connect()

  // Run migrations
  await client.query(`
    CREATE SCHEMA IF NOT EXISTS maproom;
    -- ... migration SQL ...
  `)

  return client
}
```

---

## Known Limitations (Accepted for MVP)

### What We're NOT Testing (On Purpose)

**1. Database performance at scale**
- Not testing with 1M+ files
- Acceptable - optimize if users report slowness
- Can add benchmark test later

**2. Concurrent request handling**
- Not testing race conditions
- Acceptable - stateless filter function, no shared state

**3. All possible file extensions**
- Not testing 1000+ extension types
- Acceptable - alphanumeric validation covers normal cases

**4. Unicode/International extensions**
- Not testing `.файл`, `.文件`
- Acceptable - rare, can add if requested

**5. Performance under database load**
- Not testing filter during DB maintenance/vacuum
- Acceptable - infrastructure concern, not feature concern

---

## Monitoring & Observability

### Production Metrics (Future)

**Once shipped, monitor:**

```typescript
// Hypothetical metrics
metrics.increment('search.filter.file_type.used')
metrics.histogram('search.filter.file_type.extension_count', extensions.length)
metrics.timing('search.filter.file_type.parse_duration', duration)
metrics.increment('search.filter.file_type.empty_input')
metrics.increment('search.filter.file_type.over_limit')
```

**Use cases:**
- See if users actually use the feature
- Detect pathological inputs (too many extensions)
- Identify performance bottlenecks

---

## Bug Triage Strategy

### If Bugs Emerge Post-Launch

**Severity levels:**

**P0 (Immediate fix):**
- SQL injection vulnerability
- Wrong results returned (returns .md when filtering to .ts)
- Crashes MCP server

**P1 (Fix in next sprint):**
- Edge case not handled (e.g., extension with numbers fails)
- Performance degradation (>2x slower than baseline)

**P2 (Backlog):**
- Feature request (support regex)
- UX improvement (better error messages)

**Add test for each P0/P1 bug before fixing** (prevent regression).

---

## Conclusion

This quality strategy is **pragmatic and focused**:

- **30 tests** covering critical paths (not 300 tests covering theory)
- **<5 second** test suite (fast feedback loop)
- **80% coverage** (confidence without ceremony)
- **E2E verification** (feature actually works, not just "tests pass")

The strategy ensures the feature ships with **high confidence** but doesn't gold-plate testing for theoretical cases that won't happen. If edge cases emerge in production, we add tests and fix - but we don't block launch on hypotheticals.

**Goal:** Ship a working, tested feature in 1-2 days, not a perfectly tested feature in 2 weeks.
