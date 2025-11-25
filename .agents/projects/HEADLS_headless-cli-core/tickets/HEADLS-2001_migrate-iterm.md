# Ticket: Migrate iTerm Logic to ITermProvider

**ID:** HEADLS-2001
**Phase:** 2
**Status:** Pending
**Assigned To:** TypeScript Engineer

## Summary
Refactor the existing `packages/cli/src/iterm/` logic into a class `ITermProvider` that implements `TerminalProvider`.

## Background
The current iTerm logic is scattered in `src/iterm`. We need to encapsulate it into the new provider structure while preserving exact behavior.

## Acceptance Criteria
- [ ] `ITermProvider` implemented in `packages/cli/src/terminal/providers/iterm.ts`.
- [ ] Existing JXA/AppleScript logic from `src/iterm/` is moved/called by this provider.
- [ ] `createWindow`, `splitPane`, `runCommand` work exactly as they do now on macOS.
- [ ] `initialize` checks for `TERM_PROGRAM` and throws if not iTerm (double safety).

## Technical Requirements
- **File Path**: `packages/cli/src/terminal/providers/iterm.ts`
- **Migration**: You may keep `src/iterm/*.ts` as helper files if they are complex, but the *entry point* for the rest of the app must be the Provider.
- **Refactor**: Ideally, inline the logic if it's small enough, or keep it as a utility module `src/terminal/utils/iterm-bridge.ts`.

## Implementation Notes
- Ensure `runCommand` handles the specific escaping required for AppleScript.

## Dependencies
- HEADLS-1001

## Risks
- Regressions in iTerm automation. Manual verification required.

