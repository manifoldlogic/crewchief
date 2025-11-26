# Ticket: HEADLS-3001: Update Orchestrator to Use TerminalProvider

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - N/A (integration change)
- [x] **Verified** - Scheduler accepts TerminalProvider in constructor

## Agents
- TypeScript Engineer
- verify-ticket
- commit-ticket

## Summary
Update the `Scheduler` to use injected `TerminalProvider` instead of direct iTerm imports.

## Background
The core orchestration logic had hard dependencies on iTerm service. Dependency injection enables provider swapping.

## Acceptance Criteria
- [x] `Scheduler` constructor accepts `TerminalProvider` parameter
- [x] All terminal operations use the injected provider
- [x] No direct imports of `src/iterm` in `src/orchestrator/scheduler.ts`

## Technical Requirements
- **Refactor**: Constructor injection pattern
- **Type**: `private terminal: TerminalProvider`
- **Import**: `TerminalProvider` from `../terminal/interface`

## Implementation Notes
- Provider instance passed from CLI entry point
- Enables testing with MockProvider

## Dependencies
- HEADLS-1001 (TerminalProvider interface)
- HEADLS-2001 (ITermProvider)
- HEADLS-2002 (HeadlessProvider)

## Risk Assessment
- **Risk**: Breaking orchestration flow
  - **Mitigation**: Interface matches existing ITermService API

## Files/Packages Affected
- `packages/cli/src/orchestrator/scheduler.ts` (modified)
