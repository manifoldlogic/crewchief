# daemon-client

## What This Is

Internal TypeScript library that provides JSON-RPC 2.0 communication with the `crewchief-maproom` Rust daemon. **Not published to npm** - shared dependency used by maproom-mcp and vscode-maproom.

The daemon pattern provides **20-50x performance improvement** over spawning a new process per request (225ms vs 160-400ms cold start).

## Architecture

```
maproom-mcp / vscode-maproom
         ↓ imports
    daemon-client (this package)
         ↓ spawns & manages
    crewchief-maproom serve (Rust daemon)
         ↓ JSON-RPC over stdio
    SQLite database
```

## Type Synchronization

Types in `src/client.ts` must stay in sync with Rust. Look for comments like:
```typescript
// Sync with: crates/maproom/src/daemon/types.rs ContextParams
```

**When modifying types here:**
1. Update the corresponding Rust struct in `crates/maproom/src/daemon/types.rs`
2. Or vice versa - Rust is the source of truth

Key sync points:
- `SearchParams` ↔ `SearchParams` in Rust
- `ContextParams` ↔ `ContextParams` in Rust
- `RustContextBundle` ↔ `ContextBundle` in Rust

## Key Components

- `DaemonClient` - Main client class, manages daemon lifecycle
- `DaemonLifecycle` - Handles spawn, restart, backoff
- `RpcProtocol` - JSON-RPC serialization/parsing
- Error types: `DaemonError`, `DaemonTimeoutError`, `DaemonCrashError`

## Common Commands

```bash
pnpm build        # Compile TypeScript
pnpm test         # Unit tests
pnpm test:watch   # Watch mode
```

## Gotchas

- **No npm publish needed** - Changes propagate to dependents via monorepo linking
- **Daemon auto-starts** - First request spawns daemon if not running
- **Graceful shutdown** - `stop()` waits for in-flight requests (up to `shutdownTimeout`)
- **Request ID rollover** - IDs reset at `Number.MAX_SAFE_INTEGER` to prevent overflow

## When Working Here

- Test changes via maproom-mcp or vscode-maproom (no standalone test binary)
- If adding new RPC methods, update both TypeScript and Rust sides
- Circuit breaker prevents restart storms - check `DaemonLifecycle` for tuning
