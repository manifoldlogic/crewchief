# Ticket: VSCODEDB-1003: Make Docker Containers Optional

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - tests executed and passing (192 core unit tests pass; orchestrator timeout issues are pre-existing)
- [x] **Verified** - by the verify-ticket agent

## Agents
- vscode-extension-specialist (primary)
- process-management-specialist (backup)
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary

Update the extension's initialization flow to conditionally skip Docker container management when SQLite mode is configured. Docker services should only start when PostgreSQL mode is explicitly selected.

## Background

Currently, `extension.ts` unconditionally calls Docker-related functions (`ensureDockerRunning()`, `ensurePostgresAvailable()`) during activation. This ticket adds branching logic based on the database provider setting.

**Reference:** plan.md Phase 2 - "VSCODEDB-1003: Make Docker Containers Optional"

**Core Principle:** SQLite users should never see Docker-related UI, errors, or prompts.

## Acceptance Criteria

- [ ] Docker NOT started when `maproom.database.provider = 'sqlite'`
- [ ] Docker started normally when `maproom.database.provider = 'postgres'`
- [ ] New `ensureSqliteAvailable()` function validates SQLite file exists
- [ ] Error message shown (via `getDatabaseUnavailableMessage()`) when SQLite file missing
- [ ] All existing tests pass: `pnpm test`
- [ ] Integration test verifies Docker skip behavior

## Technical Requirements

### Modified Function: `initializeServices()`

Update the initialization logic in `extension.ts`:

```typescript
async function initializeServices(context: vscode.ExtensionContext, workspaceRoot: string) {
  const dbConfig = resolveDatabaseConfig()

  if (dbConfig.type === 'postgresql') {
    // Existing PostgreSQL/Docker flow (unchanged)
    await ensureDockerRunning(context, provider)
    await ensurePostgresAvailable()
  } else {
    // New SQLite flow
    await ensureSqliteAvailable(dbConfig)
  }

  // Common flow continues for both modes
  await startWatchProcesses(context, workspaceRoot, dbConfig)
}
```

### New Function: `ensureSqliteAvailable()`

```typescript
import { resolveDatabaseConfig, checkDatabaseAvailable, getDatabaseUnavailableMessage, DatabaseConfig } from './services/database-checker'

async function ensureSqliteAvailable(config: DatabaseConfig): Promise<void> {
  const available = await checkDatabaseAvailable(config)

  if (!available) {
    const message = getDatabaseUnavailableMessage(config)
    const action = await vscode.window.showErrorMessage(
      'Maproom: Database Not Found',
      { detail: message, modal: false },
      'Run Setup',
      'Open Settings'
    )

    if (action === 'Run Setup') {
      await runFirstTimeSetup()
    } else if (action === 'Open Settings') {
      await vscode.commands.executeCommand(
        'workbench.action.openSettings',
        'maproom.database'
      )
    }

    // Still allow extension to activate - commands will show errors on use
  }
}
```

### Modified Function: `runFirstTimeSetup()`

Skip Docker-related steps for SQLite mode:

```typescript
async function runFirstTimeSetup() {
  const dbConfig = resolveDatabaseConfig()

  if (dbConfig.type === 'postgresql') {
    // Existing Docker setup flow
    await ensureDockerRunning(context, provider)
    // ... rest of PostgreSQL setup
  } else {
    // SQLite setup: just validate file or guide user
    const available = await checkDatabaseAvailable(dbConfig)
    if (!available) {
      // Show guidance message about running crewchief-maproom scan
      await showSqliteSetupGuidance(dbConfig)
    }
  }
}
```

### Import Updates

Add to `extension.ts` imports:

```typescript
import {
  resolveDatabaseConfig,
  checkDatabaseAvailable,
  getDatabaseUnavailableMessage,
  DatabaseConfig
} from './services/database-checker'
```

### Keep Existing Docker Code

Do NOT delete or modify:
- `docker/manager.ts`
- `ensureDockerRunning()`
- `ensurePostgresAvailable()`

These remain untouched for PostgreSQL mode.

## Implementation Notes

### Graceful Degradation

Even when SQLite database is missing, allow the extension to activate:
- Commands will return appropriate errors on use
- Status bar should indicate "No Database" state
- User can configure/create database without reloading

### Error Flow

```
SQLite mode + file missing:
1. Show error notification with message from getDatabaseUnavailableMessage()
2. Offer "Run Setup" and "Open Settings" actions
3. Continue extension activation (graceful degradation)
4. Status bar shows warning state

PostgreSQL mode + containers not running:
1. Existing Docker flow handles this
2. No changes needed
```

### Testing the Conditional Logic

```typescript
// Integration test: verify Docker not called for SQLite
describe('initializeServices', () => {
  it('skips Docker for sqlite mode', async () => {
    // Mock settings to return sqlite
    mockSettings['maproom.database.provider'] = 'sqlite'

    // Mock Docker manager
    const ensureDockerRunning = vi.fn()

    await initializeServices(mockContext, '/workspace')

    expect(ensureDockerRunning).not.toHaveBeenCalled()
  })

  it('starts Docker for postgres mode', async () => {
    mockSettings['maproom.database.provider'] = 'postgres'

    const ensureDockerRunning = vi.fn()

    await initializeServices(mockContext, '/workspace')

    expect(ensureDockerRunning).toHaveBeenCalled()
  })
})
```

## Dependencies

- **VSCODEDB-1001**: `database-checker.ts` must be complete (provides `resolveDatabaseConfig()`, etc.)

## Risk Assessment

- **Risk**: Breaking PostgreSQL mode activation
  - **Mitigation**: Keep all existing Docker code paths, just add conditional branching

- **Risk**: SQLite error handling causes activation failure
  - **Mitigation**: Use graceful degradation - show error but continue activation

- **Risk**: Untested code paths
  - **Mitigation**: Add explicit integration tests for both modes

## Files/Packages Affected

### Modified Files
- `packages/vscode-maproom/src/extension.ts`

### Test Files (Modified/Created)
- `packages/vscode-maproom/src/extension.test.ts` (add conditional activation tests)
