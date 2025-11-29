# Ticket: VSCODEDB-1004: Update Core Activation Flow for SQLite

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - tests executed and passing
- [x] **Verified** - by the verify-ticket agent

## Agents
- vscode-extension-specialist (primary)
- process-management-specialist (backup)
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary

Complete the core activation flow changes to support SQLite mode: update ProcessOrchestrator configuration to use database URL from `database-checker.ts`, add status bar mode indicator, and ensure sub-500ms activation for SQLite mode.

## Background

With VSCODEDB-1001 (database-checker.ts) and VSCODEDB-1003 (Docker optional) complete, this ticket finalizes the activation flow by:
1. Passing the resolved database URL to ProcessOrchestrator via existing `databaseUrlOverride` field
2. Adding a status bar indicator showing current database mode
3. Verifying activation performance

**Reference:** plan.md Phase 2 - "VSCODEDB-1004: Update Core Activation Flow for SQLite"

**Key Discovery:** The `databaseUrlOverride` field already exists in `OrchestratorConfig` (line 57 of orchestrator.ts) and already supports SQLite URLs. This minimizes required changes.

## Acceptance Criteria

- [ ] Extension activates without Docker when `provider = 'sqlite'` and database file exists
- [ ] ProcessOrchestrator receives correct database URL via `databaseUrlOverride` for both SQLite and PostgreSQL modes
- [ ] Status bar shows `$(database) SQLite` or `$(database) PostgreSQL` indicator
- [ ] Status bar tooltip shows database path/connection info
- [ ] Status bar click opens Maproom settings
- [ ] Activation time <500ms for SQLite mode (measured in Output channel)
- [ ] All existing tests pass: `pnpm test`

## Technical Requirements

### ProcessOrchestrator Configuration

Update the ProcessOrchestrator instantiation in `extension.ts`:

```typescript
import { resolveDatabaseConfig, getDatabaseUrl } from './services/database-checker'

// In activation flow:
const dbConfig = resolveDatabaseConfig()

orchestrator = new ProcessOrchestrator(outputChannel!, {
  extensionRoot: context.extensionPath,
  workspaceRoot,
  databaseUrlOverride: getDatabaseUrl(dbConfig),  // Use resolved URL for both modes
  secretsManager,
  provider,
})
```

**Note:** The `postgres` config field remains available as fallback, but `databaseUrlOverride` takes precedence when provided (existing behavior in `buildEnvironment()` at line 338).

### Status Bar Mode Indicator

Add database mode indicator to status bar:

```typescript
// Create status bar item
const dbStatusItem = vscode.window.createStatusBarItem(
  vscode.StatusBarAlignment.Right,
  100  // Low priority - appears on right side
)

// Configure based on database mode
const dbConfig = resolveDatabaseConfig()
dbStatusItem.text = `$(database) ${dbConfig.type === 'sqlite' ? 'SQLite' : 'PostgreSQL'}`
dbStatusItem.tooltip = `Maproom: ${dbConfig.type === 'sqlite'
  ? `SQLite database at ${dbConfig.path}`
  : 'PostgreSQL database'}`
dbStatusItem.command = 'workbench.action.openSettings'
dbStatusItem.show()

// Register for cleanup
context.subscriptions.push(dbStatusItem)
```

### Integration with Existing StatusBarManager

The extension already has `StatusBarManager` class in `src/ui/statusBar.ts`. Either:

**Option A:** Add to existing StatusBarManager:
```typescript
// In StatusBarManager class
private databaseStatusItem: vscode.StatusBarItem

public updateDatabaseMode(config: DatabaseConfig): void {
  this.databaseStatusItem.text = `$(database) ${config.type === 'sqlite' ? 'SQLite' : 'PostgreSQL'}`
  this.databaseStatusItem.tooltip = `Maproom: ${config.type === 'sqlite'
    ? `SQLite database at ${config.path}`
    : 'PostgreSQL database'}`
}
```

**Option B:** Create separate status bar item in extension.ts (simpler if StatusBarManager is complex)

Choose based on existing StatusBarManager complexity.

### Activation Performance

Measure and log activation time:

```typescript
const startTime = performance.now()

// ... activation code ...

const activationTime = performance.now() - startTime
outputChannel.appendLine(`Maproom: Activated in ${activationTime.toFixed(0)}ms (${dbConfig.type} mode)`)

// Warn if slow
if (activationTime > 500) {
  outputChannel.appendLine(`Warning: Activation exceeded 500ms target`)
}
```

### Auto-Detection of Existing Database

On activation, check for default SQLite database:

```typescript
// In activation flow for SQLite mode
if (dbConfig.type === 'sqlite' && dbConfig.path) {
  const defaultPath = path.join(homedir(), '.maproom', 'maproom.db')
  if (dbConfig.path === defaultPath && existsSync(defaultPath)) {
    outputChannel.appendLine(`Maproom: Found existing database at ${defaultPath}`)
  }
}
```

## Implementation Notes

### Preserve Backward Compatibility

The `postgres` config in `OrchestratorConfig` remains:
- Existing code using `config.postgres` still works
- `databaseUrlOverride` takes precedence when provided
- No breaking changes to ProcessOrchestrator interface

### Status Bar Codicons

VSCode built-in codicons used:
- `$(database)` - Database icon
- See: https://code.visualstudio.com/api/references/icons-in-labels

### Click Command

Use standard settings command:
```typescript
dbStatusItem.command = {
  command: 'workbench.action.openSettings',
  arguments: ['maproom.database']  // Opens settings filtered to maproom.database
}
```

### Error States

If database unavailable, status bar should indicate:
```typescript
if (!available) {
  dbStatusItem.text = `$(warning) No Database`
  dbStatusItem.backgroundColor = new vscode.ThemeColor('statusBarItem.warningBackground')
}
```

## Dependencies

- **VSCODEDB-1001**: `database-checker.ts` must be complete
- **VSCODEDB-1002**: Settings schema must be complete
- **VSCODEDB-1003**: Docker conditional logic must be complete

## Risk Assessment

- **Risk**: Status bar conflicts with existing items
  - **Mitigation**: Use low priority (100) and test with existing extension state

- **Risk**: Activation time regression
  - **Mitigation**: Measure and log activation time, set 500ms threshold

- **Risk**: Breaking ProcessOrchestrator
  - **Mitigation**: `databaseUrlOverride` is existing field with existing precedence logic

## Files/Packages Affected

### Modified Files
- `packages/vscode-maproom/src/extension.ts` (main changes)
- `packages/vscode-maproom/src/ui/statusBar.ts` (optional - if integrating there)

### Test Files
- `packages/vscode-maproom/src/extension.test.ts` (add activation tests)

## Testing

### Unit Tests

```typescript
describe('activation performance', () => {
  it('activates in <500ms for sqlite mode', async () => {
    // Mock SQLite file exists
    mockSettings['maproom.database.provider'] = 'sqlite'
    vi.mocked(existsSync).mockReturnValue(true)

    const startTime = performance.now()
    await activate(mockContext)
    const elapsed = performance.now() - startTime

    expect(elapsed).toBeLessThan(500)
  })
})

describe('ProcessOrchestrator configuration', () => {
  it('passes sqlite URL via databaseUrlOverride', () => {
    mockSettings['maproom.database.provider'] = 'sqlite'

    // Verify orchestrator created with correct URL
    expect(orchestratorConfig.databaseUrlOverride).toMatch(/^sqlite:\/\//)
  })

  it('passes postgresql URL via databaseUrlOverride', () => {
    mockSettings['maproom.database.provider'] = 'postgres'

    expect(orchestratorConfig.databaseUrlOverride).toMatch(/^postgresql:\/\//)
  })
})

describe('status bar', () => {
  it('shows SQLite mode indicator', () => {
    mockSettings['maproom.database.provider'] = 'sqlite'
    // Verify status bar text
  })

  it('shows PostgreSQL mode indicator', () => {
    mockSettings['maproom.database.provider'] = 'postgres'
    // Verify status bar text
  })
})
```

## Verification Checklist

After implementation:
1. [ ] Run `pnpm test` - all tests pass
2. [ ] Run `pnpm compile` - no TypeScript errors
3. [ ] Run `pnpm vsce:package` - VSIX builds
4. [ ] Manual test: SQLite mode activates without Docker prompts
5. [ ] Manual test: Status bar shows correct mode
6. [ ] Manual test: Activation time logged in Output channel
7. [ ] Manual test: PostgreSQL mode still works (regression check)
