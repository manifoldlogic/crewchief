# Architecture: Headless CLI Core

## 1. Core Design Pattern: Strategy Pattern
We will use the Strategy Pattern to inject the appropriate `TerminalProvider` at runtime.

```typescript
interface TerminalProvider {
  id: string; // 'iterm' | 'headless' | 'mock'
  
  // Lifecycle
  initialize(): Promise<void>;
  dispose(): Promise<void>;

  // Layout
  createWindow(options?: WindowOptions): Promise<string>; // returns windowId
  createTab(windowId: string): Promise<string>; // returns tabId
  splitPane(targetId: string, direction: 'vertical' | 'horizontal'): Promise<string>; // returns paneId

  // Execution
  runCommand(paneId: string, command: string): Promise<void>;
  focus(paneId: string): Promise<void>;
}
```

## 2. Component Structure

```
packages/cli/src/
├── terminal/
│   ├── interface.ts       # The TerminalProvider interface
│   ├── factory.ts         # Detects env and returns provider instance
│   ├── providers/
│   │   ├── iterm.ts       # Refactored from src/iterm/
│   │   ├── headless.ts    # New: Child process manager
│   │   └── mock.ts        # New: No-op for tests
│   └── manager.ts         # High-level API used by Orchestrator
```

## 3. Headless Provider Implementation
The `HeadlessProvider` cannot split panes visually. It will map "panes" to **Background Processes**.

- `splitPane()`: Logic operation (creates a new logical context ID).
- `runCommand()`: Spawns `child_process.spawn(cmd, { detached: true, stdio: 'pipe' })`.
- **Logs**: Streams stdout/stderr from all "panes" to a single multiplexed output (prefixed with `[PaneID]`).

## 4. Integration
- **Entry Point (`src/cli/index.ts`)**:
  - Remove the hard check for `iTerm.app`.
  - Call `TerminalFactory.autoDetect()`.
  - Initialize `Orchestrator` with the detected provider.

## 5. Long-term Maintainability
- This abstraction allows adding `TmuxProvider` or `ZellijProvider` in the future without changing the core orchestrator logic.
- `MockProvider` enables true integration testing of the orchestration flow in CI.

