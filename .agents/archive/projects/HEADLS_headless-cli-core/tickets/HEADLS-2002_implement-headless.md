# Ticket: HEADLS-2002: Implement HeadlessProvider

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - N/A (manual smoke testing required)
- [x] **Verified** - HeadlessProvider spawns and manages processes

## Agents
- TypeScript Engineer
- verify-ticket
- commit-ticket

## Summary
Implement `HeadlessProvider` using Node's `child_process` for background execution without UI.

## Background
For Linux/CI support, we need a provider that executes "panes" as background processes. It handles input/output streaming but ignores layout requests.

## Acceptance Criteria
- [x] `HeadlessProvider` implemented in `packages/cli/src/terminal/providers/headless.ts`
- [x] `createWindow`/`splitPane` generate logical IDs without spawning processes
- [x] `runCommand` spawns a child process using `child_process.spawn`
- [x] Logs from child processes are piped to main process stdout with pane ID prefix
- [x] Layout requests (`createTab`, `splitPane`) return IDs but do no UI work

## Technical Requirements
- **File Path**: `packages/cli/src/terminal/providers/headless.ts`
- **Spawning**: Uses `child_process.spawn` with `shell: true`
- **Output**: Streams stdout/stderr with `[paneId]` prefix via logger
- **Process Tracking**: `Map<string, ChildProcess>` for cleanup

## Implementation Notes
- Window IDs include timestamp for uniqueness
- Pane IDs use incremental counter
- stdio set to 'pipe' for output capture

## Dependencies
- HEADLS-1001 (TerminalProvider interface)

## Risk Assessment
- **Risk**: Zombie processes if cleanup fails
  - **Mitigation**: See HEADLS-2003 for process management

## Files/Packages Affected
- `packages/cli/src/terminal/providers/headless.ts` (created)
