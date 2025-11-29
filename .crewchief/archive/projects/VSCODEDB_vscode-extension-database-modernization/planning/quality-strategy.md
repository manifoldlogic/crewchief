# VSCODEDB - Quality Strategy

## Test Philosophy

This project follows a pragmatic testing approach focused on confidence over coverage. The goal is to prevent rework and catch regressions, not achieve ceremonial metrics.

## Critical Paths

The following paths represent core functionality that must be tested:

### 1. Database Configuration Resolution

**What to Test**:
- `resolveDatabaseConfig()` returns correct type for each provider setting
- SQLite path expansion (~, relative paths)
- PostgreSQL URL construction from settings
- Default values when settings are empty

**Why Critical**:
- Wrong configuration breaks entire extension
- Path expansion bugs cause "file not found" errors
- URL construction bugs cause connection failures

**Test Type**: Unit tests

### 2. Database Availability Check

**What to Test**:
- SQLite mode: `existsSync()` integration
- PostgreSQL mode: TCP socket connection
- Error message generation for both modes

**Why Critical**:
- False positives cause silent failures
- False negatives block users from valid databases
- Poor error messages cause support burden

**Test Type**: Unit tests with file system mocking

### 3. Conditional Docker Activation

**What to Test**:
- Docker not started when SQLite mode
- Docker started when PostgreSQL mode
- No regression in existing PostgreSQL flow

**Why Critical**:
- Starting Docker for SQLite users defeats the project purpose
- Not starting Docker for PostgreSQL users breaks their setup

**Test Type**: Integration tests with mock Docker manager

### 4. Extension Activation Flow

**What to Test**:
- Full activation succeeds with SQLite mode
- Full activation succeeds with PostgreSQL mode
- Activation time <500ms for SQLite mode
- Error handling for missing database

**Why Critical**:
- Activation failures make extension unusable
- Slow activation degrades VSCode startup experience

**Test Type**: Integration tests

## Test Inventory

### Existing Tests to Preserve

```
src/
├── extension.test.ts                    # Extension lifecycle
├── services/postgres-checker.ts         # No tests (add in this project)
├── docker/manager.test.ts               # Docker manager
├── process/orchestrator.test.ts         # Process orchestration
├── process/recovery.test.ts             # Process recovery
├── ui/statusBar.test.ts                 # Status bar UI
└── ui/setupWizard.test.ts               # Setup wizard
```

All existing tests must continue to pass.

### New Tests to Add

#### Unit Tests

**`services/database-checker.test.ts`**:
```typescript
describe('resolveDatabaseConfig', () => {
  it('returns sqlite config when provider is sqlite', () => {})
  it('returns postgresql config when provider is postgres', () => {})
  it('expands tilde in sqlite path', () => {})
  it('uses default path when sqlitePath is empty', () => {})
  it('uses custom path when sqlitePath is set', () => {})
})

describe('checkDatabaseAvailable', () => {
  it('returns true when sqlite file exists', () => {})
  it('returns false when sqlite file missing', () => {})
  it('returns true when postgres TCP connects', () => {})
  it('returns false when postgres TCP fails', () => {})
})

describe('getDatabaseUnavailableMessage', () => {
  it('returns sqlite message with file path', () => {})
  it('returns postgres message with setup instructions', () => {})
})
```

**`extension.test.ts` additions**:
```typescript
describe('initializeServices', () => {
  it('skips Docker for sqlite mode', () => {})
  it('starts Docker for postgres mode', () => {})
})

describe('activation performance', () => {
  it('activates in <500ms for sqlite mode', () => {})
})
```

#### Integration Tests

**`test/integration.test.ts` additions**:
```typescript
describe('SQLite mode E2E', () => {
  it('activates with existing sqlite database', () => {})
  it('shows error for missing sqlite database', () => {})
  it('search command works with sqlite', () => {})
})
```

## Testing Approach by Ticket

### VSCODEDB-1001: database-checker.ts

**Unit Tests Required**:
- All `resolveDatabaseConfig()` scenarios
- All `checkDatabaseAvailable()` scenarios
- All error message generation

**Mocking**:
- `vscode.workspace.getConfiguration()` for settings
- `existsSync()` for file checks
- `createConnection()` for TCP checks

### VSCODEDB-1002: Extension Settings Schema

**Validation Tests**:
- Schema parses correctly (manual VSIX validation)
- Default values work as expected
- Enum constraints enforced

**No automated tests** - schema validation is done by VSCode.

### VSCODEDB-1003: Docker Optional

**Integration Tests Required**:
- Mock Docker manager
- Verify not called in SQLite mode
- Verify called in PostgreSQL mode

### VSCODEDB-1004: Activation Flow

**Integration Tests Required**:
- Full activation with SQLite database
- Full activation with PostgreSQL (existing tests)
- Performance measurement

### VSCODEDB-1005: Documentation

**No automated tests** - manual review only.

## Risk-Based Testing

### High Risk (Comprehensive Testing)

| Component | Risk | Test Coverage |
|-----------|------|---------------|
| `resolveDatabaseConfig()` | Wrong config breaks extension | 100% branch coverage |
| `checkDatabaseAvailable()` | False results confuse users | Both backends tested |
| Docker conditional | SQLite users hit Docker errors | Mock verification |

### Medium Risk (Targeted Testing)

| Component | Risk | Test Coverage |
|-----------|------|---------------|
| Error messages | Poor UX | Spot checks |
| Settings schema | Invalid defaults | Schema validation |
| ProcessOrchestrator URL | Connection failures | Existing tests |

### Low Risk (Manual Testing)

| Component | Risk | Test Coverage |
|-----------|------|---------------|
| Documentation | Outdated info | Manual review |
| Setup wizard | UX issues | Manual testing |

## Test Infrastructure

### SQLite Test Fixture Strategy

Integration tests require actual SQLite database files. Strategy:

**1. Temp Directory Pattern**:
```typescript
import { mkdtempSync, writeFileSync, rmSync } from 'node:fs'
import { join } from 'node:path'
import { tmpdir } from 'node:os'

let testDir: string

beforeEach(() => {
  testDir = mkdtempSync(join(tmpdir(), 'maproom-test-'))
})

afterEach(() => {
  rmSync(testDir, { recursive: true, force: true })
})
```

**2. Minimal SQLite Fixture**:
For most tests, an empty file is sufficient since `existsSync()` is the primary check:
```typescript
// Create empty file to simulate database
writeFileSync(join(testDir, 'test.db'), '')
```

**3. Full Schema Fixture (if needed)**:
Reference MCPDB project's test helpers for creating databases with schema:
```typescript
// packages/maproom-mcp/test/helpers/test-database.ts pattern
// Only needed for tests that actually query the database
```

**4. Home Directory Simulation**:
For tests involving `~/.maproom/maproom.db`:
```typescript
// Mock os.homedir() to return temp directory
vi.mock('node:os', async (importOriginal) => ({
  ...(await importOriginal()),
  homedir: () => testDir
}))
```

### Test Framework

The extension already uses **Vitest** with these scripts:
```json
"test": "vitest run",
"test:watch": "vitest",
"test:coverage": "vitest run --coverage"
```

No changes to test infrastructure required.

### Mocking Strategy

**VSCode API Mocking**:
```typescript
// Existing pattern in codebase
vi.mock('vscode', () => ({
  workspace: {
    getConfiguration: vi.fn(() => ({
      get: vi.fn((key) => mockSettings[key])
    }))
  }
}))
```

**File System Mocking**:
```typescript
vi.mock('node:fs', () => ({
  existsSync: vi.fn((path) => mockFiles.includes(path))
}))
```

**Network Mocking**:
```typescript
vi.mock('node:net', () => ({
  createConnection: vi.fn(() => mockSocket)
}))
```

## Acceptance Criteria Verification

### VSCODEDB-1001

| Criterion | Verification |
|-----------|-------------|
| `resolveDatabaseConfig()` returns SQLite config | Unit test |
| `checkDatabaseAvailable()` works for SQLite | Unit test |
| Error messages helpful | Manual + unit test |

### VSCODEDB-1002

| Criterion | Verification |
|-----------|-------------|
| Settings schema valid | VSIX packaging |
| Defaults work | Integration test |
| UI renders correctly | Manual |

### VSCODEDB-1003

| Criterion | Verification |
|-----------|-------------|
| Docker not started for SQLite | Integration test |
| Docker still works for PostgreSQL | Existing tests |

### VSCODEDB-1004

| Criterion | Verification |
|-----------|-------------|
| SQLite activation works | Integration test |
| PostgreSQL activation works | Existing tests |
| Activation <500ms | Performance test |

### VSCODEDB-1005

| Criterion | Verification |
|-----------|-------------|
| README updated | Manual review |
| Settings documented | Manual review |

## CI Integration

The existing CI pipeline runs:
```yaml
- name: Run tests
  working-directory: packages/vscode-maproom
  run: pnpm test
```

No changes to CI required. New tests will be picked up automatically.

## Test Execution Timeline

| Phase | Tests | When |
|-------|-------|------|
| VSCODEDB-1001 | database-checker.test.ts | After implementation |
| VSCODEDB-1002 | Schema validation | During VSIX build |
| VSCODEDB-1003 | Docker mock tests | After implementation |
| VSCODEDB-1004 | Integration tests | After all code complete |
| VSCODEDB-1005 | Manual review | Final phase |

## Quality Gates

Before marking tickets complete:

1. **All unit tests pass**: `pnpm test`
2. **No new TypeScript errors**: `pnpm compile`
3. **VSIX packages successfully**: `pnpm vsce:package`
4. **Manual smoke test**: Follow procedure below

## Manual Smoke Test Procedure

Execute after all MVP tickets (VSCODEDB-1001 through VSCODEDB-1005) are complete:

### SQLite Mode Smoke Test

```bash
# 1. Prepare clean environment
rm -f ~/.maproom/maproom.db  # Remove existing database (if any)

# 2. Build and package VSIX
cd packages/vscode-maproom
pnpm compile && pnpm vsce:package

# 3. Install VSIX in fresh VSCode window
# Open VSCode → Extensions → "..." → Install from VSIX → select .vsix file

# 4. Verify extension detects missing database
# Expected: Setup wizard runs automatically OR error message with guidance

# 5. Create test database
cd /path/to/any/repo
crewchief-maproom scan --sqlite ~/.maproom/maproom.db .

# 6. Restart VSCode or run "Developer: Reload Window"
# Expected: Extension activates without Docker prompts

# 7. Verify status bar shows SQLite mode
# Expected: Status bar shows "$(database) SQLite"

# 8. Run search command
# Expected: Search results return from SQLite database

# 9. Check activation time
# Open Output panel → "Maproom" channel
# Expected: Activation completes in <500ms
```

### PostgreSQL Mode Smoke Test (Regression)

```bash
# 1. Change settings
# Settings → Maproom → Database → Provider → "postgres"

# 2. Reload VSCode
# Expected: Extension prompts to start Docker

# 3. Start Docker services
# Follow existing PostgreSQL setup flow

# 4. Verify functionality
# Run search command
# Expected: Results return from PostgreSQL
```

### Smoke Test Checklist

| Step | SQLite | PostgreSQL |
|------|--------|------------|
| Extension activates | ☐ | ☐ |
| Correct database detected | ☐ | ☐ |
| Status bar shows mode | ☐ | ☐ |
| Search returns results | ☐ | ☐ |
| No Docker prompts (SQLite only) | ☐ | N/A |
| Docker starts (PostgreSQL only) | N/A | ☐ |
