# Ticket: VSMAP-2003: Implement initial scan with progress notification

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - tests executed and passing (or N/A if no tests)
- [x] **Verified** - by the verify-ticket agent

**Note on "Tests pass"**:
- If tests were created/modified, you MUST run them and show output
- "Tests pass" means tests were EXECUTED and all passed
- "Tests pass - N/A" is only valid for documentation-only tickets
- Test file existence alone does NOT satisfy this requirement

## Agents
- process-management-specialist
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary
Trigger `crewchief-maproom scan` after setup wizard completes. Parse progress output and show VSCode progress notification with file counts and percentage.

## Background
This completes Phase 2 (Setup Wizard) of the VSMAP plan. After users select their provider (VSMAP-2001) and configure credentials (VSMAP-2002), the extension should immediately scan the workspace to build the initial semantic index. This provides a smooth onboarding experience where users see immediate progress and know when indexing is complete.

The Rust binary emits NDJSON progress events which we parse and display in a VSCode progress notification.

Reference: VSMAP_PLAN.md Phase 2 "Setup Wizard - Initial Scan"

## Acceptance Criteria
- [x] Scan process spawns automatically after wizard completes successfully
- [x] Progress notification shows 0-100% with file counts
- [x] Notification is dismissible and scan continues in background
- [x] Status bar updates to "Indexed" on completion with file count
- [x] Error shown if scan fails with actionable error message
- [x] Users can see detailed progress in Output channel

## Technical Requirements
- Spawn command: `crewchief-maproom scan --path <workspace>`
- Use `vscode.window.withProgress()` for cancellable progress notification
- Parse stdout for progress events: `{ type: 'progress', percent: 45, files: 1500 }`
- Update progress: `progress.report({ increment: 5, message: 'Indexed 1500 files' })`
- Progress notification options: `{ location: vscode.ProgressLocation.Notification, title: 'Indexing workspace', cancellable: false }`
- On completion, update status bar via StatusBarManager

## Implementation Notes
The scan module should orchestrate:
1. Spawn the scan process using BinarySpawner
2. Create progress notification with `withProgress()`
3. Parse stdout using StdoutParser (VSMAP-1004)
4. Update progress notification as events arrive
5. Handle completion and errors

Progress calculation:
- Store last reported percentage
- Calculate increment: `currentPercent - lastPercent`
- Update message with file count: `"Indexed 1,500 files"`

Error handling:
- If binary exits with non-zero code, show error notification
- Parse stderr for error details
- Suggest actions: "Check Output channel for details"

Completion handling:
- Close progress notification
- Update status bar to "Indexed: 1,500 files"
- Store completion time in workspace state
- Log summary to Output channel

Integration with setup wizard:
```typescript
// In setupWizard.ts after credential setup
await scanWorkspace(context, workspaceRoot);
```

## Dependencies
- VSMAP-2002 (credential storage) must be complete
- VSMAP-1003 (binary spawner) for process management
- VSMAP-1004 (stdout parser) for parsing progress events
- VSMAP-1005 (status bar manager) for status updates

## Risk Assessment
- **Risk**: Scan may take 5-10 minutes on large codebases, blocking user
  - **Mitigation**: Make notification dismissible, scan runs in background
- **Risk**: Users may close VSCode during scan
  - **Mitigation**: Log progress to workspace state, resume on next activation (future enhancement)
- **Risk**: Binary may crash during scan
  - **Mitigation**: Show error notification with actionable guidance, reference VSMAP-3002 crash recovery

## Files/Packages Affected
- `src/process/scan.ts` (new file, ~120 lines)
- `src/ui/setupWizard.ts` (integrate scan trigger)
- `src/test/scan.test.ts` (new test file)
