# Ticket: Define TerminalProvider Interface

**ID:** HEADLS-1001
**Phase:** 1
**Status:** Pending
**Assigned To:** TypeScript Engineer

## Summary
Create the `TerminalProvider` interface and associated types to abstract terminal operations. This interface will be the contract for all terminal implementations (iTerm, Headless, Mock).

## Background
Currently, `packages/cli` is tightly coupled to iTerm2 via `src/iterm/`. To support headless/CI environments, we need to abstract terminal operations behind a common interface.

## Acceptance Criteria
- [ ] `TerminalProvider` interface is defined in `packages/cli/src/terminal/interface.ts`.
- [ ] Interface includes `id`, `initialize`, `dispose`, `createWindow`, `createTab`, `splitPane`, `runCommand`, `focus`.
- [ ] Types for `WindowOptions` and `LayoutDef` are defined.
- [ ] No implementation details (like `iTerm` specific types) leak into the interface.

## Technical Requirements
- **File Path**: `packages/cli/src/terminal/interface.ts`
- **Methods**:
  - `initialize(): Promise<void>`
  - `dispose(): Promise<void>`
  - `createWindow(options?: WindowOptions): Promise<string>` (returns ID)
  - `createTab(windowId: string): Promise<string>` (returns ID)
  - `splitPane(targetId: string, direction: 'vertical' | 'horizontal'): Promise<string>` (returns ID)
  - `runCommand(paneId: string, command: string): Promise<void>`
  - `focus(paneId: string): Promise<void>`

## Implementation Notes
- Review `packages/cli/src/iterm/iterm.types.ts` to understand current requirements, but keep the new interface generic.
- The `id` property should be a readonly string identifier for the provider type.

## Dependencies
- None

## Risks
- Interface might miss a method needed by the complex Orchestrator logic. Review `src/orchestrator/runManager.ts` calls to `iterm` service to ensure coverage.

