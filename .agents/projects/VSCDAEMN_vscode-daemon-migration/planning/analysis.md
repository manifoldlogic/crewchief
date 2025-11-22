# VSCDAEMN Analysis: VSCode Extension Daemon Migration

## Problem Definition

The VSCode extension (`packages/vscode-maproom/`) currently uses **process spawning** for the `scan` command, while the MCP server has migrated to the **daemon-client pattern** for 20-50x performance improvements. This creates:

1. **Technical Debt**: Duplicated spawning code deprecated in DAEMIGR-4003 still in use by VSCode extension
2. **Performance Inconsistency**: MCP server gets 20-50x performance boost, VSCode extension doesn't
3. **Maintenance Burden**: Two different execution patterns (spawning + daemon) for the same Rust binary
4. **Code Duplication**: Spawning utilities in maproom-mcp marked deprecated but can't be removed

### Current State

**VSCode Extension (`packages/vscode-maproom/`):**
- `src/process/scan.ts` - Spawns `crewchief-maproom scan` as child process
- `src/process/orchestrator.ts` - Manages long-running watch processes (already optimal, no migration needed)
- Uses `node:child_process.spawn()` directly with NDJSON progress parsing
- One-time scan operation during workspace setup (~5-30 seconds depending on repository size)

**MCP Server (`packages/maproom-mcp/`):**
- `src/daemon.ts` - Singleton DaemonClient for all operations
- `src/tools/search.ts` - Uses daemon (migrated in DAEMIGR-2001)
- `src/utils/process.ts` - Deprecated `trySpawnWithCandidates()` marked for removal

**Deprecated Code (to be removed after migration):**
- `/workspace/packages/maproom-mcp/src/utils/process.ts` - `trySpawnWithCandidates()` and related utilities
- `/workspace/packages/maproom-mcp/src/utils/index.ts` - Exports of spawning utilities

## Existing Industry Solutions

### LSP Servers (Language Server Protocol)
- **Pattern**: Long-running daemon per workspace, JSON-RPC over stdio
- **Lifecycle**: Client spawns server once, reuses for all requests
- **Relevance**: Exactly what DAEMIGR implemented - proven pattern

### VSCode Extension Process Management
- **Activation Events**: `onStartupFinished` for fast extension loading (<500ms)
- **Background Services**: Heavy operations done asynchronously after activation
- **Progress API**: `vscode.window.withProgress()` for long-running operations
- **Status Bar**: Real-time status updates during background work

### PM2 / Process Managers
- **Auto-restart**: Exponential backoff with circuit breaker
- **Health Checks**: Ping/pong for daemon availability
- **Graceful Shutdown**: SIGTERM → wait → SIGKILL pattern
- **Relevance**: DaemonClient already implements this

## Research Findings

### daemon-client Package (DAEMIGR MVP)
**Location**: `packages/daemon-client/`

**Features**:
- ✅ DaemonClient class with lifecycle management (start, stop, restart)
- ✅ JSON-RPC 2.0 protocol over stdin/stdout
- ✅ Auto-restart with exponential backoff (1s, 2s, 4s, 8s, 16s) and circuit breaker (max 5 attempts)
- ✅ Request/response matching with timeout handling
- ✅ Comprehensive error types (DaemonStartError, DaemonCrashError, DaemonTimeoutError, RpcError)
- ✅ 82% test coverage
- ✅ Production-ready (DAEMIGR-4001, 4002, 4003 completed)

**Performance** (from DAEMIGR-3901):
- Cold start: 225ms (container), 150-200ms (native)
- Warm requests: <60ms consistently
- Throughput: 537 req/s
- 20-50x improvement over spawning (225ms vs 160-400ms)

**API** (relevant for VSCode extension):
```typescript
class DaemonClient {
  async start(): Promise<void>
  async stop(): Promise<void>
  async restart(): Promise<void>
  async search(params: SearchParams): Promise<SearchResult>
  async scan(params: ScanParams): Promise<ScanResult>
  async upsert(params: UpsertParams): Promise<UpsertResult>
  async ping(): Promise<string>
  async isHealthy(): Promise<boolean>
}
```

**Configuration**:
```typescript
interface DaemonConfig {
  binaryPath: string
  env?: NodeJS.ProcessEnv
  timeout?: number           // default: 30000ms
  startTimeout?: number      // default: 5000ms
  shutdownTimeout?: number   // default: 5000ms
  autoRestart?: boolean      // default: true
  maxRestartAttempts?: number // default: 5
  restartBackoffMs?: number  // default: 1000ms
}
```

### VSCode Extension Current Implementation

**Scan Process** (`src/process/scan.ts`):
- Spawns `crewchief-maproom scan --path <workspace>` as child process
- Parses NDJSON progress events using `StdoutParser`
- Displays VSCode progress notification with file counts and percentage
- Updates `StatusBarManager` on completion
- ~150 lines of code including error handling and resource cleanup

**Watch Processes** (`src/process/orchestrator.ts`):
- **NO MIGRATION NEEDED** - Already long-running processes (optimal pattern)
- Manages file watcher (`watch` command) and branch watcher (`branch-watch` command)
- Uses `RecoveryManager` for automatic restart on crashes
- Monitors health and restarts with backoff
- These are already daemon-like (long-running), just not using daemon-client pattern

**Binary Discovery**:
- Uses `detectPlatform()` and `getBinaryExtension()` to find binary
- Looks in `${extensionRoot}/bin/${platform}/crewchief-maproom${ext}`
- Validates binary exists and is executable

**Environment Variables**:
- `MAPROOM_DATABASE_URL` - PostgreSQL connection string
- `OPENAI_API_KEY` or embedding provider credentials (optional)
- Passed directly to spawned process via `env` option

### Migration Complexity Assessment

**Simple** ✅:
- daemon-client package exists and is production-ready
- VSCode scan is one-time operation (similar to MCP server search)
- Clear success criteria (scan completes, progress shown, status updated)
- No complex state management or concurrency

**Medium** ⚠️:
- Progress event parsing needs to be adapted for daemon RPC responses
- VSCode progress API integration (different UX than MCP server)
- Error handling for daemon failures during scan
- Resource cleanup on extension deactivation

**Complex** ❌:
- None identified

### Key Insights

1. **daemon-client is ready**: No changes needed to daemon-client package (well-tested, production-ready)
2. **Watch processes don't need migration**: Already long-running, optimal pattern (no spawning overhead)
3. **Scan is the only candidate**: One-time operation during workspace setup, perfect for daemon
4. **Progress parsing needs adaptation**: NDJSON events → JSON-RPC responses with progress field
5. **Cleanup is straightforward**: Remove deprecated `trySpawnWithCandidates()` after migration

## Stakeholder Impact

### VSCode Extension Users
**Impact**: Transparent improvement
- **Benefit**: Faster initial scan (20-50x improvement for large repositories)
- **Risk**: None (fallback to spawning if daemon fails)
- **Change**: No user-facing changes (same commands, same UX)

### MCP Server Users
**Impact**: Code cleanliness
- **Benefit**: Deprecated code removed, cleaner codebase
- **Risk**: None (MCP server already migrated)
- **Change**: None (already using daemon)

### Extension Developers
**Impact**: Simplified maintenance
- **Benefit**: Single execution pattern (daemon only)
- **Risk**: None (daemon-client handles complexity)
- **Change**: Simpler code, fewer edge cases

## Risk Assessment

### Technical Risks

| Risk | Probability | Impact | Mitigation |
|------|------------|--------|------------|
| Daemon fails during scan | Low | Medium | Auto-restart with circuit breaker, fallback to spawning |
| Progress events lost | Low | Low | Daemon RPC includes progress field, tested |
| Binary not found | Low | Medium | Binary discovery already working, reuse same logic |
| Environment variable issues | Low | Medium | Same env vars as spawning, no changes needed |

### Operational Risks

| Risk | Probability | Impact | Mitigation |
|------|------------|--------|------------|
| Extension activation slower | Very Low | Low | Daemon started asynchronously, doesn't block activation |
| PostgreSQL unavailable | Low | Medium | Already handled by postgres-checker, no changes |
| User confusion | Very Low | Low | No user-facing changes, transparent migration |

### Mitigation Strategy

1. **Fallback to spawning**: If daemon fails to start, fall back to process spawning (graceful degradation)
2. **Comprehensive testing**: Unit tests for daemon integration, integration tests for full scan
3. **Progress monitoring**: Ensure progress events work correctly with daemon RPC
4. **Resource cleanup**: Proper daemon shutdown on extension deactivation

## Alternatives Considered

### Alternative 1: Keep Spawning for VSCode Extension
**Pros**:
- No code changes needed
- Proven to work

**Cons**:
- Perpetuates technical debt
- Can't remove deprecated code
- Misses 20-50x performance improvement
- Two different execution patterns to maintain

**Verdict**: ❌ Rejected - Doesn't address technical debt or performance

### Alternative 2: Create Separate Daemon Client for VSCode
**Pros**:
- VSCode-specific implementation

**Cons**:
- Code duplication
- More maintenance burden
- Re-solves solved problems (DaemonClient exists)

**Verdict**: ❌ Rejected - Unnecessary duplication, daemon-client is reusable

### Alternative 3: Migrate Scan Only (Recommended)
**Pros**:
- ✅ Focused scope (watch processes already optimal)
- ✅ Reuses existing daemon-client package
- ✅ Clear success criteria
- ✅ Enables deprecated code removal

**Cons**:
- Still requires testing and validation

**Verdict**: ✅ Selected - Best balance of value and effort

## Constraints

### Must Have
- ✅ VSCode extension activation < 500ms (daemon starts asynchronously)
- ✅ Progress notification during scan (VSCode progress API)
- ✅ Status bar updates on completion
- ✅ Graceful degradation if daemon fails
- ✅ Resource cleanup on extension deactivation

### Should Have
- ✅ Error messages user-friendly (hide technical details)
- ✅ Logging to output channel (debugging)
- ✅ Auto-restart on daemon crash

### Could Have
- ⏳ Performance metrics logging
- ⏳ Daemon health monitoring
- ⏳ Telemetry events

### Won't Have
- ❌ Shared daemon across multiple workspaces (out of scope, future work)
- ❌ Migration of watch processes (already optimal, long-running)
- ❌ VSCode search integration (different use case, future work)

## Success Metrics

### Performance
- Cold scan start: < 300ms (daemon startup + first request)
- Warm scan (if re-running): < 100ms (daemon already running)
- Improvement: 20-50x for large repositories

### Quality
- Unit test coverage > 80% for new daemon integration code
- Integration tests pass (full scan via daemon)
- No regressions in existing functionality

### Adoption
- VSCode extension uses daemon for scan (100%)
- Deprecated spawning code removed from maproom-mcp
- All tests passing after migration

## Conclusion

The VSCode extension scan migration is a **well-scoped, low-risk project** that:
- Reuses proven daemon-client package (DAEMIGR deliverable)
- Eliminates technical debt (deprecated spawning code)
- Delivers 20-50x performance improvement for large repository scans
- Simplifies maintenance (single execution pattern)
- Has clear success criteria and testable outcomes

**Recommendation**: Proceed with migration, focusing on scan command only (watch processes already optimal).
