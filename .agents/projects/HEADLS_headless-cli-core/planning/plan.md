# Execution Plan: Headless CLI Core

## Phase 1: Interface & Factory (Tickets 1-3)
1.  **Define Interface**: Create `TerminalProvider` interface and types.
2.  **Implement Mock**: Create `MockProvider` for testing.
3.  **Implement Factory**: Create `TerminalFactory` with auto-detection logic.

## Phase 2: Provider Implementation (Tickets 4-6)
4.  **Migrate iTerm**: Refactor existing `src/iterm/` code into `ITermProvider` class.
5.  **Implement Headless**: Create `HeadlessProvider` using Node `child_process`.
6.  **Process Management**: Implement basic signal handling and cleanup for Headless mode.

## Phase 3: Integration & Cleanup (Tickets 7-9)
7.  **Orchestrator Update**: Update `RunManager` to use the injected provider instead of static imports.
8.  **CLI Entry Point**: Update `index.ts` to remove hardcoded checks and use the Factory.
9.  **Validation**: Run manual regression tests on macOS and smoke tests on Linux (Docker).

## Agent Assignments
- **TypeScript Engineer**: All implementation tickets.
- **Quality Engineer**: Creation of integration tests using MockProvider.

## Success Definition
- `crewchief --headless` runs successfully.
- Existing iTerm functionality is preserved.
- Codebase has 0 references to `src/iterm/` from outside `src/terminal/providers/iterm.ts`.

