# Ticket: VSCODEDB-1001: Create database-checker.ts

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - tests executed and passing
- [x] **Verified** - by the verify-ticket agent

## Agents
- vscode-extension-specialist (primary)
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary

Create a unified database availability checker (`database-checker.ts`) that abstracts both SQLite (file existence) and PostgreSQL (TCP connectivity) backends into a single interface. This is the foundational ticket for the VSCODEDB project.

## Background

The Maproom VSCode extension currently only supports PostgreSQL via `postgres-checker.ts`. This ticket creates a new `database-checker.ts` service that:
1. Resolves database configuration from VSCode settings
2. Checks database availability (file existence for SQLite, TCP for PostgreSQL)
3. Provides helpful error messages when databases are unavailable

**Reference:** plan.md Phase 1 - "VSCODEDB-1001: Create database-checker.ts"

**Pattern Source:** The implementation follows patterns established in MCPDB project's `packages/maproom-mcp/src/utils/resolve-database.ts`

## Acceptance Criteria

- [x] `resolveDatabaseConfig()` returns SQLite config when `maproom.database.provider = 'sqlite'`
- [x] `resolveDatabaseConfig()` returns PostgreSQL config when `maproom.database.provider = 'postgres'`
- [x] `checkDatabaseAvailable()` uses `existsSync()` for SQLite mode
- [x] `checkDatabaseAvailable()` uses TCP socket check for PostgreSQL mode (delegates to existing `checkPostgresAvailable()`)
- [x] All unit tests pass with `pnpm test -- src/services/database-checker.test.ts`
- [x] `postgres-checker.ts` has deprecation comment added

## Technical Requirements

### Interface Definition

```typescript
export interface DatabaseConfig {
  type: 'sqlite' | 'postgresql'
  url: string
  path?: string  // Only for SQLite
}

export interface PostgresConfig {
  host: string
  port: number
  user: string
  password: string
  database: string
}
```

### Core Functions

```typescript
// Resolve database configuration from VSCode settings
export function resolveDatabaseConfig(): DatabaseConfig

// Check if database is available
export async function checkDatabaseAvailable(config: DatabaseConfig): Promise<boolean>

// Get database URL string for ProcessOrchestrator
export function getDatabaseUrl(config: DatabaseConfig): string

// Get helpful error message when database unavailable
export function getDatabaseUnavailableMessage(config: DatabaseConfig): string
```

### Helper Functions

```typescript
// Expand tilde in paths
function expandPath(path: string): string

// Get PostgreSQL config from settings (reuse existing pattern)
function getPostgresConfigFromSettings(): PostgresConfig

// Build PostgreSQL URL from config
function getPostgresUrl(config: PostgresConfig): string
```

### Path Resolution

SQLite path resolution:
1. Read `maproom.database.sqlitePath` from settings
2. If empty, use default `~/.maproom/maproom.db`
3. Expand tilde (`~`) to home directory
4. Resolve relative paths to absolute

### Imports

```typescript
import { existsSync } from 'node:fs'
import { createConnection, type Socket } from 'node:net'
import * as vscode from 'vscode'
import { homedir } from 'node:os'
import { resolve, isAbsolute } from 'node:path'
```

## Implementation Notes

### Settings Access Pattern

```typescript
const config = vscode.workspace.getConfiguration('maproom.database')
const provider = config.get<string>('provider') ?? 'sqlite'
```

### SQLite URL Format

```typescript
// Format: sqlite:///absolute/path/to/file.db
const url = `sqlite://${resolvedPath}`
```

### Delegate PostgreSQL Checking

Do NOT reimplement TCP checking. Import and delegate to existing `postgres-checker.ts`:

```typescript
import { checkPostgresAvailable } from './postgres-checker'

// In checkDatabaseAvailable():
if (config.type === 'postgresql') {
  const pgConfig = getPostgresConfigFromSettings()
  return checkPostgresAvailable(pgConfig)
}
```

### Error Messages

SQLite unavailable message should include:
- File path that was checked
- Command to create an index: `crewchief-maproom scan --sqlite <path> /path/to/repo`
- Suggestion to check settings

PostgreSQL unavailable message: delegate to existing `getPostgresUnavailableMessage()` from postgres-checker.ts or reuse existing pattern.

## Dependencies

- None (this is the foundational ticket)

## Risk Assessment

- **Risk**: Path expansion bugs could cause "file not found" errors
  - **Mitigation**: Comprehensive unit tests for tilde expansion, relative paths, and absolute paths

- **Risk**: Breaking existing PostgreSQL flow
  - **Mitigation**: Delegate to existing `postgres-checker.ts` functions, don't reimplement

## Files/Packages Affected

### New Files
- `packages/vscode-maproom/src/services/database-checker.ts`
- `packages/vscode-maproom/src/services/database-checker.test.ts`

### Modified Files
- `packages/vscode-maproom/src/services/postgres-checker.ts` (add deprecation comment only)

## Test Coverage

Create `database-checker.test.ts` with the following test cases:

```typescript
describe('resolveDatabaseConfig', () => {
  it('returns sqlite config when provider is sqlite')
  it('returns postgresql config when provider is postgres')
  it('expands tilde in sqlite path')
  it('uses default path when sqlitePath is empty')
  it('uses custom path when sqlitePath is set')
  it('resolves relative paths to absolute')
})

describe('checkDatabaseAvailable', () => {
  it('returns true when sqlite file exists')
  it('returns false when sqlite file missing')
  it('delegates to postgres-checker for postgresql')
})

describe('getDatabaseUnavailableMessage', () => {
  it('returns sqlite message with file path')
  it('returns postgres message with setup instructions')
})

describe('expandPath', () => {
  it('expands ~ to home directory')
  it('returns absolute paths unchanged')
  it('handles paths without tilde')
})
```

### Mocking Requirements

```typescript
// Mock VSCode settings
vi.mock('vscode', () => ({
  workspace: {
    getConfiguration: vi.fn(() => ({
      get: vi.fn((key) => mockSettings[key])
    }))
  }
}))

// Mock file system
vi.mock('node:fs', () => ({
  existsSync: vi.fn((path) => mockFiles.includes(path))
}))

// Mock home directory
vi.mock('node:os', async (importOriginal) => ({
  ...(await importOriginal()),
  homedir: () => '/mock/home'
}))
```
