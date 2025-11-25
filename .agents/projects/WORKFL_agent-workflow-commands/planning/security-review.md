# Security Review: Workflow Commands

## 1. File System Access
- The CLI modifies files in `.agents/`.
- **Risk**: Path traversal?
- **Mitigation**: Validate "slug" and "name" to ensure they don't contain `..` or `/`. Use strict regex `^[A-Z0-9]{2,8}$` for slug.

## 2. Command Injection
- **Risk**: If we use user input in shell commands.
- **Mitigation**: We use `fs` operations directly, avoiding shell execution where possible.

