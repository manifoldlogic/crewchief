# Security Review: Headless CLI Core

## 1. Threat Analysis
- **Command Injection**: The `runCommand` interface accepts strings.
  - *Risk*: Malicious agent configuration could execute arbitrary shell commands.
  - *Mitigation*: This is "Remote Code Execution as a Service" by design (it's an agent runner). The security boundary is the **Worktree**. We rely on standard OS permissions.
- **Secrets Leakage**: Headless mode logs to stdout.
  - *Risk*: API keys printed to the console could be captured in CI logs.
  - *Mitigation*: Agents should use `dotenv` and not print secrets. The CLI itself should avoid logging env vars.

## 2. Process Isolation
- **HeadlessProvider**: Spawns processes as children of the CLI.
- **Risk**: A crashing agent could take down the CLI.
- **Mitigation**: Use `detached: false` but handle `error` events on the child process streams to prevent unhandled exceptions bubbling up.

## 3. OS Permissions
- **MacOS**: iTerm automation requires "Accessibility" permissions. This doesn't change.
- **Linux/Headless**: Standard user permissions apply. No root required.

## 4. Conclusion
No new significant security surface area is introduced. The decoupling actually improves security testing capabilities by allowing automated security scans in CI.

