# Ticket: VSMAP-1005: Implement StatusBarManager for UI updates

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
- vscode-extension-specialist
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary
Create `StatusBarManager` to display indexing status in VSCode status bar. Update text and tooltip based on watch process events.

## Background
Users need visibility into indexing status. The status bar shows current state (watching, indexing N files, error) and updates in real-time based on parsed events. This is the primary UI element for user feedback.

This ticket implements **Milestone 1.3: Status Bar** from Phase 1 of the VSMAP project plan, completing the core infrastructure by providing user visibility into the indexing system.

## Acceptance Criteria
- [x] StatusBarItem created and visible after activation
- [x] Text updates on watch events:
  - Idle: "$(database) Maproom Ready"
  - Watching: "$(eye) Watching..."
  - Indexing: "$(sync~spin) Indexing 15 files..."
  - Error: "$(error) Maproom Error"
- [x] Tooltip shows detailed info (last indexed time, file counts)
- [x] Clicking status bar shows Output panel
- [x] Status bar hidden on deactivation

## Technical Requirements
- Use `vscode.window.createStatusBarItem(alignment, priority)`
- Subscribe to `StdoutParser` events
- Update text on `progress` events (show file count)
- Update text on `status` events (show state)
- Update text on `error` events (show error icon)
- Set tooltip with detailed info
- Set command: `maproom.showOutput` (opens Output panel)
- Use VSCode icons: `$(database)`, `$(eye)`, `$(sync~spin)`, `$(error)`
- StatusBarAlignment: Right
- Priority: Lower than built-in items (e.g., 100)

## Implementation Notes
- Status bar should be non-intrusive (right side, low priority)
- Don't update too frequently (debounce if >10 updates/sec)
- Store last indexed timestamp in workspace state (use `context.workspaceState`)
- Clear status on deactivation (dispose StatusBarItem)
- Register disposable in extension context: `context.subscriptions.push(statusBar)`
- Command registration: register `maproom.showOutput` in extension.ts
- Tooltip should include: last indexed time, total files indexed, current state
- Use human-friendly time format (e.g., "2 minutes ago")

## Dependencies
- VSMAP-1004 (StdoutParser provides events)

## Risk Assessment
- **Risk**: Frequent updates may cause flicker
  - **Mitigation**: Debounce updates (max 1/sec), batch state changes
- **Risk**: Status bar may be hidden on some window layouts
  - **Mitigation**: Document in extension README, consider adding command to show
- **Risk**: Tooltip may be too verbose
  - **Mitigation**: Keep concise, most important info first

## Files/Packages Affected
- `src/ui/statusBar.ts` (create, ~100 lines)
- `src/ui/statusBar.test.ts` (create, ~80 lines - unit tests with mocks)
- `src/extension.ts` (modify to register statusBar and command)
- `package.json` (add command registration for `maproom.showOutput`)
