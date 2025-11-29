# Ticket: VSCEXT-2002: Update StatusBarManager integration

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - tests executed and passing
- [x] **Verified** - by the verify-ticket agent

## Agents
- vscode-extension-specialist
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary
Connect the StatusBarManager to the refactored ProcessOrchestrator to display branch information from `branch_switched` events and show reconciliation status during startup.

## Background
With the unified watch process now emitting `branch_switched` events, the status bar can display the current branch. Additionally, the status bar should show "Reconciling..." during startup reconciliation phase.

Reference: planning/plan.md - Phase 2, Ticket 2002
Reference: planning/architecture.md - NDJSON Event Types

## Acceptance Criteria
- [x] Status bar shows current branch name from `branch_switched` events
- [x] Status bar shows "Reconciling..." during startup reconciliation
- [x] Status bar transitions to "Watching" after ready
- [x] Error states display correctly with appropriate icons
- [x] Existing state machine (starting → watching → error) preserved

## Technical Requirements
- Handle `branch_switched` events to update branch display
- Add "reconciling" state to status bar state machine
- Keep existing states: starting, watching, error
- Use appropriate icons for each state
- Format: `$(icon) Maproom: [state] [branch]`

**State Transitions**:
```
starting → reconciling → watching
    ↓           ↓           ↓
  error       error       error
```

**Display Examples**:
- `$(sync~spin) Maproom: Starting...`
- `$(sync~spin) Maproom: Reconciling...`
- `$(eye) Maproom: Watching (main)`
- `$(error) Maproom: Error`

## Implementation Notes
1. Review existing StatusBarManager implementation
2. Add `reconciling` state to the state enum/type
3. Add `setBranch(branchName: string)` method
4. Subscribe to orchestrator events for branch updates
5. Ensure branch display persists across state changes

```typescript
// Example integration in extension.ts
orchestrator.on('branch_switched', (event: BranchSwitchedEvent) => {
  statusBar.setBranch(event.new_branch)
})

statusBar.setState('reconciling')
await reconcileChanges(context)
statusBar.setState('watching')
```

## Dependencies
- VSCEXT-2001 (Refactored ProcessOrchestrator emitting branch_switched events)

## Risk Assessment
- **Risk**: Breaking existing status bar functionality
  - **Mitigation**: Preserve existing state machine, add new states additively
- **Risk**: Branch name too long for status bar
  - **Mitigation**: Truncate long branch names with ellipsis

## Files/Packages Affected
- `packages/vscode-maproom/src/ui/statusBar.ts` - Add reconciling state, branch display
- `packages/vscode-maproom/src/ui/statusBar.test.ts` - Update tests
