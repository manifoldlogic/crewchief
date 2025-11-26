# Ticket: HEADLS-1001: Define TerminalProvider Interface

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - N/A (interface definition only)
- [x] **Verified** - interface implemented and used by all providers

## Agents
- TypeScript Engineer
- verify-ticket
- commit-ticket

## Summary
Create the `TerminalProvider` interface and associated types to abstract terminal operations.

## Background
Currently, `packages/cli` is tightly coupled to iTerm2 via `src/iterm/`. To support headless/CI environments, we need to abstract terminal operations behind a common interface.

## Acceptance Criteria
- [x] `TerminalProvider` interface is defined in `packages/cli/src/terminal/interface.ts`
- [x] Interface includes `id`, `initialize`, `dispose`, `createWindow`, `createTab`, `splitPane`, `runCommand`, `focus`
- [x] Types for `WindowOptions` and `SplitDirection` are defined
- [x] No implementation details (like iTerm specific types) leak into the interface

## Technical Requirements
- **File Path**: `packages/cli/src/terminal/interface.ts`
- **Methods**:
  - `initialize(): Promise<void>`
  - `dispose(): Promise<void>`
  - `createWindow(options?: WindowOptions): Promise<string>`
  - `createTab(windowId: string): Promise<string>`
  - `splitPane(targetId: string, direction: SplitDirection): Promise<string>`
  - `runCommand(paneId: string, command: string): Promise<void>`
  - `focus(paneId: string): Promise<void>`

## Implementation Notes
- Interface is generic and provider-agnostic
- `id` property identifies the provider type ('iterm', 'headless', 'mock')

## Dependencies
- None

## Risk Assessment
- **Risk**: Interface might miss methods needed by Orchestrator
  - **Mitigation**: Reviewed `runManager.ts` calls to ensure coverage

## Files/Packages Affected
- `packages/cli/src/terminal/interface.ts` (created)
