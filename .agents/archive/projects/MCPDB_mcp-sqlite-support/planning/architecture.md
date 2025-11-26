# MCPDB Architecture - MCP Server SQLite Support

## Solution Overview

Extend the Maproom MCP server to support SQLite as a database backend alongside PostgreSQL. The solution follows a **detection-first** pattern where the URL scheme determines the backend, with SQLite as the zero-config default for simpler deployments.

## Architecture Decisions

### Decision 1: URL Scheme Detection

**Approach**: Parse `MAPROOM_DATABASE_URL` prefix to determine backend type

```
sqlite://...  → SQLite backend
postgresql:// → PostgreSQL backend
postgres://   → PostgreSQL backend (alias)
(none)        → Auto-detect SQLite default location
```

**Rationale**:
- Consistent with MAPCLI implementation
- No configuration migration required
- Backward compatible with existing PostgreSQL deployments

### Decision 2: SQLite Default Location

**Approach**: When no `MAPROOM_DATABASE_URL` is set, check for SQLite file at `~/.maproom/maproom.db`

**Resolution Order**:
1. `MAPROOM_DATABASE_URL` environment variable (explicit)
2. `IN_DEVCONTAINER=true` → PostgreSQL container (devcontainer)
3. `~/.maproom/maproom.db` exists → SQLite (zero-config)
4. Default PostgreSQL localhost:5433 (legacy fallback)

**Rationale**:
- Zero-config experience for SQLite users
- DevContainer users continue to get PostgreSQL
- Legacy localhost fallback maintains backward compatibility

### Decision 3: Test Infrastructure

**Approach**: Separate SQLite test files (not abstracted helpers)

```typescript
// helpers/sqlite.ts - NEW, isolated from PostgreSQL helpers
export function createTestSqliteDatabase(): string {
  const testDbPath = join(tmpdir(), `maproom-test-${Date.now()}.db`)
  copyFileSync(FIXTURE_SOURCE, testDbPath)
  return testDbPath
}
```

**Rationale**:
- SQLite tests use separate helper file (`helpers/sqlite.ts`)
- PostgreSQL helpers (`helpers/database.ts`) remain unchanged
- No abstraction layer complexity
- Tests can run independently with either backend
- Pre-indexed SQLite fixture provides consistent test data

### Decision 4: Legacy PostgreSQL Dependency Handling

**Approach**: Conditional execution with graceful degradation for SQLite mode

**Problem**: Several MCP server code paths bypass the daemon and use direct PostgreSQL:
- `search.ts:fetchChunkIds()` - enriches search results with chunk IDs
- `index.ts:getPg()` - creates direct PostgreSQL connection
- `index.ts:handleStatus()` - queries database directly for stats

**Strategy for MVP**:
1. Detect backend type via `resolveDatabaseConfig().type`
2. For PostgreSQL: execute existing code unchanged
3. For SQLite: skip PostgreSQL-dependent features with warnings

```typescript
// Pseudo-code for conditional execution
const config = resolveDatabaseConfig()

if (config.type === 'sqlite') {
  // Skip fetchChunkIds, use chunk_id=0 with warning
  log.warn('SQLite mode: chunk IDs not available, using 0')
  return hits.map(hit => ({ ...hit, chunk_id: 0 }))
} else {
  // PostgreSQL: use existing fetchChunkIds
  const chunkIdMap = await fetchChunkIds(client, repo, hits)
}
```

**Rationale**:
- Minimal code changes for MVP
- PostgreSQL functionality unchanged (no regressions)
- SQLite users get core functionality (search works)
- Clear documentation of limitations
- Proper fix (daemon returns chunk IDs) deferred to Phase 2

## Component Design

### 1. URL Parser Module (`resolve-database.ts`)

```typescript
interface DatabaseConfig {
  type: 'postgresql' | 'sqlite'
  url: string
  path?: string  // SQLite file path (for validation)
}

export function resolveDatabaseConfig(): DatabaseConfig {
  const url = process.env.MAPROOM_DATABASE_URL

  // Tier 1: Explicit URL
  if (url) {
    if (url.startsWith('sqlite://')) {
      return parseSqliteUrl(url)
    }
    return { type: 'postgresql', url }
  }

  // Tier 2: DevContainer
  if (process.env.IN_DEVCONTAINER === 'true') {
    return {
      type: 'postgresql',
      url: 'postgresql://maproom:maproom@maproom-postgres:5432/maproom'
    }
  }

  // Tier 3: SQLite default
  const sqlitePath = expandPath('~/.maproom/maproom.db')
  if (existsSync(sqlitePath)) {
    return {
      type: 'sqlite',
      url: `sqlite://${sqlitePath}`,
      path: sqlitePath
    }
  }

  // Tier 4: Legacy PostgreSQL fallback
  return {
    type: 'postgresql',
    url: 'postgresql://maproom:maproom@localhost:5433/maproom'
  }
}
```

### 2. SQLite URL Parser (`parseSqliteUrl`)

```typescript
function parseSqliteUrl(url: string): DatabaseConfig {
  // Extract path from sqlite:// URL
  const path = url.slice('sqlite://'.length)

  // Expand ~ to home directory
  const expanded = path.startsWith('~')
    ? path.replace('~', homedir())
    : path

  // Resolve relative paths
  const resolved = isAbsolute(expanded)
    ? expanded
    : resolve(process.cwd(), expanded)

  // Validate file exists (optional - can defer to daemon)
  // Omit validation for create-on-demand scenarios

  return {
    type: 'sqlite',
    url: `sqlite://${resolved}`,
    path: resolved
  }
}
```

### 3. Daemon Configuration (`daemon.ts`)

Current:
```typescript
if (!process.env.MAPROOM_DATABASE_URL) {
  throw new Error('MAPROOM_DATABASE_URL environment variable is required')
}
```

Updated:
```typescript
const config = resolveDatabaseConfig()

// Validate based on type
if (config.type === 'sqlite' && config.path) {
  if (!existsSync(config.path)) {
    throw new Error(
      `SQLite database not found: ${config.path}\n\n` +
      `To create an index:\n` +
      `  crewchief-maproom scan --path /your/repo\n\n` +
      `Or specify a different database:\n` +
      `  export MAPROOM_DATABASE_URL=sqlite:///path/to/your.db`
    )
  }
}

daemonClient = new DaemonClient({
  binaryPath,
  env: {
    MAPROOM_DATABASE_URL: config.url,
    // ... other env vars
  }
})
```

### 4. Test Helpers (`helpers/database.ts`)

Add SQLite-compatible interface:

```typescript
export interface TestDatabaseClient {
  query<T>(sql: string, params?: unknown[]): Promise<{ rows: T[] }>
  end(): Promise<void>
}

export async function createTestClient(): Promise<TestDatabaseClient> {
  const url = getTestDatabaseUrl()

  if (url.startsWith('sqlite://')) {
    // SQLite: spawn daemon and use JSON-RPC
    // Or use pre-indexed fixture directly
    return new SqliteTestClient(url)
  }

  // PostgreSQL: use pg client
  const { Client } = await import('pg')
  const client = new Client({ connectionString: url })
  await client.connect()
  return client
}
```

## Data Flow

### Search Request with SQLite

```
MCP Client
    │
    │ tools/call: search
    ▼
MCP Server (index.ts)
    │
    │ resolveDatabaseConfig()
    │   → type: 'sqlite', url: 'sqlite://~/.maproom/maproom.db'
    ▼
getDaemonClient()
    │
    │ env: { MAPROOM_DATABASE_URL: 'sqlite://...' }
    ▼
Rust Daemon (crewchief-maproom serve)
    │
    │ VectorStore::get_store()
    │   → SqliteStore
    ▼
SQLite Database
    │
    │ FTS5 search
    ▼
Search Results
```

### Status Request (Zero-Config)

```
MCP Client
    │
    │ tools/call: status
    ▼
MCP Server
    │
    │ No MAPROOM_DATABASE_URL
    │ No IN_DEVCONTAINER
    │ Check ~/.maproom/maproom.db → EXISTS
    │   → type: 'sqlite', url: 'sqlite://~/.maproom/maproom.db'
    ▼
Daemon → SQLite → Status Response
```

## PostgreSQL-Specific Code Paths

The following code paths have hardcoded PostgreSQL dependencies that require special handling for SQLite:

### search.ts:fetchChunkIds() - CRITICAL

**Location**: `src/tools/search.ts:138-182`
**Purpose**: Enrich search results with database chunk IDs
**Problem**: Uses direct PostgreSQL client with `maproom.` schema queries

**SQLite Handling**:
```typescript
// In handleSearchTool(), before fetchChunkIds call:
const config = resolveDatabaseConfig()
let chunkIdMap: Map<string, number>

if (config.type === 'sqlite') {
  // SQLite: daemon doesn't return chunk IDs (Phase 2 enhancement)
  log.warn({ hits: rustOutput.hits.length }, 'SQLite mode: chunk IDs unavailable, using 0')
  chunkIdMap = new Map() // Empty map = all chunk_id will be 0
} else {
  // PostgreSQL: use existing fetchChunkIds
  chunkIdMap = await fetchChunkIds(client, repo, rustOutput.hits)
}
```

### index.ts:handleStatus() - DEGRADED

**Location**: `src/index.ts:353-457`
**Purpose**: Return index statistics (repo count, file count, chunk count)
**Problem**: Uses `getPg()` for direct SQL queries

**SQLite Handling**:
```typescript
async function handleStatus(params: any): Promise<any> {
  const config = resolveDatabaseConfig()

  if (config.type === 'sqlite') {
    // Return basic response with SQLite info
    return {
      repos: [],
      totalRepos: 0,
      totalFiles: 0,
      totalChunks: 0,
      hint: 'SQLite mode: detailed statistics not available. Use search tool for indexed content.',
      backendType: 'sqlite',
      sqlitePath: config.path
    }
  }

  // PostgreSQL: existing implementation unchanged
  const client = await getPg()
  // ... existing code
}
```

### index.ts:getPg() - NO CHANGE

**Location**: `src/index.ts:332-341`
**Purpose**: Create PostgreSQL client for direct queries
**Handling**: Not modified. Callers check backend type before calling.

## Interface Contracts

### DatabaseConfig

```typescript
interface DatabaseConfig {
  /** Backend type */
  type: 'postgresql' | 'sqlite'

  /** Full connection URL (passed to daemon) */
  url: string

  /** SQLite file path (for validation, only set for sqlite type) */
  path?: string
}
```

### Updated resolveDatabase Export

```typescript
// Backward compatible: returns string URL
export function resolveDatabase(): string

// New: returns full config with type detection
export function resolveDatabaseConfig(): DatabaseConfig

// New: checks if URL is SQLite
export function isSqliteUrl(url: string): boolean
```

## Error Handling

### SQLite-Specific Errors

| Error Condition | User Message |
|-----------------|--------------|
| SQLite file not found | "SQLite database not found: /path/to/db.sqlite\n\nTo create an index:\n  crewchief-maproom scan --path /your/repo" |
| Invalid SQLite URL | "Invalid SQLite URL format. Expected: sqlite:///path/to/database.db" |
| SQLite permission denied | "Cannot read SQLite database: Permission denied\n  Path: /path/to/db.sqlite" |

### Graceful Degradation

When SQLite file is missing but PostgreSQL is available:
1. Log warning about missing SQLite
2. Fall back to PostgreSQL localhost
3. Include hint in status response

## Technology Choices

| Component | Technology | Rationale |
|-----------|------------|-----------|
| URL parsing | Node.js built-ins | Zero dependencies, cross-platform |
| Path expansion | `os.homedir()`, `path.resolve()` | Standard Node.js |
| File validation | `fs.existsSync()` | Synchronous for simplicity |
| Test abstraction | Interface + factory | Backend-agnostic testing |

## Performance Considerations

1. **URL Resolution** - Called once per daemon initialization, no caching needed
2. **File Existence Check** - Single `existsSync` call, ~1ms overhead
3. **Test Setup** - SQLite tests are faster (no network, no container)

## Migration Path

### Existing PostgreSQL Users

No changes required. Behavior unchanged:
- `MAPROOM_DATABASE_URL=postgresql://...` continues to work
- `IN_DEVCONTAINER=true` continues to use PostgreSQL

### New SQLite Users

Zero-config path:
1. Run `crewchief-maproom scan --path /repo` (creates `~/.maproom/maproom.db`)
2. MCP server auto-detects SQLite database
3. Search works immediately

### Explicit SQLite Configuration

```bash
export MAPROOM_DATABASE_URL=sqlite:///path/to/custom.db
```

## Diagram

```
                        ┌─────────────────────────────────────────────────────────┐
                        │                    MCP Server                           │
                        │  ┌────────────────┐    ┌──────────────────────┐        │
                        │  │ resolve-       │    │ daemon.ts            │        │
  MAPROOM_DATABASE_URL──┼─▶│ database.ts    │───▶│                      │        │
                        │  │                │    │ getDaemonClient()    │        │
                        │  │ type: sqlite   │    │   env: { URL: ... }  │        │
                        │  │ url: sqlite:// │    │                      │        │
                        │  └────────────────┘    └──────────┬───────────┘        │
                        │                                   │                     │
                        │                                   │ spawn               │
                        │                                   ▼                     │
                        │                        ┌──────────────────────┐        │
                        │                        │ crewchief-maproom    │        │
                        │                        │ serve                │        │
                        │                        │                      │        │
                        │                        │ VectorStore trait    │        │
                        │                        │   ├─ PostgresStore   │        │
                        │                        │   └─ SqliteStore ◀── │        │
                        │                        └──────────┬───────────┘        │
                        └───────────────────────────────────┼─────────────────────┘
                                                            │
                                                            ▼
                                               ┌──────────────────────┐
                                               │ ~/.maproom/          │
                                               │   maproom.db         │
                                               │                      │
                                               │ (SQLite + FTS5)      │
                                               └──────────────────────┘
```
