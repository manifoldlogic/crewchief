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

## Type Synchronization with Rust

**Source of Truth**: Rust types in `crates/maproom/src/search/errors.rs` and `crates/maproom/src/search/results.rs`

**Sync Pattern**: TypeScript types in `src/types.ts` mirror Rust structs with sync comments.

### Manual Sync Checklist

When Rust types change:
- [ ] Update corresponding TypeScript interfaces
- [ ] Verify sync comments still link correctly
- [ ] Run type sync validation tests: `pnpm test types.test.ts`
- [ ] Check integration tests pass

### Type Sync Validation

Run validation tests:
```bash
cd packages/daemon-client
pnpm test types.test.ts
```

Tests verify:
- Enum values match exactly (ErrorType, PipelineStage)
- Structure fields match (SearchErrorDetails, QueryUnderstanding)
- Serialization roundtrip works

### Adding New Error Types

1. **Rust** (`crates/maproom/src/search/errors.rs`):
   - Add variant to `ErrorType` enum
   - Add conversion case in `from_pipeline_error()`
   - Add 1-2 actionable suggestions

2. **TypeScript** (`packages/daemon-client/src/types.ts`):
   - Add variant to `ErrorType` union type
   - Update sync comment if needed

3. **Validation** (`packages/daemon-client/src/types.test.ts`):
   - Add new variant to validation test array
   - Verify test passes

4. **Integration Test** (`crates/maproom/tests/daemon_error_serialization.rs`):
   - Add test case for new error type
   - Verify serialization works end-to-end

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
