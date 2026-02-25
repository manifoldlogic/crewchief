# daemon-client

## What This Is

Internal TypeScript library providing JSON-RPC 2.0 communication with the `crewchief-maproom` Rust daemon. **Not published to npm** — shared dependency used by maproom-mcp and vscode-maproom.

## Type Synchronization

Types in `src/client.ts` and `src/types.ts` must stay in sync with Rust. Both files contain `// Sync with:` comments.

Full sync workflow: `.claude/docs/type-sync-workflow.md`
Adding error types: `docs/runbooks/adding-error-types.md`

## Gotchas

- **No npm publish needed** — changes propagate to dependents via monorepo linking
- **Daemon auto-starts** — first request spawns daemon if not running
- **Graceful shutdown** — `stop()` waits for in-flight requests (up to `shutdownTimeout`)
- **Request ID rollover** — IDs reset at `Number.MAX_SAFE_INTEGER` to prevent overflow
- **No standalone test binary** — test changes via maproom-mcp or vscode-maproom

## When Working Here

- If adding new RPC methods, update both TypeScript and Rust sides (see type-sync-workflow)
- Circuit breaker prevents restart storms — check `DaemonLifecycle` for tuning
- Run type sync validation: `pnpm test types.test.ts`
