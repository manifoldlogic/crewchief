# Ticket: Update Orchestrator to Use TerminalProvider

**ID:** HEADLS-3001
**Phase:** 3
**Status:** Pending
**Assigned To:** TypeScript Engineer

## Summary
Update the `Orchestrator` and `RunManager` to use the injected `TerminalProvider` instance instead of importing `src/iterm` directly.

## Background
The core logic currently has hard dependencies on the iTerm service. We need to swap this for the provider interface.

## Acceptance Criteria
- [ ] `src/orchestrator/runManager.ts` accepts `TerminalProvider` in constructor/init.
- [ ] All calls to `iterm.createWindow`, etc., are replaced with `provider.createWindow`.
- [ ] No direct imports of `src/iterm` remain in `src/orchestrator`.

## Technical Requirements
- **Refactor**: This is a "search and replace" refactor but requires careful type checking.
- **Initialization**: The provider instance should be passed down from the CLI entry point.

## Implementation Notes
- This is the "wiring" step that makes the previous tickets actually used.

## Dependencies
- HEADLS-1001
- HEADLS-2001
- HEADLS-2002

## Risks
- Breaking the orchestration flow if the interface doesn't perfectly match the old behavior.

