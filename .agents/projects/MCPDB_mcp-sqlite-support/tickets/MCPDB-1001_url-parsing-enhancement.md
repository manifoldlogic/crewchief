# Ticket: MCPDB-1001: URL Parsing Enhancement

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - tests executed and passing
- [x] **Verified** - by the verify-ticket agent

## Agents
- general-purpose (TypeScript implementation)
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary
Add SQLite URL parsing and `DatabaseConfig` type to `resolve-database.ts`, enabling the MCP server to detect and handle both SQLite and PostgreSQL database URLs.

## Background
The MCP server currently only supports PostgreSQL URLs. With MAPCLI completing SQLite support in the Rust daemon, the TypeScript layer needs to:
1. Parse `sqlite://` URL scheme
2. Expand `~` in paths to home directory
3. Auto-detect SQLite at `~/.maproom/maproom.db`
4. Return structured config with backend type information

**Plan Reference:** Phase 1 - URL Parsing Enhancement (plan.md)

## Acceptance Criteria
- [x] `DatabaseConfig` interface defined with `type`, `url`, and optional `path` fields
- [x] `resolveDatabaseConfig()` function returns `DatabaseConfig` instead of raw string
- [x] SQLite URLs (`sqlite://...`) correctly parsed with path expansion
- [x] `~/.maproom/maproom.db` auto-detected when no URL specified and file exists
- [x] `isSqliteUrl()` helper function exported for use by other modules
- [x] Backward-compatible `resolveDatabase()` export maintained (returns string)
- [x] Unit tests pass for all URL parsing scenarios

## Technical Requirements

### DatabaseConfig Interface
```typescript
interface DatabaseConfig {
  type: 'postgresql' | 'sqlite'
  url: string
  path?: string  // SQLite file path (for validation)
}
```

### Resolution Priority (4 tiers)
1. **Explicit URL**: `MAPROOM_DATABASE_URL` environment variable
2. **DevContainer**: `IN_DEVCONTAINER=true` → PostgreSQL container
3. **SQLite Default**: `~/.maproom/maproom.db` exists → SQLite
4. **PostgreSQL Fallback**: `localhost:5433` (legacy default)

### Path Handling
- Expand `~` to `os.homedir()`
- Resolve relative paths against `process.cwd()`
- Validate file extension (`.db`, `.sqlite`, `.sqlite3`)

## Implementation Notes

### File to Modify
`packages/maproom-mcp/src/utils/resolve-database.ts`

### Suggested Implementation
```typescript
import { existsSync } from 'node:fs'
import { homedir } from 'node:os'
import { resolve, isAbsolute } from 'node:path'

export interface DatabaseConfig {
  type: 'postgresql' | 'sqlite'
  url: string
  path?: string
}

export function isSqliteUrl(url: string): boolean {
  return url.startsWith('sqlite://')
}

function expandPath(p: string): string {
  return p.startsWith('~') ? p.replace('~', homedir()) : p
}

function parseSqliteUrl(url: string): DatabaseConfig {
  const path = url.slice('sqlite://'.length)
  const expanded = expandPath(path)
  const resolved = isAbsolute(expanded) ? expanded : resolve(process.cwd(), expanded)

  return {
    type: 'sqlite',
    url: `sqlite://${resolved}`,
    path: resolved
  }
}

export function resolveDatabaseConfig(): DatabaseConfig {
  const url = process.env.MAPROOM_DATABASE_URL

  // Tier 1: Explicit URL
  if (url) {
    if (isSqliteUrl(url)) {
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

  // Tier 4: PostgreSQL fallback
  return {
    type: 'postgresql',
    url: 'postgresql://maproom:maproom@localhost:5433/maproom'
  }
}

// Backward compatible export
export function resolveDatabase(): string {
  return resolveDatabaseConfig().url
}
```

### Test Cases Required
1. Parse absolute `sqlite://` URL
2. Parse relative `sqlite://` URL
3. Expand `~` in sqlite path
4. Return `postgresql` type for `postgres://` URL
5. Return `postgresql` type for `postgresql://` URL
6. Detect SQLite when `~/.maproom/maproom.db` exists (mock fs)
7. Fall back to PostgreSQL when SQLite not found
8. DevContainer takes precedence over SQLite default
9. Explicit URL takes precedence over auto-detection

## Dependencies
- None (first ticket in project)

## Risk Assessment
- **Risk**: Path handling differences on Windows
  - **Mitigation**: Use Node.js path module which handles cross-platform paths; document Windows limitations if any
- **Risk**: Breaking existing PostgreSQL functionality
  - **Mitigation**: Maintain backward-compatible `resolveDatabase()` export; run existing tests

## Files/Packages Affected
- `packages/maproom-mcp/src/utils/resolve-database.ts` (modify)
- `packages/maproom-mcp/tests/unit/resolve-database.test.ts` (create/modify)
