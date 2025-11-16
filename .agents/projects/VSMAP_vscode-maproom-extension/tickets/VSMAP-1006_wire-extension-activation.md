# Ticket: VSMAP-1006: Wire extension activation to start all services

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - tests executed and passing (or N/A if no tests)
- [ ] **Verified** - by the verify-ticket agent

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
Connect all Phase 1 components in the extension's `activate()` function. Start Docker, spawn processes, show status bar. Orchestrate the complete extension lifecycle.

## Background
The individual pieces (DockerManager, ProcessOrchestrator, StatusBarManager) are complete. Now we need to orchestrate them in the extension lifecycle, ensuring proper startup sequence and error handling.

This ticket completes **Phase 1: Core Infrastructure** of the VSMAP project plan by integrating all components into a working extension that activates and spawns watch processes.

## Acceptance Criteria
- [ ] Extension activates on workspace open (activation event: `onStartupFinished`)
- [ ] `activate()` completes in <500ms (fast activation requirement)
- [ ] Docker services start asynchronously (don't block activation)
- [ ] Watch processes spawn after Docker healthy
- [ ] Status bar shows progress during startup
- [ ] Error handling: if Docker fails, show error and stop gracefully
- [ ] `deactivate()` stops all services and processes cleanly

## Technical Requirements
- Activation event in package.json: `"onStartupFinished"`
- `activate()` function in `src/extension.ts`
- Start Docker asynchronously: `dockerManager.ensureServicesRunning()` in background
- Show status bar immediately (loading state)
- Spawn processes after Docker healthy (await health checks)
- Handle errors gracefully (log, show notification, set status to error)
- `deactivate()` calls `dockerManager.stop()` and `orchestrator.stopWatching()`
- Register all disposables in `context.subscriptions` (auto-cleanup)
- Add logging to Output panel for debugging activation

## Implementation Notes
- Don't await Docker startup in activate() (violates <500ms requirement)
- Use background task with VSCode progress notification:
  ```typescript
  vscode.window.withProgress({
    location: vscode.ProgressLocation.Notification,
    title: "Starting Maproom services..."
  }, async (progress) => { ... });
  ```
- Store manager instances in extension context for cleanup
- Sequence:
  1. Create Output channel
  2. Create StatusBarManager (show "Starting...")
  3. Create DockerManager
  4. Return from activate() (fast!)
  5. Background: Start Docker
  6. Background: Wait for health checks
  7. Background: Create ProcessOrchestrator
  8. Background: Start watch processes
  9. Background: Update StatusBar to "Watching"
- Error recovery: if any step fails, show error, update status bar to error state, don't crash
- Test activation manually (no automated tests for this ticket - integration test)

## Dependencies
- VSMAP-1001 (DockerManager)
- VSMAP-1003 (ProcessOrchestrator)
- VSMAP-1005 (StatusBarManager)

## Risk Assessment
- **Risk**: Activation may exceed 500ms if blocking on Docker
  - **Mitigation**: Asynchronous startup, immediate status bar, return quickly from activate()
- **Risk**: Background task may fail silently
  - **Mitigation**: Comprehensive error handling, log all errors, show notifications
- **Risk**: Extension may activate without workspace
  - **Mitigation**: Check for workspace folder, show error if none

## Files/Packages Affected
- `src/extension.ts` (create/modify, ~100 lines)
- `package.json` (modify, add activation events and commands)
- Manual testing checklist (document in ticket comments after implementation)
