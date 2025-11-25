# Ticket: Update CLI Entry Point

**ID:** HEADLS-3002
**Phase:** 3
**Status:** Pending
**Assigned To:** TypeScript Engineer

## Summary
Update `packages/cli/src/cli/index.ts` to remove the hard `iTerm.app` check and use `TerminalFactory`.

## Background
The entry point guards prevent running in non-iTerm environments. We need to remove these and let the Factory handle detection.

## Acceptance Criteria
- [ ] Remove `if (process.env.TERM_PROGRAM !== 'iTerm.app')` check in `index.ts`.
- [ ] Instantiate `TerminalFactory` and get the provider.
- [ ] Pass the provider to the `Orchestrator`/`RunManager`.
- [ ] Verify `--headless` flag is respected (via `HEADLS-1003` logic).

## Technical Requirements
- **File Path**: `packages/cli/src/cli/index.ts`
- **Cleanup**: Remove legacy error messages about iTerm requirements.

## Implementation Notes
- This enables the feature for end users.

## Dependencies
- HEADLS-3001
- HEADLS-1003

## Risks
- None.

