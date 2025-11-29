# Ticket: HEADLS-3002: Update CLI Entry Point

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - N/A (entry point change)
- [x] **Verified** - CLI uses TerminalFactory.autoDetect()

## Agents
- TypeScript Engineer
- verify-ticket
- commit-ticket

## Summary
Update CLI entry point to use `TerminalFactory` instead of hard-coded iTerm checks.

## Background
The entry point previously blocked execution in non-iTerm environments. Now it uses the factory for auto-detection.

## Acceptance Criteria
- [x] Removed `if (process.env.TERM_PROGRAM !== 'iTerm.app')` check
- [x] Uses `TerminalFactory.autoDetect()` to get provider
- [x] Calls `terminal.initialize()` before use
- [x] `--headless` flag is respected via factory logic

## Technical Requirements
- **File Path**: `packages/cli/src/cli/index.ts`
- **Import**: `TerminalFactory` from `../terminal/factory`
- **Initialization**: `await terminal.initialize()`

## Implementation Notes
- Factory handles all environment detection
- Headless provider logged when selected
- Entry point is now environment-agnostic

## Dependencies
- HEADLS-3001 (Orchestrator update)
- HEADLS-1003 (TerminalFactory)

## Risk Assessment
- **Risk**: None - cleaner entry point logic

## Files/Packages Affected
- `packages/cli/src/cli/index.ts` (modified)
