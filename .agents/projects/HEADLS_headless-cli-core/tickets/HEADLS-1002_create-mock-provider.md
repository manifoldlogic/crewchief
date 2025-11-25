# Ticket: Create MockProvider

**ID:** HEADLS-1002
**Phase:** 1
**Status:** Pending
**Assigned To:** TypeScript Engineer

## Summary
Implement a `MockProvider` that adheres to the `TerminalProvider` interface. This provider will be used for unit testing the orchestrator and factory logic without side effects.

## Background
Testing the orchestrator currently requires a real terminal environment or complex mocking of the iTerm service. A dedicated `MockProvider` allows us to record command executions and verify logic in isolation.

## Acceptance Criteria
- [ ] `MockProvider` class implemented in `packages/cli/src/terminal/providers/mock.ts`.
- [ ] All interface methods are implemented as no-ops or state recorders.
- [ ] `runCommand` stores executed commands in a public array `executedCommands` for assertion.
- [ ] `createWindow`/`splitPane` return deterministic UUIDs (or simple incrementing IDs).

## Technical Requirements
- **File Path**: `packages/cli/src/terminal/providers/mock.ts`
- **State**:
  - `executedCommands: { paneId: string; command: string }[]`
  - `windows: string[]`
  - `panes: Record<string, { parent: string }>`
- **Behavior**:
  - `initialize`/`dispose` should log debug messages.
  - `splitPane` should logically track the hierarchy (optional, but helpful for advanced tests).

## Implementation Notes
- Use this provider to verify that the Interface from HEADLS-1001 is implementable.

## Dependencies
- HEADLS-1001

## Risks
- Mock might be too simple and miss async race conditions present in real providers.

