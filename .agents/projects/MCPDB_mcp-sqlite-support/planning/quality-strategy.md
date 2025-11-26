# MCPDB Quality Strategy - MCP Server SQLite Support

## Testing Philosophy

This project focuses on **integration confidence** rather than coverage metrics. The goal is to verify that MCP tools work correctly with SQLite backends without breaking existing PostgreSQL functionality.

## Test Categories

### 1. Unit Tests - URL Parsing

**Purpose**: Verify SQLite URL parsing handles all edge cases

**Test File**: `tests/unit/resolve-database.test.ts`

**Cases**:
```typescript
describe('resolveDatabaseConfig', () => {
  // SQLite URL parsing
  test('parses absolute sqlite:// URL')
  test('parses relative sqlite:// URL')
  test('expands ~ in sqlite:// path')
  test('normalizes path separators on Windows')

  // PostgreSQL URL handling
  test('returns postgresql type for postgres:// URL')
  test('returns postgresql type for postgresql:// URL')

  // Auto-detection
  test('detects SQLite when ~/.maproom/maproom.db exists')
  test('falls back to PostgreSQL when SQLite not found')
  test('uses DevContainer PostgreSQL when IN_DEVCONTAINER=true')

  // Priority
  test('explicit URL takes precedence over auto-detection')
  test('DevContainer takes precedence over SQLite default')
})

describe('isSqliteUrl', () => {
  test('returns true for sqlite:// URLs')
  test('returns false for postgresql:// URLs')
  test('returns false for empty string')
})
```

**Confidence Target**: 100% coverage of URL parsing logic

### 2. Integration Tests - MCP Tools

**Purpose**: Verify MCP tools work with SQLite backend

**Test File**: `tests/integration/sqlite-backend.test.ts`

**Prerequisites**:
- Pre-indexed SQLite fixture from `crates/maproom/tests/fixtures/pre-indexed-maproom.db`
- Copy fixture to temp location before tests

**Cases**:
```typescript
describe('MCP Tools with SQLite Backend', () => {
  beforeAll(() => {
    // Set MAPROOM_DATABASE_URL to SQLite fixture
    process.env.MAPROOM_DATABASE_URL = `sqlite://${FIXTURE_PATH}`
  })

  test('status tool returns repo information')
  test('search tool returns FTS results')
  test('open tool retrieves code content')
  test('search with empty results returns helpful hint')
  test('status shows SQLite backend type')
})
```

**Confidence Target**: Core tools work end-to-end

### 3. Backward Compatibility Tests

**Purpose**: Ensure PostgreSQL functionality unchanged

**Test File**: Existing test suite (run without SQLite changes)

**Strategy**:
```bash
# Run existing tests with PostgreSQL (no changes)
TEST_DATABASE_URL=postgresql://... pnpm test

# Run new SQLite tests
MAPROOM_DATABASE_URL=sqlite://... pnpm test:sqlite
```

**Confidence Target**: Zero regressions in PostgreSQL path

### 4. Error Handling Tests

**Purpose**: Verify helpful error messages for common issues

**Test File**: `tests/unit/error-messages.test.ts`

**Cases**:
```typescript
describe('SQLite Error Messages', () => {
  test('missing SQLite file provides create instructions')
  test('invalid SQLite URL format provides example')
  test('permission denied includes path in message')
})
```

## Test Infrastructure Changes

### Test Isolation Strategy

**Approach**: Separate test files, NOT abstracted helpers

SQLite tests use:
- **Separate helper file**: `tests/helpers/sqlite.ts` (NEW)
- **Separate test files**: `tests/integration/sqlite-*.test.ts`
- **Pre-indexed fixture**: `crates/maproom/tests/fixtures/pre-indexed-maproom.db`

PostgreSQL tests use:
- **Existing helpers**: `tests/helpers/database.ts` (UNCHANGED)
- **Existing test files**: All other `*.test.ts` files
- **PostgreSQL service container**: Via CI or local Docker

**Rationale**:
- No complex abstraction layer
- Each backend tested independently
- No risk of PostgreSQL helper changes breaking SQLite tests
- Pre-indexed fixture ensures consistent test data

### New Test Helper: `helpers/sqlite.ts`

```typescript
/**
 * SQLite test utilities (SEPARATE from database.ts)
 * Does NOT modify or interact with PostgreSQL helpers
 */
import { copyFileSync, unlinkSync, existsSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join, resolve } from 'node:path'

// Relative path from packages/maproom-mcp to fixture
const FIXTURE_SOURCE = resolve(__dirname, '../../../../crates/maproom/tests/fixtures/pre-indexed-maproom.db')

export function createTestSqliteDatabase(): string {
  if (!existsSync(FIXTURE_SOURCE)) {
    throw new Error(`SQLite fixture not found: ${FIXTURE_SOURCE}\nRun: cargo test --features sqlite --test create_sqlite_fixture -- --ignored`)
  }
  const testDbPath = join(tmpdir(), `maproom-test-${Date.now()}.db`)
  copyFileSync(FIXTURE_SOURCE, testDbPath)
  return testDbPath
}

export function cleanupTestSqliteDatabase(path: string): void {
  try {
    unlinkSync(path)
  } catch {
    // Ignore cleanup errors
  }
}

export function getSqliteFixturePath(): string {
  return FIXTURE_SOURCE
}
```

### Test Configuration: `vitest.config.ts`

No changes needed. SQLite tests run in same environment, just with different `MAPROOM_DATABASE_URL`.

## CI Integration

### GitHub Actions Update

**Note on CI Job Naming**:
- `test-sqlite-e2e` (EXISTING) - Tests Rust CLI with SQLite backend
- `test-mcp-sqlite` (NEW) - Tests TypeScript MCP server with SQLite backend

These test different layers and both are needed.

```yaml
# Add SQLite test job to .github/workflows/test.yml
# NOTE: This is DIFFERENT from test-sqlite-e2e which tests Rust CLI
test-mcp-sqlite:
  name: MCP SQLite Tests (TypeScript)
  runs-on: ubuntu-latest

  steps:
    - name: Checkout code
      uses: actions/checkout@v4

    - name: Setup Node.js
      uses: actions/setup-node@v4
      with:
        node-version: '20'

    - name: Setup pnpm
      uses: pnpm/action-setup@v4

    - name: Install dependencies
      run: pnpm install --frozen-lockfile

    - name: Setup Rust (for fixture generation)
      uses: actions-rust-lang/setup-rust-toolchain@v1

    - name: Ensure SQLite fixture exists
      run: |
        if [ ! -f crates/maproom/tests/fixtures/pre-indexed-maproom.db ]; then
          cargo test --features sqlite --test create_sqlite_fixture -- --ignored
        fi

    - name: Run MCP SQLite tests
      working-directory: packages/maproom-mcp
      run: pnpm test:sqlite
      env:
        MAPROOM_DATABASE_URL: sqlite://${{ github.workspace }}/crates/maproom/tests/fixtures/pre-indexed-maproom.db
```

## Critical Paths

### Path 1: SQLite URL Resolution

```
Environment Variable → URL Parser → DatabaseConfig → Daemon Env
```

**Tests**:
1. Parse various URL formats
2. Verify config type detection
3. Confirm daemon receives correct URL

### Path 2: MCP Tool Execution

```
MCP Request → Tool Handler → Daemon Client → Rust Binary → SQLite → Response
```

**Tests**:
1. status tool returns data
2. search tool finds chunks
3. open tool retrieves content

### Path 3: Error Propagation

```
Missing File → Daemon Error → MCP Error Response → User-Friendly Message
```

**Tests**:
1. Missing file produces helpful message
2. Invalid URL produces format example
3. Connection errors include troubleshooting steps

## Acceptance Criteria

### Must Pass (MVP)

- [ ] URL parser correctly identifies SQLite vs PostgreSQL
- [ ] `status` tool works with SQLite backend
- [ ] `search` tool returns FTS results from SQLite
- [ ] `open` tool retrieves code from SQLite-indexed files
- [ ] Existing PostgreSQL tests continue to pass
- [ ] CI runs SQLite tests without PostgreSQL service

### Should Pass (Quality)

- [ ] Error messages guide users to fix issues
- [ ] Auto-detection finds `~/.maproom/maproom.db`
- [ ] Test suite runs in <30 seconds with SQLite
- [ ] No new npm dependencies added

### Nice to Have

- [ ] Debug mode shows SQLite vs PostgreSQL detection
- [ ] Status tool indicates backend type
- [ ] Performance baseline documented

## Test Matrix

| Scenario | SQLite | PostgreSQL | Expected |
|----------|--------|------------|----------|
| Explicit sqlite:// URL | ✓ | - | SQLite backend |
| Explicit postgresql:// URL | - | ✓ | PostgreSQL backend |
| IN_DEVCONTAINER=true | - | ✓ | PostgreSQL backend |
| ~/.maproom/maproom.db exists | ✓ | - | SQLite backend |
| No config, no default file | - | ✓ | PostgreSQL fallback |
| Invalid URL format | ✓ | ✓ | Clear error message |

## Risk Mitigation

### Risk: Breaking PostgreSQL Tests

**Mitigation**: Run full PostgreSQL test suite as separate CI job before merge

### Risk: SQLite Fixture Staleness

**Mitigation**: Regenerate fixture in CI if missing; document fixture creation

### Risk: Platform-Specific Path Issues

**Mitigation**: Test path handling on Linux (CI) and document Windows limitations

## Definition of Done

1. All unit tests for URL parsing pass
2. Integration tests with SQLite fixture pass
3. Existing PostgreSQL tests pass (zero regressions)
4. CI pipeline updated with SQLite test job
5. Test documentation updated in `docs/testing/`
