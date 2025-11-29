# Existing Test Infrastructure: Maproom MCP

**Purpose:** Document available test infrastructure to prevent reinvention and enable faster test implementation.

**Last Updated:** 2025-11-18

---

## Overview

The maproom-mcp package already has comprehensive test infrastructure including database helpers, fixtures, and established test patterns. **Use these instead of creating new infrastructure.**

---

## Database Test Helpers

**Location:** `packages/maproom-mcp/tests/helpers/database.ts` (285 lines)

This file provides complete database setup, teardown, and data creation utilities.

### Core Setup/Teardown Functions

```typescript
// Create and configure test database connection
export async function setupTestDatabase(): Promise<Client>

// Clean up test database and close connection
export async function teardownTestDatabase(client: Client): Promise<void>

// Get test database URL from environment
export function getTestDatabaseUrl(): string
```

### Data Creation Utilities

```typescript
// Create test repository
export async function createTestRepo(
  client: Client,
  name: string
): Promise<number>

// Create test worktree
export async function createTestWorktree(
  client: Client,
  repoId: number,
  name: string,
  absPath: string
): Promise<number>

// Create test file entry
export async function createTestFile(
  client: Client,
  worktreeId: number,
  relpath: string
): Promise<number>

// Create test chunk (code snippet)
export async function createTestChunk(
  client: Client,
  fileId: number,
  content: string,
  startLine: number,
  endLine: number
): Promise<number>
```

### Advanced Utilities

```typescript
// Index files from fixtures directory
export async function indexTestFixtures(
  fixturesPath: string,
  repo: string,
  worktree: string
): Promise<void>

// Clean up test data by repo name
export async function cleanupTestRepo(
  client: Client,
  repoName: string
): Promise<void>
```

### Usage Example

```typescript
import {
  setupTestDatabase,
  teardownTestDatabase,
  createTestRepo,
  createTestWorktree,
  createTestFile
} from '../helpers/database.js'

describe('Open Tool E2E Tests', () => {
  let client: Client

  beforeAll(async () => {
    client = await setupTestDatabase()
  })

  afterAll(async () => {
    await teardownTestDatabase(client)
  })

  it('should open file from database', async () => {
    // Create test data
    const repoId = await createTestRepo(client, 'test-repo')
    const worktreeId = await createTestWorktree(
      client,
      repoId,
      'main',
      '/workspace/test'
    )
    await createTestFile(client, worktreeId, 'src/index.ts')

    // Test your code
    const result = await openTool({
      relpath: 'src/index.ts',
      worktree: 'main'
    })

    expect(result.content).toBeDefined()
  })
})
```

---

## Test Fixtures

**Location:** `packages/maproom-mcp/tests/fixtures/`

### Available Fixtures

**`sample-repo/`** - Complete TypeScript repository for testing
```
tests/fixtures/sample-repo/
├── src/
│   ├── index.ts          # Main entry point
│   ├── utils.ts          # Utility functions
│   └── types.ts          # Type definitions
├── tests/
│   └── index.test.ts     # Unit tests
└── README.md             # Documentation
```

### Using Fixtures

```typescript
import { indexTestFixtures } from '../helpers/database.js'
import path from 'path'

it('should index and search fixtures', async () => {
  const fixturesPath = path.join(process.cwd(), 'tests/fixtures/sample-repo')

  // Index the fixtures
  await indexTestFixtures(fixturesPath, 'sample-repo', 'main')

  // Now search/open works with real data
  const searchResults = await searchTool({ query: 'export' })
  expect(searchResults.length).toBeGreaterThan(0)

  const openResult = await openTool({
    relpath: 'src/index.ts',
    worktree: 'main'
  })
  expect(openResult.content).toContain('export')
})
```

---

## Validation Utilities

**Location:** `packages/maproom-mcp/src/utils/validation.ts` (179 lines)

### Available Functions

```typescript
// Validate and normalize relative paths
export function validatePath(relpath: string): string

// Ensure path is within repository boundaries
export function validateWithinRepo(
  resolvedPath: string,
  repoRoot: string
): void

// Validate file size limits
export async function validateFileSize(
  filePath: string,
  maxSize: number
): Promise<void>

// Parse line range strings
export function parseRange(range?: string): { start?: number; end?: number }
```

### What's NOT Available (Need to Add)

**`fileExists()` function** - NEW utility needed for this project:

```typescript
// TO BE ADDED in validation.ts
export async function fileExists(filePath: string): Promise<boolean> {
  try {
    await fs.access(filePath, fs.constants.R_OK)
    return true
  } catch {
    return false
  }
}
```

---

## Test Patterns and Examples

### Pattern 1: E2E Test with Database

**Reference:** See `tests/tools/search.int.test.ts` for examples

```typescript
describe('Tool E2E Tests', () => {
  let client: Client

  beforeAll(async () => {
    client = await setupTestDatabase()
  })

  afterAll(async () => {
    await teardownTestDatabase(client)
  })

  it('complete workflow test', async () => {
    // Setup: Create test data
    const repoId = await createTestRepo(client, 'test')
    const worktreeId = await createTestWorktree(client, repoId, 'main', '/workspace/test')
    await createTestFile(client, worktreeId, 'test.ts')

    // Action: Call tool
    const result = await myTool({ /* params */ })

    // Verify: Check results
    expect(result).toBeDefined()
  })
})
```

### Pattern 2: Polluted Database Test

**Use case:** Testing multi-candidate fallback behavior

```typescript
it('handles database pollution gracefully', async () => {
  const repoId = await createTestRepo(client, 'test')

  // Create MULTIPLE worktrees with same name but different paths
  const worktree1 = await createTestWorktree(
    client, repoId, 'main', '/wrong/path'
  )
  const worktree2 = await createTestWorktree(
    client, repoId, 'main', '/workspace/test'  // Correct path
  )

  // Both have same file
  await createTestFile(client, worktree1, 'src/main.ts')
  await createTestFile(client, worktree2, 'src/main.ts')

  // Tool should fall back to correct path
  const result = await openTool({
    relpath: 'src/main.ts',
    worktree: 'main'
  })

  expect(result.content).toBeDefined()  // Should work despite pollution
})
```

### Pattern 3: Security Test

**Reference:** See `tests/utils/validation.test.ts` for security test examples

```typescript
describe('Security Tests', () => {
  it('rejects path traversal attempts', async () => {
    await expect(
      openTool({
        relpath: '../../../etc/passwd',
        worktree: 'main'
      })
    ).rejects.toThrow('Path traversal detected')
  })

  it('rejects absolute paths', async () => {
    await expect(
      openTool({
        relpath: '/etc/passwd',
        worktree: 'main'
      })
    ).rejects.toThrow('Absolute paths not allowed')
  })
})
```

---

## Test File Organization

```
packages/maproom-mcp/tests/
├── helpers/              # Reusable utilities
│   ├── database.ts       # Database setup/teardown
│   └── fixtures.ts       # Fixture loading helpers
├── fixtures/             # Sample data for tests
│   └── sample-repo/      # TypeScript sample repository
├── tools/                # Tool-specific tests
│   ├── search.test.ts    # Unit tests (mocked)
│   ├── search.int.test.ts # Integration tests (real DB)
│   ├── open.test.ts      # Unit tests
│   └── open.int.test.ts  # Integration tests (SOME SKIPPED!)
└── utils/                # Utility tests
    └── validation.test.ts # Validation function tests
```

---

## Environment Configuration

**Required Environment Variables:**

```bash
# Test database connection
TEST_DATABASE_URL=postgresql://maproom:maproom@localhost:5432/maproom_test

# Or individual components
PGHOST=localhost
PGPORT=5432
PGUSER=maproom
PGPASSWORD=maproom
PGDATABASE=maproom_test
```

**Test Database Setup:**

The test database is automatically configured via `setupTestDatabase()`. No manual schema creation needed - the helper handles it.

---

## Common Mistakes to Avoid

### ❌ DON'T: Create New Database Helpers

```typescript
// DON'T DO THIS - database.ts already exists!
async function mySetupDatabase() {
  const client = new Client({ /* config */ })
  await client.connect()
  // ... more setup
  return client
}
```

### ✅ DO: Import Existing Helpers

```typescript
// DO THIS INSTEAD
import { setupTestDatabase } from '../helpers/database.js'

const client = await setupTestDatabase()
```

---

### ❌ DON'T: Create Test Fixtures Manually

```typescript
// DON'T DO THIS - fixtures already exist!
const testFiles = {
  'index.ts': 'export const x = 1',
  'utils.ts': 'export function helper() {}'
}

for (const [name, content] of Object.entries(testFiles)) {
  await fs.writeFile(`/tmp/test/${name}`, content)
}
```

### ✅ DO: Use Existing Fixtures

```typescript
// DO THIS INSTEAD
import { indexTestFixtures } from '../helpers/database.js'

await indexTestFixtures(
  'tests/fixtures/sample-repo',
  'test-repo',
  'main'
)
```

---

### ❌ DON'T: Implement Test Setup/Teardown From Scratch

```typescript
// DON'T DO THIS
beforeAll(async () => {
  client = new Client({ /* manual config */ })
  await client.connect()
  await client.query('CREATE SCHEMA IF NOT EXISTS maproom')
  // ... lots of setup code
})

afterAll(async () => {
  await client.query('DROP SCHEMA maproom CASCADE')
  await client.end()
  // ... lots of teardown code
})
```

### ✅ DO: Use Helper Functions

```typescript
// DO THIS INSTEAD
let client: Client

beforeAll(async () => {
  client = await setupTestDatabase()  // Handles everything
})

afterAll(async () => {
  await teardownTestDatabase(client)  // Clean exit
})
```

---

## Quick Reference: What to Reuse

| Need | Use This | Don't Create |
|------|----------|--------------|
| Database connection | `setupTestDatabase()` | New Client setup |
| Teardown | `teardownTestDatabase()` | Custom cleanup |
| Test repository | `createTestRepo()` | Manual INSERT |
| Test worktree | `createTestWorktree()` | Manual INSERT |
| Test file entry | `createTestFile()` | Manual INSERT |
| Sample code files | `tests/fixtures/sample-repo/` | New test files |
| Index fixtures | `indexTestFixtures()` | Custom indexing |
| Path validation | `validatePath()` from validation.ts | New validators |
| Security checks | `validateWithinRepo()` from validation.ts | New security code |

---

## Adding New Helpers (If Truly Needed)

**Before adding new helpers, ask:**
1. Does `database.ts` already provide this?
2. Could I use `indexTestFixtures()` instead?
3. Is this truly reusable or just for one test?

**If adding new helper:**
1. Add to existing files (`database.ts` or `validation.ts`)
2. Export for reuse
3. Document in this file
4. Add JSDoc comments
5. Write unit tests for the helper itself

**Example:**

```typescript
// Add to tests/helpers/database.ts

/**
 * Create multiple worktrees with the same name (for pollution testing)
 */
export async function createDuplicateWorktrees(
  client: Client,
  repoId: number,
  name: string,
  paths: string[]
): Promise<number[]> {
  const ids: number[] = []
  for (const absPath of paths) {
    const id = await createTestWorktree(client, repoId, name, absPath)
    ids.push(id)
  }
  return ids
}
```

---

## Project-Specific Additions Needed

For the OPNFIX project, we only need to ADD:

1. **`fileExists()` in validation.ts** - Filesystem existence check
2. **E2E test file: `open.e2e.test.ts`** - New test file using existing helpers
3. **Security test file: `open.security.test.ts`** - New test file using existing helpers

**Everything else already exists** - use it!

---

## Summary

**Existing Infrastructure:**
- ✅ Database helpers (setup, teardown, data creation)
- ✅ Test fixtures (sample TypeScript repository)
- ✅ Validation utilities (path, security, size checks)
- ✅ Test patterns (E2E, integration, security)

**What to Add (Only):**
- ⚠️ `fileExists()` helper function
- ⚠️ New E2E test files (using existing infrastructure)
- ⚠️ Un-skip existing integration tests

**Time Savings:**
- Using existing infrastructure saves **4-6 hours** of duplicate implementation
- Reduces project timeline from 3-5 days to 2-3 days

---

## Questions?

**"Where do I find the database helpers?"**
→ `packages/maproom-mcp/tests/helpers/database.ts`

**"How do I use the fixtures?"**
→ `await indexTestFixtures('tests/fixtures/sample-repo', 'repo', 'worktree')`

**"Can I create my own test setup?"**
→ No! Use `setupTestDatabase()` instead

**"What if I need a helper that doesn't exist?"**
→ Add it to the existing files, don't create new infrastructure files

---

**Last Updated:** 2025-11-18
**Maintainer:** Project planning / architecture team
**Review Schedule:** Update when new helpers added to codebase
