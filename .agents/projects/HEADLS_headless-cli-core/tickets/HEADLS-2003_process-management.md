# Ticket: Implement Headless Process Management

**ID:** HEADLS-2003
**Phase:** 2
**Status:** Pending
**Assigned To:** TypeScript Engineer

## Summary
Add robust lifecycle management to `HeadlessProvider` to prevent zombie processes. Implement signal handling and recursive killing.

## Background
When the main CLI process exits (cleanly or via crash), all background agent processes must be terminated. Node's default behavior might leave them running.

## Acceptance Criteria
- [ ] `dispose()` method in `HeadlessProvider` kills all tracked child processes.
- [ ] `SIGINT` and `SIGTERM` listeners on the main process trigger `dispose()`.
- [ ] Recursive killing is implemented (killing the shell and its children).
- [ ] Logs are prefixed with `[AgentName]` (if available) or `[PaneID]` for readability.

## Technical Requirements
- **Library**: Consider `tree-kill` or `execa`'s built-in cleanup options.
- **Signal Handling**:
  ```typescript
  process.on('SIGINT', async () => {
    await this.dispose();
    process.exit(0);
  });
  ```

## Implementation Notes
- Test this by spawning a long-running process (like `sleep 100`) via the provider and killing the main CLI. Verify `sleep` is gone.

## Dependencies
- HEADLS-2002

## Risks
- Signal handlers might conflict with other CLI libraries (Commander). Ensure they play nicely.

