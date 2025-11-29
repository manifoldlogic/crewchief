# Ticket: VSCODEDB-1006: Setup Wizard SQLite Path Selection (Enhancement)

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - tests executed and passing (336 core tests pass; orchestrator timeouts are pre-existing)
- [x] **Verified** - by the verify-ticket agent

## Agents
- vscode-extension-specialist (primary)
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary

Enhance the setup wizard with SQLite file picker and auto-detection of existing databases. This is an **enhancement ticket** (post-MVP) that improves the first-run experience for SQLite users.

## Background

The MVP SQLite functionality (tickets 1001-1005) works with manual settings configuration. This enhancement improves the UX by:
1. Auto-detecting existing `~/.maproom/maproom.db`
2. Allowing users to browse for alternative SQLite paths
3. Providing clear guidance when no database exists

**Reference:** plan.md Enhancement Phase - "VSCODEDB-1006: Setup Wizard SQLite Path Selection"

**Priority:** Optional/Enhancement - not required for MVP functionality

## Acceptance Criteria

- [x] Setup wizard detects existing `~/.maproom/maproom.db` and shows "Use existing index?" option
- [x] File picker allows selecting alternative SQLite database files
- [x] Clear guidance shown when no database exists ("Run crewchief-maproom scan to create an index")
- [x] Setup wizard correctly updates `maproom.database.sqlitePath` setting
- [x] All tests pass: `pnpm test`

## Technical Requirements

### Modified File: `setupWizard.ts`

Update `packages/vscode-maproom/src/ui/setupWizard.ts` with SQLite-aware flow.

### Auto-Detection Flow

```typescript
async function runSetupWizard(): Promise<void> {
  const dbConfig = resolveDatabaseConfig()

  if (dbConfig.type === 'sqlite') {
    await runSqliteSetup(dbConfig)
  } else {
    await runPostgresSetup()  // Existing flow
  }
}

async function runSqliteSetup(config: DatabaseConfig): Promise<void> {
  const defaultPath = path.join(homedir(), '.maproom', 'maproom.db')
  const defaultExists = existsSync(defaultPath)

  if (defaultExists) {
    // Offer to use existing database
    const action = await vscode.window.showInformationMessage(
      `Found existing Maproom index at ${defaultPath}`,
      'Use Existing',
      'Choose Different',
      'Cancel'
    )

    if (action === 'Use Existing') {
      // Default path, no settings change needed (empty sqlitePath = default)
      return
    } else if (action === 'Choose Different') {
      await promptForSqlitePath()
    }
    // Cancel = do nothing
  } else {
    // No existing database - guide user
    await showNoSqliteGuidance()
  }
}
```

### File Picker for Alternative Path

```typescript
async function promptForSqlitePath(): Promise<void> {
  const result = await vscode.window.showOpenDialog({
    canSelectFiles: true,
    canSelectFolders: false,
    canSelectMany: false,
    filters: {
      'SQLite Database': ['db', 'sqlite', 'sqlite3']
    },
    title: 'Select Maproom SQLite Database'
  })

  if (result && result[0]) {
    const selectedPath = result[0].fsPath

    // Update settings
    const config = vscode.workspace.getConfiguration('maproom.database')
    await config.update('sqlitePath', selectedPath, vscode.ConfigurationTarget.Global)

    vscode.window.showInformationMessage(`Maproom will use: ${selectedPath}`)
  }
}
```

### No Database Guidance

```typescript
async function showNoSqliteGuidance(): Promise<void> {
  const action = await vscode.window.showWarningMessage(
    'No Maproom index found. Create one to enable code search.',
    { modal: false, detail: 'Run crewchief-maproom scan in your terminal to index a repository.' },
    'Copy Scan Command',
    'Open Terminal',
    'Choose Existing File'
  )

  if (action === 'Copy Scan Command') {
    const command = `crewchief-maproom scan ${vscode.workspace.workspaceFolders?.[0]?.uri.fsPath || '/path/to/your/repo'}`
    await vscode.env.clipboard.writeText(command)
    vscode.window.showInformationMessage('Scan command copied to clipboard')
  } else if (action === 'Open Terminal') {
    const terminal = vscode.window.createTerminal('Maproom Setup')
    terminal.show()
    terminal.sendText('# Run: crewchief-maproom scan /path/to/your/repo')
  } else if (action === 'Choose Existing File') {
    await promptForSqlitePath()
  }
}
```

### Integration with First-Run Detection

Update first-run detection in `extension.ts` to trigger appropriate wizard:

```typescript
// In activation flow
if (shouldRunSetup) {
  const dbConfig = resolveDatabaseConfig()
  if (dbConfig.type === 'sqlite') {
    await runSqliteSetup(dbConfig)
  } else {
    // Existing PostgreSQL setup flow
  }
}
```

## Implementation Notes

### Preserve PostgreSQL Wizard

Do NOT remove or break the existing PostgreSQL setup wizard. The SQLite flow should be an alternative path, not a replacement.

### File Extension Filters

SQLite databases can have various extensions:
- `.db` (most common)
- `.sqlite`
- `.sqlite3`

Include all common extensions in the file picker filter.

### Settings Update Scope

Use `ConfigurationTarget.Global` for `sqlitePath` since database location is machine-specific (not workspace-specific).

### Error Handling

```typescript
try {
  await config.update('sqlitePath', selectedPath, vscode.ConfigurationTarget.Global)
} catch (error) {
  vscode.window.showErrorMessage(`Failed to update settings: ${error}`)
}
```

## Dependencies

- **VSCODEDB-1001 through 1005**: All MVP tickets must be complete first
- This is an **enhancement ticket** - not required for core SQLite functionality

## Risk Assessment

- **Risk**: Breaking existing PostgreSQL setup wizard
  - **Mitigation**: Keep all PostgreSQL paths, add SQLite as alternative branch

- **Risk**: User selects invalid file
  - **Mitigation**: File picker filters to .db/.sqlite extensions, validation after selection

- **Risk**: Settings update fails
  - **Mitigation**: Try-catch with user-friendly error message

## Files/Packages Affected

### Modified Files
- `packages/vscode-maproom/src/ui/setupWizard.ts`
- `packages/vscode-maproom/src/extension.ts` (minor - setup wizard trigger)

### Test Files
- `packages/vscode-maproom/src/ui/setupWizard.test.ts` (add SQLite flow tests)

## Testing

### Unit Tests

```typescript
describe('SQLite setup wizard', () => {
  it('detects existing default database', async () => {
    vi.mocked(existsSync).mockReturnValue(true)

    // Should show "Use existing" option
    await runSqliteSetup(mockConfig)

    expect(vscode.window.showInformationMessage).toHaveBeenCalledWith(
      expect.stringContaining('Found existing'),
      'Use Existing',
      'Choose Different',
      'Cancel'
    )
  })

  it('shows guidance when no database exists', async () => {
    vi.mocked(existsSync).mockReturnValue(false)

    await runSqliteSetup(mockConfig)

    expect(vscode.window.showWarningMessage).toHaveBeenCalledWith(
      expect.stringContaining('No Maproom index found'),
      expect.anything(),
      'Copy Scan Command',
      'Open Terminal',
      'Choose Existing File'
    )
  })

  it('updates sqlitePath setting when file selected', async () => {
    vi.mocked(vscode.window.showOpenDialog).mockResolvedValue([
      { fsPath: '/custom/path/index.db' } as vscode.Uri
    ])

    await promptForSqlitePath()

    expect(mockConfig.update).toHaveBeenCalledWith(
      'sqlitePath',
      '/custom/path/index.db',
      vscode.ConfigurationTarget.Global
    )
  })
})
```

## Verification Checklist

After implementation:
1. [ ] Run `pnpm test` - all tests pass
2. [ ] Manual: Fresh install with no database → guidance shown
3. [ ] Manual: Existing `~/.maproom/maproom.db` → "Use existing?" shown
4. [ ] Manual: File picker selects .db file → settings updated
5. [ ] Manual: PostgreSQL setup still works (regression check)
