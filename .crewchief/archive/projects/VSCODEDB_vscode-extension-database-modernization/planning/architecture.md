# VSCODEDB - Architecture Design

## Overview

This document defines the architecture for adding SQLite support to the VSCode extension as the default, zero-config database backend while preserving PostgreSQL as an advanced option.

## High-Level Design

```
┌─────────────────────────────────────────────────────────────────┐
│                     VSCode Extension                             │
├─────────────────────────────────────────────────────────────────┤
│  ┌──────────────┐     ┌──────────────────────────────────────┐  │
│  │ extension.ts │────▶│ database-checker.ts (NEW)            │  │
│  │              │     │  ├─ resolveDatabaseConfig()          │  │
│  │              │     │  ├─ checkDatabaseAvailable()         │  │
│  │              │     │  └─ getDatabaseUrl()                 │  │
│  └──────────────┘     └──────────────────────────────────────┘  │
│         │                              │                         │
│         │                              ▼                         │
│         │                   ┌──────────────────┐                │
│         │                   │  SQLite Mode     │                │
│         │                   │  (No Docker)     │                │
│         │                   └──────────────────┘                │
│         │                              │                         │
│         ▼                              ▼                         │
│  ┌──────────────┐           ┌──────────────────┐                │
│  │DockerManager │◀─────────▶│ PostgreSQL Mode  │                │
│  │ (optional)   │           │ (Docker)         │                │
│  └──────────────┘           └──────────────────┘                │
└─────────────────────────────────────────────────────────────────┘
                               │
                               ▼
                    ┌──────────────────┐
                    │ ProcessOrchest.  │
                    │ (both modes)     │
                    └──────────────────┘
                               │
                               ▼
                    ┌──────────────────┐
                    │ crewchief-maproom│
                    │ Rust Binary      │
                    └──────────────────┘
```

## Configuration Resolution Patterns

**Important:** The VSCode extension and MCP server use intentionally different configuration patterns:

| Layer | Pattern | Reason |
|-------|---------|--------|
| **VSCode Extension** | Settings-based | User preference (persisted in settings.json) |
| **MCP Server** | Environment-based | Runtime injection by spawning process |

**How They Work Together:**
1. Extension reads user preference from VSCode settings (`maproom.database.provider`)
2. Extension resolves database URL based on settings
3. Extension SETS `MAPROOM_DATABASE_URL` environment variable when spawning daemon
4. Daemon reads from environment variable (standard process injection)

This is the correct separation:
- **Extension** = User-facing, settings-based configuration
- **Daemon** = Server process, receives config via environment from parent

DevContainer detection in MCP server (`IN_DEVCONTAINER`) is for standalone MCP usage. When spawned by VSCode extension, the extension controls the database URL via environment variable, overriding any auto-detection.

## Architecture Decisions

### AD-1: Create `database-checker.ts` (Replace `postgres-checker.ts`)

**Decision**: Create a new unified database checker that handles both SQLite and PostgreSQL.

**Rationale**:
- Clean abstraction over two database backends
- Follows same pattern as MCPDB's `resolve-database.ts`
- Keeps `postgres-checker.ts` available for reference but deprecated

**Interface**:
```typescript
interface DatabaseConfig {
  type: 'sqlite' | 'postgresql'
  url: string
  path?: string  // Only for sqlite
}

// Core functions
function resolveDatabaseConfig(): DatabaseConfig
function checkDatabaseAvailable(config: DatabaseConfig): Promise<boolean>
function getDatabaseUrl(config: DatabaseConfig): string
function getDatabaseUnavailableMessage(config: DatabaseConfig): string
```

### AD-2: Conditional Docker Activation

**Decision**: Only start Docker services when PostgreSQL mode is configured.

**Rationale**:
- SQLite users shouldn't see Docker-related UI or errors
- Keeps fast activation path for SQLite mode
- Preserves full Docker experience for PostgreSQL users

**Implementation**:
```typescript
async function initializeServices(context, workspaceRoot) {
  const dbConfig = resolveDatabaseConfig()

  if (dbConfig.type === 'postgresql') {
    // Existing Docker flow
    await ensureDockerRunning(context, provider)
    await ensurePostgresAvailable()
  } else {
    // SQLite flow - verify file exists
    await ensureSqliteAvailable(dbConfig)
  }

  // Common flow continues
  await startWatchProcesses(context, workspaceRoot)
}
```

### AD-3: Settings Schema Extension

**Decision**: Add `maproom.database.sqlitePath` setting with smart default.

**Rationale**:
- Users need to specify non-default SQLite paths
- Default `~/.maproom/maproom.db` matches CLI behavior
- Preserves all existing PostgreSQL settings

**Schema Addition**:
```json
"maproom.database.sqlitePath": {
  "type": "string",
  "default": "",
  "description": "Path to SQLite database file. Leave empty for default (~/.maproom/maproom.db). Only used when provider is 'sqlite'."
}
```

### AD-4: Setup Wizard SQLite Path

**Decision**: Update setup wizard to allow SQLite file selection.

**Rationale**:
- New users need guidance for first-time setup
- File picker provides better UX than manual path entry
- Can auto-detect existing `~/.maproom/maproom.db`

**Flow**:
```
1. Check if ~/.maproom/maproom.db exists
   ├─ Yes: Ask "Use existing index?" → Done
   └─ No: Continue to step 2

2. Select database provider (existing step)
   ├─ SQLite (recommended): Ask for path or use default
   └─ PostgreSQL: Existing Docker flow
```

### AD-5: ProcessOrchestrator Database URL

**Decision**: Use existing `databaseUrlOverride` field to pass database URL.

**Rationale**:
- `databaseUrlOverride` already exists in OrchestratorConfig (line 57)
- Orchestrator already supports SQLite URLs via this field
- Preserves backward compatibility - no interface changes needed
- Keep `postgres` config as fallback for existing code paths

**Existing Interface** (No changes required):
```typescript
interface OrchestratorConfig {
  extensionRoot: string
  workspaceRoot: string
  postgres?: PostgresConfig      // Keep for backward compatibility
  databaseUrlOverride?: string   // Already supports SQLite URLs!
  secretsManager: SecretsManager
  provider: Provider
}
```

**Usage**:
```typescript
orchestrator = new ProcessOrchestrator(outputChannel!, {
  extensionRoot: context.extensionPath,
  workspaceRoot,
  databaseUrlOverride: dbConfig.url,  // Works for both SQLite and PostgreSQL
  secretsManager,
  provider,
})
```

**Note**: The `postgres` config remains available as a fallback. When `databaseUrlOverride` is provided, it takes precedence (existing behavior in `buildEnvironment()` at line 338).

## Component Design

### database-checker.ts

```typescript
/**
 * Unified database availability checker for Maproom VSCode extension
 *
 * Supports both SQLite (file existence) and PostgreSQL (TCP check) backends.
 * Resolution priority matches MCP server's resolve-database.ts
 */

import { existsSync } from 'node:fs'
import { createConnection, type Socket } from 'node:net'
import * as vscode from 'vscode'
import { homedir } from 'node:os'
import { resolve, isAbsolute } from 'node:path'

export interface DatabaseConfig {
  type: 'sqlite' | 'postgresql'
  url: string
  path?: string
}

export interface PostgresConfig {
  host: string
  port: number
  user: string
  password: string
  database: string
}

/**
 * Resolve database configuration from VSCode settings
 *
 * Priority:
 * 1. Settings: maproom.database.provider determines type
 * 2. For SQLite: Use sqlitePath setting or default ~/.maproom/maproom.db
 * 3. For PostgreSQL: Build URL from host/port/user/password/name settings
 */
export function resolveDatabaseConfig(): DatabaseConfig {
  const config = vscode.workspace.getConfiguration('maproom.database')
  const provider = config.get<string>('provider') ?? 'sqlite'

  if (provider === 'sqlite') {
    const pathSetting = config.get<string>('sqlitePath') ?? ''
    const path = pathSetting || `${homedir()}/.maproom/maproom.db`
    const expanded = expandPath(path)
    const resolved = isAbsolute(expanded) ? expanded : resolve(process.cwd(), expanded)

    return {
      type: 'sqlite',
      url: `sqlite://${resolved}`,
      path: resolved,
    }
  }

  // PostgreSQL mode
  const pgConfig = getPostgresConfigFromSettings()
  return {
    type: 'postgresql',
    url: getPostgresUrl(pgConfig),
  }
}

export async function checkDatabaseAvailable(config: DatabaseConfig): Promise<boolean> {
  if (config.type === 'sqlite') {
    return existsSync(config.path!)
  }
  // For PostgreSQL, use settings-based config directly (already resolved in resolveDatabaseConfig)
  const pgConfig = getPostgresConfigFromSettings()
  return checkPostgresAvailable(pgConfig)
}

export function getDatabaseUnavailableMessage(config: DatabaseConfig): string {
  if (config.type === 'sqlite') {
    return (
      `SQLite database not found at: ${config.path}\n\n` +
      `To create an index, run:\n` +
      `  crewchief-maproom scan --sqlite ${config.path} /path/to/your/repo\n\n` +
      `Or change the database path in Settings > Maproom > Database`
    )
  }
  return getPostgresUnavailableMessage()
}
```

### extension.ts Changes

**Modified Functions**:

1. `initializeServices()` - Branch on database type
2. `runFirstTimeSetup()` - Skip Docker for SQLite
3. `startWatchProcesses()` - Pass databaseUrl instead of postgres config

**New Functions**:

1. `ensureSqliteAvailable()` - Validate SQLite file exists

### package.json Settings Changes

Add new setting:
```json
"maproom.database.sqlitePath": {
  "type": "string",
  "default": "",
  "markdownDescription": "Path to SQLite database file. Leave empty for default (`~/.maproom/maproom.db`). Only used when provider is 'sqlite'."
}
```

## Data Flow

### SQLite Mode Activation

```
1. Extension activates
2. resolveDatabaseConfig() returns { type: 'sqlite', path: '~/.maproom/maproom.db' }
3. checkDatabaseAvailable() verifies file exists
4. Skip Docker initialization entirely
5. ProcessOrchestrator starts with sqlite:// URL
6. crewchief-maproom daemon connects to SQLite
7. Extension ready in <500ms
```

### PostgreSQL Mode Activation

```
1. Extension activates
2. resolveDatabaseConfig() returns { type: 'postgresql', url: 'postgresql://...' }
3. ensureDockerRunning() starts containers
4. checkDatabaseAvailable() verifies TCP connectivity
5. ProcessOrchestrator starts with postgresql:// URL
6. crewchief-maproom daemon connects to PostgreSQL
7. Extension ready (depends on Docker startup)
```

## Technology Choices

### No New Dependencies

This project adds zero new npm dependencies:
- File existence: `node:fs` (existsSync)
- Path handling: `node:path` (resolve, isAbsolute)
- Home directory: `node:os` (homedir)
- TCP check: `node:net` (existing)

### Reusing Patterns

From MCPDB project:
- `DatabaseConfig` interface shape
- `expandPath()` tilde expansion
- `isSqliteUrl()` URL detection
- Error message formatting

## Performance Considerations

### Activation Time

SQLite mode should be faster than PostgreSQL mode:
- No Docker health check (saves 1-2s)
- No TCP connectivity check (saves ~100ms)
- File existence check is <1ms

### Memory Usage

SQLite mode reduces memory footprint:
- No PostgreSQL container (~100MB)
- No Ollama container (if using cloud provider)
- Only crewchief-maproom daemon running

## Long-Term Maintainability

### Migration Path

Users can migrate between backends:
1. Export from PostgreSQL: `crewchief-maproom export --postgres --output /path/to/db.sqlite`
2. Change setting: `maproom.database.provider: sqlite`
3. Restart extension

### Feature Parity Table

| Feature | SQLite | PostgreSQL |
|---------|--------|------------|
| Full-text search | Yes | Yes |
| Vector similarity | Yes (sqlite-vec) | Yes (pgvector) |
| Incremental upsert | Yes | Yes |
| Multi-worktree | Yes | Yes |
| Detailed stats | No (degraded) | Yes |
| Team sharing | No | Yes (network) |

### Documentation Requirements

- Update extension README with SQLite-first getting started
- Document PostgreSQL as "advanced" option
- Add troubleshooting for common SQLite issues

## User Interface

### Status Bar Mode Indicator

Display current database mode in the VSCode status bar:

**Specification**:
- **Icon**: `$(database)` (VSCode built-in codicon)
- **Text**: "SQLite" or "PostgreSQL"
- **Full Display**: `$(database) SQLite` or `$(database) PostgreSQL`
- **Position**: Right side of status bar (low priority, non-intrusive)
- **Tooltip**: "Maproom: Using SQLite database at ~/.maproom/maproom.db" (with actual path)
- **Click Action**: Open Maproom settings

**Implementation**:
```typescript
const statusBarItem = vscode.window.createStatusBarItem(
  vscode.StatusBarAlignment.Right,
  100  // Low priority
)
statusBarItem.text = `$(database) ${dbConfig.type === 'sqlite' ? 'SQLite' : 'PostgreSQL'}`
statusBarItem.tooltip = `Maproom: Using ${dbConfig.type} database${dbConfig.path ? ` at ${dbConfig.path}` : ''}`
statusBarItem.command = 'maproom.openSettings'
statusBarItem.show()
```

## Error Handling

### Error Recovery Flow

When database becomes unavailable during extension operation:

```
1. Database operation fails
2. Extension shows error notification:
   - Title: "Maproom: Database Unavailable"
   - Message: getDatabaseUnavailableMessage(config)
   - Actions: ["Re-run Setup", "Open Settings", "Dismiss"]

3. User actions:
   - "Re-run Setup" → runFirstTimeSetup()
   - "Open Settings" → open Maproom settings page
   - "Dismiss" → hide notification (retry on next operation)
```

**Implementation**:
```typescript
async function handleDatabaseError(config: DatabaseConfig, error: Error) {
  const message = getDatabaseUnavailableMessage(config)
  const action = await vscode.window.showErrorMessage(
    `Maproom: Database Unavailable`,
    { detail: message, modal: false },
    'Re-run Setup',
    'Open Settings'
  )

  if (action === 'Re-run Setup') {
    await runFirstTimeSetup()
  } else if (action === 'Open Settings') {
    await vscode.commands.executeCommand(
      'workbench.action.openSettings',
      'maproom.database'
    )
  }
}
```

### First-Run Detection

Extension automatically runs setup wizard when:
1. SQLite mode AND `~/.maproom/maproom.db` does not exist
2. PostgreSQL mode AND Docker containers are not running
3. User has never completed setup (tracked in extension state)

This matches existing behavior in `extension.ts` - no changes needed to detection logic.
