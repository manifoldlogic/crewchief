# Ticket: HEADLS-2001: Migrate iTerm Logic to ITermProvider

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - N/A (wrapper around existing ITermService)
- [x] **Verified** - ITermProvider wraps ITermService correctly

## Agents
- TypeScript Engineer
- verify-ticket
- commit-ticket

## Summary
Create `ITermProvider` that wraps the existing `ITermService` to implement `TerminalProvider`.

## Background
The existing iTerm integration in `src/iterm/` is battle-tested. Rather than rewriting, we wrap it in a provider that implements the new interface.

## Acceptance Criteria
- [x] `ITermProvider` implemented in `packages/cli/src/terminal/providers/iterm.ts`
- [x] Wraps existing `ITermService` from `../../iterm/iterm.service`
- [x] `initialize()` validates iTerm.app environment and starts bridge
- [x] `dispose()` stops the bridge
- [x] All interface methods delegate to ITermService

## Technical Requirements
- **File Path**: `packages/cli/src/terminal/providers/iterm.ts`
- **Validation**: Throws if `TERM_PROGRAM !== 'iTerm.app'`
- **Method Mapping**:
  - `createWindow` → `service.createNamedWindow/createWindowWithCwd/createWindow`
  - `splitPane` → `service.focusSession` + `service.createPane`
  - `runCommand` → `service.sendLine`
  - `focus` → `service.focusSession`

## Implementation Notes
- `createTab` falls back to `createWindow` with a warning (iTerm RPC limitation)
- Provider is a thin adapter layer

## Dependencies
- HEADLS-1001 (TerminalProvider interface)
- Existing `src/iterm/iterm.service.ts`

## Risk Assessment
- **Risk**: ITermService API changes break provider
  - **Mitigation**: Provider is a thin wrapper, changes propagate easily

## Files/Packages Affected
- `packages/cli/src/terminal/providers/iterm.ts` (created)
