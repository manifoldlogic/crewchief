# Hello World Test Extension

A minimal VSCode extension for testing the vscode-extension-specialist agent.

## Features

- Displays "Hello World" in the status bar (right-aligned)
- Activates after VSCode startup (non-blocking)
- Demonstrates proper resource management and disposal

## Development

```bash
# Install dependencies
npm install

# Compile TypeScript
npm run compile

# Watch mode for development
npm run watch

# Package as VSIX
npm run package
```

## Architecture Notes

**Activation Strategy:**
- Uses `onStartupFinished` to avoid blocking VSCode startup
- Target activation time: <500ms
- No heavy dependencies or async initialization

**Resource Management:**
- StatusBarItem registered in `context.subscriptions` for automatic disposal
- Explicit cleanup in `deactivate()` function

**Performance:**
- Zero external dependencies
- Synchronous activation (no async delays)
- Minimal memory footprint
