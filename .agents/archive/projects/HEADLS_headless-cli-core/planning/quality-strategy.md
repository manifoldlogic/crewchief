# Quality Strategy: Headless CLI Core

## 1. Test Strategy
We shift from "Manual Verification" (does it look right in iTerm?) to **Automated Verification** using the `MockProvider`.

### Levels of Testing
1.  **Unit Tests (`src/terminal/providers/*.test.ts`)**:
    - Verify `HeadlessProvider` spawns processes correctly.
    - Verify `TerminalFactory` detects environment variables correctly.
2.  **Integration Tests (`src/orchestrator/*.test.ts`)**:
    - Run the full agent spawn flow using `MockProvider`.
    - Assert that `runCommand` was called with the expected strings.
3.  **Manual Verification (iTerm)**:
    - Regression test: ensure the UI still works on macOS.

## 2. Critical Paths
- **Process Lifecycle**: Ensure headless processes don't become zombies when the main CLI exits.
- **Signal Handling**: `SIGINT` (Ctrl+C) must propagate to child processes in Headless mode.

## 3. Acceptance Criteria
- [ ] `crewchief spawn` works in VSCode terminal (uses Headless/Stream mode).
- [ ] `crewchief spawn` works in iTerm2 (uses Split Panes).
- [ ] `npm test` runs orchestrator tests without opening windows.
- [ ] No `TERM_PROGRAM` error on Linux.

## 4. Risk Mitigation
- **Mocking**: Use `MockProvider` extensively to cover edge cases (command failure, layout errors).
- **Dogfooding**: The team should try running agents in VSCode terminal to verify usability of the log stream.

