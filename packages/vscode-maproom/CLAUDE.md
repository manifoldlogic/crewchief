# vscode-maproom

## What This Is

VSCode extension providing semantic code search. Spawns the Maproom MCP server which in turn spawns the Rust daemon.

## Architecture

```
VSCode Extension (this package)
    ↓ activates on startup
Extension Host
    ↓ spawns as child process
MCP Server (from maproom-mcp, bundled or via npx)
    ↓ uses daemon-client internally
Rust Daemon (crewchief-maproom serve)
    ↓ queries
SQLite Database (~/.maproom/maproom.db)
```

## Binary Preparation

Binaries are copied from `packages/cli/bin/` during build:

```bash
pnpm prepare:binaries   # Copy binaries from cli/bin/ to vscode-maproom/bin/
pnpm build              # Runs prepare:binaries, then compiles TypeScript
pnpm vsce:package       # Creates .vsix (requires binaries in bin/)
```

**Binary source**: `packages/cli/bin/<platform>/crewchief-maproom`
**Binary target**: `packages/vscode-maproom/bin/<platform>/crewchief-maproom`

The `prepare-binaries.js` script:
- Copies from CLI package (same binaries as @crewchief/cli)
- Skips copy if binaries already exist (CI downloads them separately)
- Sets executable permissions on Unix

## Common Commands

```bash
pnpm build           # Compile + prepare binaries
pnpm test            # Run tests
pnpm vsce:package    # Create .vsix package

# Release (triggers GitHub workflow)
pnpm release:patch
```

## Configuration

User settings in VSCode:
- `maproom.database.sqlitePath` - Override default database location
- `maproom.ollama.endpoint` - Ollama URL (use `host.docker.internal` in devcontainers)

## Gotchas

- **Binaries must exist before packaging** - `prepare:binaries` fails if CLI binaries missing
- **CI exception** - In CI, missing binaries during build are allowed (downloaded later)
- **Activation timing** - Uses `onStartupFinished` for lazy activation
- **No daemon-client dependency in package.json** - It's a monorepo peer, not npm installed

## When Working Here

- After Rust changes, rebuild CLI binaries first: `cd packages/cli && pnpm build:rust`
- Test extension locally: F5 in VSCode opens Extension Development Host
- Check Output panel > "Maproom" for daemon logs
- Extension commands: `Maproom: Show Status`, `Maproom: Setup`
