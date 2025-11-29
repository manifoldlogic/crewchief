# Ticket: HEADLS-1002: Create MockProvider

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - N/A (provider is for testing purposes)
- [x] **Verified** - MockProvider implemented with full state tracking

## Agents
- TypeScript Engineer
- verify-ticket
- commit-ticket

## Summary
Create a `MockProvider` for unit testing that records all operations without side effects.

## Background
Testing the orchestrator requires a terminal provider that doesn't actually spawn processes or interact with terminals. The MockProvider enables deterministic testing.

## Acceptance Criteria
- [x] `MockProvider` implemented in `packages/cli/src/terminal/providers/mock.ts`
- [x] Implements `TerminalProvider` interface
- [x] Tracks created windows in `windows` array
- [x] Tracks created panes in `panes` record with parent relationships
- [x] Records executed commands in `executedCommands` array
- [x] State is reset on `dispose()`

## Technical Requirements
- **File Path**: `packages/cli/src/terminal/providers/mock.ts`
- **Public State**: `executedCommands`, `windows`, `panes` for test assertions
- **Validation**: Throws errors for invalid pane/window IDs

## Implementation Notes
- Windows automatically create an initial pane
- Panes track their parent window for hierarchy validation
- All methods validate that referenced IDs exist

## Dependencies
- HEADLS-1001 (TerminalProvider interface)

## Risk Assessment
- **Risk**: Mock behavior diverges from real providers
  - **Mitigation**: Mock validates ID existence like real providers would

## Files/Packages Affected
- `packages/cli/src/terminal/providers/mock.ts` (created)
