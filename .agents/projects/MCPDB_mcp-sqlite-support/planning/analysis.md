# MCPDB Analysis - MCP Server SQLite Support

## Problem Definition

The Maproom MCP server (`packages/maproom-mcp/`) currently assumes PostgreSQL as the only database backend. With the completion of VECSTORE (VectorStore trait) and MAPCLI (CLI SQLite support), the Rust daemon now supports both PostgreSQL and SQLite backends. However, the TypeScript MCP server layer has not been updated to:

1. Parse and validate `sqlite://` database URLs
2. Handle SQLite file paths in environment configuration
3. Run tests without requiring a PostgreSQL service container
4. Provide appropriate error messages for SQLite-specific scenarios

## Current State Analysis

### Existing Database Resolution (`resolve-database.ts`)

```typescript
export function resolveDatabase(): string {
  // 1. Explicit override
  if (process.env.MAPROOM_DATABASE_URL) {
    return process.env.MAPROOM_DATABASE_URL
  }

  // 2. DevContainer
  if (process.env.IN_DEVCONTAINER === 'true') {
    return 'postgresql://maproom:maproom@maproom-postgres:5432/maproom'
  }

  // 3. Default localhost
  return 'postgresql://maproom:maproom@localhost:5433/maproom'
}
```

**Issues:**
- Defaults always assume PostgreSQL
- No SQLite URL scheme recognition
- No SQLite file path validation
- No auto-detection of `~/.maproom/maproom.db`

### Daemon Integration (`daemon.ts`)

```typescript
if (!process.env.MAPROOM_DATABASE_URL) {
  throw new Error(
    'MAPROOM_DATABASE_URL environment variable is required for daemon operation'
  )
}
```

**Issues:**
- Validation assumes `MAPROOM_DATABASE_URL` is always set
- No distinction between PostgreSQL connection strings and SQLite file paths
- Error messages don't guide SQLite users

### Test Helpers (`helpers/database.ts`)

```typescript
import { Client } from 'pg'

export function getDatabaseUrl(): string {
  const dbUrl = process.env.TEST_DATABASE_URL || process.env.MAPROOM_DATABASE_URL
  if (!dbUrl) {
    throw new Error(
      'No TEST_DATABASE_URL or MAPROOM_DATABASE_URL environment variable set. ' +
      'Set TEST_DATABASE_URL to run E2E tests with a test database.'
    )
  }
  return dbUrl
}

export async function createClient(): Promise<Client> {
  const client = new Client({ connectionString: getDatabaseUrl() })
  await client.connect()
  return client
}
```

**Issues:**
- Hard dependency on `pg` (PostgreSQL client library)
- All test utilities assume PostgreSQL
- No mechanism for SQLite-based test fixtures

### PostgreSQL-Specific Code Paths (Critical)

Beyond the obvious database resolution, several MCP server code paths have hardcoded PostgreSQL dependencies that bypass the daemon abstraction:

#### 1. `search.ts:fetchChunkIds()` (Lines 138-182)

```typescript
async function fetchChunkIds(
  client: Client,  // <-- PostgreSQL client
  repo: string,
  hits: RustSearchHit[]
): Promise<Map<string, number>> {
  // Direct PostgreSQL SQL with maproom schema
  const query = `
    SELECT c.id, f.relpath, c.start_line, c.end_line
    FROM maproom.chunks c
    JOIN maproom.files f ON f.id = c.file_id
    JOIN maproom.repos r ON r.id = f.repo_id
    WHERE r.name = $1 ...
  `
}
```

**Impact:** Search tool receives PostgreSQL client from caller and executes schema-specific SQL.
**Location:** `src/tools/search.ts:138-182`

#### 2. `index.ts:getPg()` (Lines 332-341)

```typescript
async function getPg(): Promise<Client> {
  const DEFAULT_DATABASE_URL = 'postgresql://maproom:maproom@maproom-postgres:5432/maproom'
  const connectionString = process.env.MAPROOM_DATABASE_URL || process.env.PG_DATABASE_URL || DEFAULT_DATABASE_URL
  const client = new Client({ connectionString })
  await client.connect()
  return client
}
```

**Impact:** Doesn't use `resolveDatabase()`, always creates PostgreSQL client.
**Location:** `src/index.ts:332-341`

#### 3. `index.ts:handleStatus()` (Lines 353-457)

```typescript
async function handleStatus(params: any): Promise<any> {
  const client = await getPg()  // <-- Uses getPg()
  // Direct PostgreSQL queries for repo stats
  const statsQuery = `
    SELECT r.name as repo_name, w.name as worktree_name...
    FROM maproom.repos r
    LEFT JOIN maproom.worktrees w ON w.repo_id = r.id
    ...
  `
}
```

**Impact:** Status tool bypasses daemon entirely, uses direct PostgreSQL SQL.
**Location:** `src/index.ts:353-457`

#### Summary of PostgreSQL Dependencies

| Code Path | Uses Daemon? | SQLite Compatible? | MVP Strategy |
|-----------|--------------|-------------------|--------------|
| `resolveDatabaseConfig()` | N/A | Yes | Core deliverable |
| `getDaemonClient()` | Yes | Yes | URL passed to daemon |
| `handleSearchTool()` | Partial | Partial | Skip `fetchChunkIds` for SQLite |
| `handleStatus()` | No | No | Degraded response for SQLite |
| `handleOpen()` | Yes | Yes | Works via daemon |
| Test helpers | No | No | Separate SQLite helpers |

## Dependencies

### Completed Prerequisites

1. **VECSTORE** (Complete) - VectorStore trait with SQLite implementation
2. **MAPCLI** (Complete) - CLI supports SQLite via `MAPROOM_DATABASE_URL=sqlite://...`

### Upstream Dependencies

- `crewchief-maproom` binary with SQLite feature (`--features sqlite`)
- Pre-indexed SQLite fixture at `crates/maproom/tests/fixtures/pre-indexed-maproom.db`

## Scope Definition

### In Scope

1. **URL Parsing Enhancement**
   - Parse `sqlite://` URL scheme
   - Validate SQLite file path existence
   - Support relative and absolute paths
   - Handle `~/.maproom/maproom.db` default

2. **Daemon Configuration**
   - Pass SQLite URLs to daemon without modification
   - Handle missing database file gracefully
   - Provide SQLite-specific error messages

3. **Test Infrastructure**
   - Create SQLite-based test helpers
   - Mock or bypass PostgreSQL-only test utilities
   - Enable running tests without PostgreSQL service

4. **Integration Tests**
   - Verify MCP tools work with SQLite backend
   - Test status, search, open tools
   - Validate error handling for missing SQLite files

### Out of Scope

1. **PostgreSQL Feature Changes** - Existing PostgreSQL functionality must remain unchanged
2. **Rust Daemon Modifications** - MAPCLI already handles SQLite in the daemon
3. **Embedding Provider Changes** - Provider detection unchanged
4. **VSCode Extension** - Handled by separate VSCODEDB project

## Risk Assessment

| Risk | Impact | Likelihood | Mitigation |
|------|--------|------------|------------|
| Breaking PostgreSQL tests | High | Low | Run full test suite before/after changes |
| URL parsing edge cases | Medium | Medium | Comprehensive unit tests for URL parser |
| Test isolation failures | Medium | Low | Use separate SQLite fixture file |
| Daemon compatibility | Medium | Low | MAPCLI already validated SQLite daemon |
| **fetchChunkIds PostgreSQL dependency** | **High** | **High** | Skip call for SQLite, use chunk_id=0 with warning |
| **handleStatus PostgreSQL dependency** | **Medium** | **High** | Return degraded response for SQLite with hint |
| **getPg() direct connection** | **High** | **High** | Conditional execution based on backend type |

## Technical Constraints

1. **No Breaking Changes** - All existing PostgreSQL functionality must work
2. **Zero External Dependencies** - SQLite URL parsing must not add new npm packages
3. **Backward Compatible Tests** - Tests must pass with PostgreSQL when available
4. **Cross-Platform Paths** - SQLite file paths must work on macOS, Linux, Windows

## Success Criteria

1. `MAPROOM_DATABASE_URL=sqlite:///path/to/db.sqlite` works in MCP server
2. `~/.maproom/maproom.db` auto-detected when no URL specified
3. MCP tools (`search`, `status`, `open`) return valid results with SQLite
4. Tests can run without PostgreSQL service container
5. All existing PostgreSQL tests continue to pass

## Research Findings

### SQLite URL Format Conventions

Standard SQLite connection string formats:
- `sqlite:///absolute/path/to/database.db` (3 slashes for absolute)
- `sqlite://./relative/path.db` (2 slashes + relative)
- `sqlite::memory:` (in-memory database, not relevant for MCP)

### Node.js Path Handling

```typescript
import { homedir } from 'node:os'
import { resolve, isAbsolute } from 'node:path'
import { existsSync } from 'node:fs'

// Expand ~ to home directory
const expandPath = (p: string) =>
  p.startsWith('~') ? p.replace('~', homedir()) : p

// Validate SQLite URL
const validateSqliteUrl = (url: string): string => {
  if (!url.startsWith('sqlite://')) throw new Error('Invalid SQLite URL')
  const path = url.slice('sqlite://'.length)
  const expanded = expandPath(path)
  const resolved = isAbsolute(expanded) ? expanded : resolve(process.cwd(), expanded)
  if (!existsSync(resolved)) throw new Error(`SQLite database not found: ${resolved}`)
  return `sqlite://${resolved}`
}
```

### Test Strategy Research

Pattern for database-agnostic testing:
1. Check `MAPROOM_DATABASE_URL` prefix
2. If `sqlite://` → use SQLite test helpers
3. If `postgresql://` → use existing pg helpers
4. If unset → default to SQLite for zero-config testing

This matches the MAPCLI approach where SQLite is the zero-config default.
