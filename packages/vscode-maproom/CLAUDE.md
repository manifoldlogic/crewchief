# vscode-maproom

VSCode extension providing semantic code search. Spawns the Maproom MCP server which spawns the Rust daemon.

## Binary Preparation (Critical)

Binaries are copied from `packages/cli/bin/` during build:

```bash
pnpm prepare:binaries   # Copy binaries from cli/bin/ to vscode-maproom/bin/
pnpm build              # Runs prepare:binaries, then compiles TypeScript
pnpm vsce:package       # Creates .vsix (requires binaries in bin/)
```

After Rust changes, rebuild CLI binaries first: `cd packages/cli && pnpm build:rust`

## Configuration

- `maproom.database.sqlitePath` — Override default database location
- `maproom.ollama.endpoint` — Ollama URL (**use `host.docker.internal` in devcontainers**, not `localhost`)

## Gotchas

- **Binaries must exist before packaging** — `prepare:binaries` fails if CLI binaries missing
- **CI exception** — in CI, missing binaries during build are allowed (downloaded later)
- **Activation timing** — uses `onStartupFinished` for lazy activation
- **No daemon-client in package.json** — it's a monorepo peer, not npm installed

## Docs

- Daemon architecture: `docs/architecture/daemon.md`
