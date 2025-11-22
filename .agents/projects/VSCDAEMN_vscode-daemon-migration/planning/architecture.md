# VSCDAEMN Architecture: VSCode Extension Daemon Migration

## Architecture Overview

This project migrates the VSCode extension's `scan` command from **process spawning** to **daemon-client pattern**, achieving consistency with the MCP server architecture and enabling removal of deprecated spawning utilities.

### High-Level Architecture

```
┌─────────────────────────────────────────────────────────────┐
│ VSCode Extension (packages/vscode-maproom/)                 │
│                                                              │
│  ┌──────────────────────────────────────────────────────┐  │
│  │ extension.ts (Entry Point)                           │  │
│  │ - activate(): Fast activation (<500ms)               │  │
│  │ - Register commands                                  │  │
│  │ - Initialize UI components                           │  │
│  └─────────────────┬────────────────────────────────────┘  │
│                    │                                         │
│  ┌─────────────────▼────────────────────────────────────┐  │
│  │ process/scan.ts (MIGRATION TARGET)                   │  │
│  │                                                       │  │
│  │ BEFORE:                                              │  │
│  │ - spawn('crewchief-maproom scan')                   │  │
│  │ - Parse NDJSON progress events                       │  │
│  │ - Display VSCode progress                            │  │
│  │                                                       │  │
│  │ AFTER:                                               │  │
│  │ - DaemonClient.scan(params)                         │  │
│  │ - Handle JSON-RPC progress responses                 │  │
│  │ - Display VSCode progress                            │  │
│  └─────────────────┬────────────────────────────────────┘  │
│                    │                                         │
│                    │ Uses                                    │
│                    ▼                                         │
│  ┌─────────────────────────────────────────────────────┐  │
│  │ daemon-client (NEW DEPENDENCY)                       │  │
│  │ packages/daemon-client/                              │  │
│  │                                                       │  │
│  │ - DaemonClient class                                 │  │
│  │ - Auto-restart with circuit breaker                  │  │
│  │ - JSON-RPC 2.0 protocol                             │  │
│  │ - Comprehensive error handling                       │  │
│  └─────────────────┬────────────────────────────────────┘  │
│                    │                                         │
└────────────────────┼─────────────────────────────────────────┘
                     │
                     │ JSON-RPC over stdin/stdout
                     ▼
          ┌──────────────────────────┐
          │ crewchief-maproom daemon │
          │ (Rust binary)            │
          │                          │
          │ - serve subcommand       │
          │ - Connection pooling     │
          │ - Long-running process   │
          └────────┬─────────────────┘
                   │
                   │ PostgreSQL protocol
                   ▼
          ┌──────────────────────────┐
          │ PostgreSQL + pgvector    │
          │ (Docker container)       │
          │                          │
          │ - repos, worktrees       │
          │ - chunks, embeddings     │
          │ - chunk_edges            │
          └──────────────────────────┘
```

### Component Changes

#### New Component: daemon-client Integration
**Location**: `packages/vscode-maproom/src/daemon/`

**Responsibilities**:
- Create and manage DaemonClient singleton per workspace
- Configure daemon with extension root binary path
- Pass environment variables (DATABASE_URL, API keys)
- Handle daemon lifecycle (start, stop, health checks)
- Graceful shutdown on extension deactivation

**API**:
```typescript
export function getDaemonClient(config: DaemonConfig): DaemonClient
export function shutdownDaemon(): Promise<void>
export function isDaemonHealthy(): Promise<boolean>
```

#### Modified Component: process/scan.ts
**Changes**:
- Replace `spawn()` calls with `DaemonClient.scan()`
- Remove NDJSON parsing (daemon returns structured JSON-RPC responses)
- Adapt progress handling for RPC progress field
- Maintain same VSCode progress API integration
- Keep error handling and resource cleanup patterns

**Before**:
```typescript
export async function runInitialScan(config: ScanConfig): Promise<void> {
  const binaryPath = await findBinary(config.extensionRoot)
  const child = spawn(binaryPath, ['scan', '--path', config.workspaceRoot], {
    env: { MAPROOM_DATABASE_URL: config.databaseUrl, ...config.env }
  })
  
  const parser = new StdoutParser()
  child.stdout.pipe(parser)
  
  await vscode.window.withProgress({
    location: vscode.ProgressLocation.Notification,
    title: 'Scanning workspace',
  }, async (progress) => {
    for await (const event of parser) {
      progress.report({ 
        message: `${event.files_processed}/${event.total_files} files`,
        increment: event.percentage 
      })
    }
  })
}
```

**After**:
```typescript
import { getDaemonClient } from '../daemon'

export async function runInitialScan(config: ScanConfig): Promise<void> {
  const daemon = getDaemonClient({
    binaryPath: await findBinary(config.extensionRoot),
    env: { MAPROOM_DATABASE_URL: config.databaseUrl, ...config.env },
    timeout: 300000, // 5 minutes for large scans
  })
  
  await vscode.window.withProgress({
    location: vscode.ProgressLocation.Notification,
    title: 'Scanning workspace',
  }, async (progress) => {
    const result = await daemon.scan({
      path: config.workspaceRoot,
      onProgress: (event) => {
        progress.report({ 
          message: `${event.files_processed}/${event.total_files} files`,
          increment: event.percentage 
        })
      }
    })
    return result
  })
}
```

#### Removed Components (Cleanup)
**Location**: `packages/maproom-mcp/src/utils/`

**Files to Remove**:
- `process.ts` - Contains deprecated `trySpawnWithCandidates()`
- `index.ts` - Re-exports spawning utilities

**Rationale**:
- Marked deprecated in DAEMIGR-4003
- Only used by VSCode extension (last consumer)
- After migration, no code uses spawning pattern
- daemon-client handles all process management

#### Unchanged Components (Out of Scope)

**process/orchestrator.ts** - Watch process management
- ❌ **NO MIGRATION** - Already optimal (long-running processes)
- Uses `RecoveryManager` for restart on crash
- Manages file watcher and branch watcher
- These are daemon-like (long-running), not one-time spawns

**Docker/Postgres Management** - Container lifecycle
- ❌ **NO CHANGES** - Already working correctly
- `docker/manager.ts` - Docker container management
- `services/postgres-checker.ts` - Health checks
- Independent of scan execution pattern

**UI Components** - Status bar, setup wizard
- ❌ **NO CHANGES** - UI remains identical
- `ui/statusBar.ts` - Real-time status updates
- `ui/setupWizard.ts` - Initial configuration
- Progress reporting adapts to daemon responses

## Data Flow

### Current Data Flow (Spawning Pattern)

```
┌──────────────┐
│ User Trigger │ maproom.setup command
└──────┬───────┘
       │
       ▼
┌──────────────────────────────────────┐
│ setupWizard.ts                       │
│ - Collect DATABASE_URL               │
│ - Collect embedding provider         │
└──────┬───────────────────────────────┘
       │
       ▼
┌──────────────────────────────────────┐
│ scan.ts: runInitialScan()            │
│ 1. Find binary in bin/${platform}/   │
│ 2. spawn('crewchief-maproom scan')   │
│ 3. Pass env vars (DATABASE_URL, etc) │
└──────┬───────────────────────────────┘
       │
       │ spawn()
       ▼
┌──────────────────────────────────────┐
│ crewchief-maproom binary             │
│ - scan subcommand                    │
│ - Reads DATABASE_URL from env        │
│ - Outputs NDJSON progress to stdout  │
│ - Exits on completion                │
└──────┬───────────────────────────────┘
       │
       │ stdout (NDJSON stream)
       ▼
┌──────────────────────────────────────┐
│ StdoutParser                         │
│ - Parses NDJSON lines                │
│ - Emits WatchEvent objects           │
└──────┬───────────────────────────────┘
       │
       │ async iterator
       ▼
┌──────────────────────────────────────┐
│ VSCode Progress API                  │
│ - withProgress()                     │
│ - progress.report()                  │
│ - Notification with percentage       │
└──────┬───────────────────────────────┘
       │
       ▼
┌──────────────────────────────────────┐
│ StatusBarManager.setIndexed()       │
│ - Update status to "Indexed"        │
│ - Show file count                    │
└──────────────────────────────────────┘
```

### New Data Flow (Daemon Pattern)

```
┌──────────────┐
│ User Trigger │ maproom.setup command
└──────┬───────┘
       │
       ▼
┌──────────────────────────────────────┐
│ setupWizard.ts                       │
│ - Collect DATABASE_URL               │
│ - Collect embedding provider         │
└──────┬───────────────────────────────┘
       │
       ▼
┌──────────────────────────────────────┐
│ daemon/index.ts: getDaemonClient()   │
│ 1. Create DaemonClient singleton     │
│ 2. Configure with binary path        │
│ 3. Pass env vars (DATABASE_URL, etc) │
│ 4. Start daemon (auto-restart ready) │
└──────┬───────────────────────────────┘
       │
       │ DaemonClient.start()
       ▼
┌──────────────────────────────────────┐
│ DaemonClient (daemon-client package) │
│ - spawns 'crewchief-maproom serve'   │
│ - Establishes JSON-RPC over stdio    │
│ - Monitors health (ping)             │
│ - Auto-restart on crash              │
└──────┬───────────────────────────────┘
       │
       │ JSON-RPC 2.0 request
       ▼
┌──────────────────────────────────────┐
│ crewchief-maproom daemon             │
│ - serve subcommand (long-running)    │
│ - Reads DATABASE_URL from env        │
│ - Maintains connection pool          │
│ - Returns JSON-RPC responses         │
└──────┬───────────────────────────────┘
       │
       │ JSON-RPC 2.0 response (with progress)
       ▼
┌──────────────────────────────────────┐
│ scan.ts: daemon.scan(params)         │
│ - Send scan RPC request              │
│ - Receive progress in response       │
│ - Call onProgress callback           │
└──────┬───────────────────────────────┘
       │
       │ progress callback
       ▼
┌──────────────────────────────────────┐
│ VSCode Progress API                  │
│ - withProgress()                     │
│ - progress.report()                  │
│ - Notification with percentage       │
└──────┬───────────────────────────────┘
       │
       ▼
┌──────────────────────────────────────┐
│ StatusBarManager.setIndexed()       │
│ - Update status to "Indexed"        │
│ - Show file count                    │
└──────────────────────────────────────┘
```

### Key Differences

| Aspect | Spawning (Before) | Daemon (After) |
|--------|------------------|----------------|
| **Process Lifecycle** | New process per scan | Long-running daemon reused |
| **Communication** | stdout NDJSON stream | JSON-RPC 2.0 over stdio |
| **Progress Reporting** | NDJSON events parsed | JSON-RPC progress field |
| **Error Handling** | Exit codes + stderr | JSON-RPC error responses |
| **Resource Usage** | Process spawn overhead | Connection pool reuse |
| **Startup Time** | 100-200ms per scan | 225ms first scan, <60ms subsequent |

## Configuration

### DaemonClient Configuration

```typescript
interface DaemonConfig {
  binaryPath: string              // ${extensionRoot}/bin/${platform}/crewchief-maproom
  env: {
    MAPROOM_DATABASE_URL: string  // PostgreSQL connection string
    OPENAI_API_KEY?: string       // Optional, from SecretStorage
    // ... other embedding provider credentials
  }
  timeout: 300000                 // 5 minutes for large scans
  startTimeout: 5000              // 5 seconds to start daemon
  shutdownTimeout: 5000           // 5 seconds graceful shutdown
  autoRestart: true               // Enable auto-restart
  maxRestartAttempts: 5           // Circuit breaker threshold
  restartBackoffMs: 1000          // Initial backoff (exponential)
}
```

### Extension Configuration (No Changes)

Existing configuration remains unchanged:
- `DATABASE_URL` stored in VSCode SecretStorage
- Embedding provider credentials (OPENAI_API_KEY, etc.) in SecretStorage
- PostgreSQL connection via `postgres-checker` service
- Docker configuration for local PostgreSQL container

## Performance Considerations

### Cold Start Performance

**Spawning Pattern** (Current):
```
Total: 100-200ms
├─ Binary discovery: 5-10ms
├─ Process spawn: 50-100ms
├─ Binary initialization: 20-50ms
└─ First query: 30-60ms
```

**Daemon Pattern** (New):
```
First scan: 225ms
├─ Daemon startup: 150-200ms
│  ├─ Binary discovery: 5-10ms
│  ├─ Process spawn: 50-100ms
│  ├─ JSON-RPC init: 20-50ms
│  └─ Connection pool: 30-60ms
└─ First query: 50-75ms

Subsequent scans: <60ms
└─ RPC request/response: 50-60ms
```

**Trade-off**: Slightly slower first scan (+25-125ms), but 20-50x faster for re-scans.

### Memory Footprint

**Spawning Pattern**:
- No persistent process (0MB when idle)
- Spawn overhead per scan (~20-50MB during execution)

**Daemon Pattern**:
- Persistent daemon (~50-80MB baseline)
- Connection pool (~20MB)
- Auto-restart adds minimal overhead (~1MB)

**Trade-off**: Higher baseline memory (+70-100MB), but avoids spawn overhead.

### Optimization Opportunities

1. **Lazy Daemon Start**: Only start daemon when first scan triggered (not on extension activation)
2. **Connection Pool Tuning**: Adjust pool size based on scan workload (default: 5 connections)
3. **Timeout Configuration**: Longer timeout for initial scans (5 minutes), shorter for re-scans
4. **Circuit Breaker**: Prevent restart loops (max 5 attempts, then require manual intervention)

## Security Considerations

### Environment Variable Exposure

**Risk**: Database URL and API keys visible in `/proc/<pid>/environ`

**Mitigation** (from DAEMIGR security review):
- Use VSCode SecretStorage for credentials (encrypted at rest)
- Environment variables only passed to daemon process (not spawned children)
- Daemon process runs with user privileges (no privilege escalation)
- PostgreSQL URL contains localhost-only connection (no network exposure)

### Binary Integrity

**Risk**: Malicious binary executed by extension

**Mitigation**:
- Binary shipped with extension (VSIX package)
- Binary path from hardcoded candidates (not user-configurable)
- File permissions checked (must be executable, owned by user)
- Binary checksum verification (future enhancement)

### Daemon Crash Recovery

**Risk**: Daemon crash exposes extension to instability

**Mitigation**:
- Auto-restart with exponential backoff (1s, 2s, 4s, 8s, 16s)
- Circuit breaker after 5 restart attempts (prevents loops)
- Graceful degradation: Fallback to spawning if daemon fails
- Error messages logged to output channel (no credential exposure)

## Technology Choices

### Why Reuse daemon-client Package?

**Alternatives Considered**:
1. Inline daemon management in VSCode extension
2. Create VSCode-specific daemon client
3. Reuse daemon-client package (CHOSEN)

**Decision Rationale**:
- ✅ Already production-ready (82% test coverage, DAEMIGR MVP complete)
- ✅ Proven in MCP server (20-50x performance improvement)
- ✅ Comprehensive error handling and auto-restart
- ✅ No duplication of complex process lifecycle management
- ✅ Simpler testing (daemon-client already tested)

### Why Keep Watch Processes Unchanged?

**Rationale**:
- Watch processes are already long-running (daemon-like pattern)
- No spawn overhead (started once, run until stopped)
- RecoveryManager already handles restart on crash
- Migrating to daemon-client adds complexity without benefit
- Out of scope for this project

### Why JSON-RPC 2.0?

**Rationale** (inherited from MAPDAEMON/DAEMIGR):
- Standard protocol (well-defined spec)
- Request/response matching with IDs
- Error codes and structured error responses
- Proven in LSP, VSCode extensions, production systems
- Low latency for local IPC (~0.5-1ms overhead)

## Long-Term Maintainability

### MVP Focus

This migration focuses on **shipping value**, not **enterprise complexity**:
- ✅ Single execution pattern (daemon only)
- ✅ Remove deprecated spawning code
- ✅ 20-50x performance improvement
- ✅ Reuse proven daemon-client package

**Explicitly NOT implementing**:
- ❌ Shared daemon across workspaces (future work)
- ❌ Performance telemetry (future work)
- ❌ Binary signature verification (future work)
- ❌ Platform-specific secrets management (future work)

### Future Enhancements (Post-MVP)

**Shared Daemon** (Phase 3 Project):
- One daemon serves multiple VSCode workspaces
- Socket-based IPC instead of stdio
- More complex lifecycle management
- Significant resource savings for multi-workspace users

**Performance Telemetry**:
- Log scan latency, throughput, error rates
- Identify performance bottlenecks
- Optimize connection pool size

**Enhanced Security**:
- Binary signature verification
- Platform-specific secrets (Keychain, Credential Manager)
- Audit logging for compliance

## Deployment Strategy

### Extension Packaging

**No Changes Required**:
- Binary already bundled in VSIX (bin/${platform}/crewchief-maproom)
- daemon-client package added as dependency in package.json
- Extension size increases by ~100KB (daemon-client code)

### Rollout Plan

**Phase 1**: Internal testing (developer machines)
- Test on macOS, Linux, Windows
- Verify daemon starts correctly
- Confirm scan completes successfully

**Phase 2**: Beta release (opt-in users)
- Release as pre-release version (0.2.0-beta.1)
- Monitor crash reports and error logs
- Fix critical bugs before stable release

**Phase 3**: Stable release (all users)
- Release as stable version (0.2.0)
- Monitor adoption metrics
- Prepare rollback plan if critical issues found

### Rollback Strategy

If critical issues arise:
1. Revert to previous extension version (0.1.0)
2. Spawning pattern still available (no code removal until tested)
3. Users can downgrade via VSIX file
4. Fix bugs in separate release (0.2.1)

## Conclusion

The VSCode extension daemon migration is a **straightforward architectural improvement** that:
- Reuses proven daemon-client package (DAEMIGR deliverable)
- Achieves 20-50x performance improvement for re-scans
- Simplifies codebase (single execution pattern)
- Enables removal of deprecated spawning utilities
- Has clear deployment and rollback strategy

**Recommendation**: Proceed with migration as designed, focusing on MVP scope.
