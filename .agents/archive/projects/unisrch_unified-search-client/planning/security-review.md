# Security Review: Unified Search Client

## Security Assessment
- **Command Injection**: This is the primary risk. If user input is concatenated directly into a shell command string, it allows RCE.
- **Mitigation**: Use `child_process.spawn` (or `execFile`) which takes arguments as an array, avoiding shell interpretation. **NEVER** use `exec` with user input.

## Gaps & Risks
- **Path Traversal**: Ensure the binary path is fixed and trusted.

## Mitigations
- **Spawn with Array**: Strictly enforce `spawn(command, [args])` pattern.
- **Input Validation**: Use Zod to validate search query structure before passing it.
