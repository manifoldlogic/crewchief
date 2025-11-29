# Ticket: HEADLS-2003: Implement Headless Process Management

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - N/A (process management is runtime behavior)
- [x] **Verified** - Signal handlers and tree-kill implemented

## Agents
- TypeScript Engineer
- verify-ticket
- commit-ticket

## Summary
Implement robust process lifecycle management for HeadlessProvider to prevent zombie processes.

## Background
Child processes must be properly cleaned up on exit, signal interrupts, and errors. Without proper management, zombie processes accumulate.

## Acceptance Criteria
- [x] Uses `tree-kill` package for killing process trees
- [x] Signal handlers registered for SIGINT, SIGTERM, exit
- [x] `dispose()` kills all tracked processes
- [x] Process exit events clean up tracking map
- [x] Error events are logged without crashing

## Technical Requirements
- **Package**: `tree-kill` for recursive process termination
- **Signals**: SIGINT, SIGTERM, exit handlers call `dispose()`
- **Cleanup**: `treeKill(pid, 'SIGTERM')` for graceful shutdown
- **Tracking**: Processes removed from map on 'exit' event

## Implementation Notes
- `handleSignal()` is bound to class instance
- Promises collected for parallel cleanup in `dispose()`
- Non-blocking cleanup (resolve even on kill errors)

## Dependencies
- HEADLS-2002 (HeadlessProvider base implementation)

## Risk Assessment
- **Risk**: Signal handler race conditions
  - **Mitigation**: dispose() is idempotent, clears map after cleanup

## Files/Packages Affected
- `packages/cli/src/terminal/providers/headless.ts` (modified)
- `package.json` (tree-kill dependency)
