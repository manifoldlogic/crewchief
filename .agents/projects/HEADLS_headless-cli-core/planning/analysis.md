# Analysis: Headless CLI Core

## 1. Problem Definition
The CrewChief CLI currently has a hard dependency on `iTerm.app` on macOS.
- **Code Evidence**: `packages/cli/src/cli/index.ts` explicitly checks `process.env.TERM_PROGRAM !== 'iTerm.app'` and exits with an error if false.
- **Impact**:
  - Cannot run in Linux environments (servers, devcontainers).
  - Cannot run in CI/CD pipelines (headless).
  - Cannot run on Windows.
  - Developers using VSCode terminal or other emulators (Alacritty, Kitty) are locked out.

## 2. Existing Solutions & Context
- **Current Implementation**:
  - `packages/cli/src/iterm/` contains specific AppleScript/JXA logic to manipulate windows and panes.
  - `packages/cli/src/terminal/` exists but appears to be an incomplete abstraction or just a wrapper.
- **Industry Standard**:
  - Most CLI tools (e.g., `tmux`, `zellij`, `overmind`) abstract the concept of a "Session" or "Window" behind a provider interface.
  - **Headless Mode**: Usually implies running processes in the background (detached) or streaming logs to stdout without interactive window management.

## 3. Requirements
1.  **Abstract Interface**: A `TerminalProvider` interface must define methods for:
    - `createWindow()` / `createTab()`
    - `splitPane()`
    - `runCommand(paneId, command)`
    - `setLayout(layoutDef)`
2.  **Implementations**:
    - `ITermProvider`: Preserves existing functionality (MacOS only).
    - `HeadlessProvider`: Runs commands using Node's `child_process` or a simplified process manager, logging to stdout/file. No window management.
    - `MockProvider`: For unit testing orchestrator logic without side effects.
3.  **Auto-Detection**: The CLI should detect the environment and choose the best provider:
    - If `TERM_PROGRAM == iTerm.app` -> `ITermProvider`.
    - If `CI=true` or `--headless` flag -> `HeadlessProvider`.
    - Default fallback -> `HeadlessProvider` (safe default).

## 4. Risks
- **UX Degradation**: The "Dashboard" view (multiple panes) is a key value prop. Headless mode loses this.
  - *Mitigation*: Headless mode should output a clear log stream or a simple TUI status summary.
- **Complexity**: Managing process lifecycles in `HeadlessProvider` (zombies, signal handling) is harder than letting iTerm handle it.

