# Ticket: Implement HeadlessProvider

**ID:** HEADLS-2002
**Phase:** 2
**Status:** Pending
**Assigned To:** TypeScript Engineer

## Summary
Implement the `HeadlessProvider` using Node's `child_process` (or `execa`). This provider runs commands in the background without a UI.

## Background
For Linux/CI support, we need a provider that executes "panes" as background processes. It must handle input/output streaming but ignore layout requests.

## Acceptance Criteria
- [ ] `HeadlessProvider` implemented in `packages/cli/src/terminal/providers/headless.ts`.
- [ ] `createWindow`/`splitPane` generate a logical ID but do NOT spawn processes or windows.
- [ ] `runCommand` spawns a child process.
- [ ] Logs from child processes are piped to the main process stdout.
- [ ] Layout requests (`createTab`, `splitPane`) are gracefully ignored (return IDs but do no UI work).

## Technical Requirements
- **File Path**: `packages/cli/src/terminal/providers/headless.ts`
- **Spawning**: Use `child_process.spawn` or `execa` (recommended).
- **Output**: Stream `stdout` and `stderr` of child to parent, prefixed with `[PaneID]`.

## Implementation Notes
- Use a `Map<string, ChildProcess>` to track running processes.

## Dependencies
- HEADLS-1001

## Risks
- Zombie processes (covered in HEADLS-2003).

