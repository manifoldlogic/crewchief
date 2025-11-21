# Project: DAEMIGR - Daemon Client Migration

## Overview
**DAEMIGR** is a critical architecture migration project to eliminate the "split-brain" process-spawning approach across all Maproom clients and migrate them to use the new daemon architecture implemented in **MAPDAEMON**.

## Problem Statement

### Current State: Process Spawning Overhead

The MAPDAEMON project successfully implemented a high-performance daemon with:
- ✅ JSON-RPC 2.0 protocol over stdin/stdout
- ✅ Connection pooling (shared across requests)
- ✅ Sub-millisecond ping latency
- ✅ Efficient `search` method integration
- ✅ Production-ready and tested

**However**, the daemon is **not being used** by any clients yet. All clients still spawn new processes:

#### 1. MCP Server (`packages/maproom-mcp/`)
**Current:** Spawns `crewchief-maproom search` for **every single search request**
- **File:** `packages/maproom-mcp/src/tools/search.ts` (line 268-291)
- **Overhead:** ~200-500ms per request (process spawn + DB handshake)
- **Impact:** HIGH - Core search functionality, frequent requests
- **Connection Pooling:** None (new connection per request)

#### 2. VSCode Extension (`packages/vscode-maproom/`)
**Current:** Mixed approach
- ✅ **watch/branch-watch:** Long-running processes (good!)
- ❌ **scan:** Spawns `crewchief-maproom scan` for each initial/manual scan
  - **File:** `packages/vscode-maproom/src/process/scan.ts` (line 219)
  - **Overhead:** ~200-500ms startup (less critical, infrequent operation)
  - **Impact:** MEDIUM - Only used during initial setup and manual scans

### Performance Comparison

| Operation | Current (Spawn) | With Daemon | Improvement |
|-----------|----------------|-------------|-------------|
| **MCP Search (cold)** | ~200-500ms | ~200-500ms (first request) | None |
| **MCP Search (warm)** | ~200-500ms | ~0.5-5ms | **50-100x faster** |
| **VSCode Scan** | ~200-500ms | ~200-500ms | Same (infrequent) |
| **Concurrent Requests** | N processes, N DB connections | 1 daemon, 1 pool | **Massive savings** |

---

## Solution: Daemon Client Library

Implement a **DaemonClient** library that:
1. Starts the daemon once (lazy initialization)
2. Sends JSON-RPC requests via stdin
3. Parses JSON-RPC responses from stdout
4. Handles daemon lifecycle (health checks, crash recovery)
5. Provides high-level APIs for `ping`, `search`, etc.

---

## Project Scope

### In Scope
1. **Create daemon client library** (`packages/daemon-client/`)
   - TypeScript package for Node.js
   - Handles daemon process lifecycle
   - JSON-RPC request/response management
   - Error handling and recovery

2. **Migrate MCP Server** (`packages/maproom-mcp/`)
   - Replace `trySpawnWithCandidates` in `tools/search.ts`
   - Use `DaemonClient` for all search operations
   - Keep chunk ID fetching logic (still needed)
   - Update error handling for JSON-RPC errors

3. **Deprecate old process spawning utilities**
   - Mark `packages/maproom-mcp/src/utils/process.ts` as deprecated
   - Keep for VSCode extension scan operations (less critical)

4. **Integration testing**
   - Test daemon lifecycle (start, stop, restart on crash)
   - Test concurrent requests
   - Test error recovery

5. **Documentation**
   - Update MCP README with daemon architecture
   - Document migration for future clients

### Out of Scope (Future Work)
- **VSCode Extension migration:** Keep scan as-is (infrequent operation, low ROI)
- **watch/branch-watch:** Already long-running, no migration needed
- **CLI usage:** CLI commands remain unchanged (direct binary execution)

### Dependencies
- ✅ **MAPDAEMON** complete - Daemon implementation ready
- ⚠️ **NODE_ENV=development** recommended during development for better debugging

---

## Architecture

### Current Architecture (MCP Server)

```
MCP Request → TypeScript Handler
                ↓
          spawn crewchief-maproom search
                ↓
          New Process + New DB Connection
                ↓
          Execute Query
                ↓
          Return JSON → Parse
                ↓
          Process Exits + Connection Closes
```

**Problems:**
- Process spawn overhead (~100-200ms)
- Database handshake overhead (~50-150ms)
- No connection reuse
- No state sharing across requests

### Target Architecture (With Daemon)

```
MCP Request → TypeScript Handler
                ↓
          DaemonClient.search()
                ↓
          JSON-RPC request to running daemon via stdin
                ↓
          Daemon (already running, connection pooled)
                ↓
          Execute Query (pooled connection, reused)
                ↓
          JSON-RPC response via stdout
                ↓
          Parse and return
```

**Benefits:**
- No process spawn (0ms overhead after first request)
- Connection pooled (~0ms handshake)
- **Total latency improvement: 50-100x for warm requests**

---

## Key Design Decisions

### 1. Separate Package vs Embedded Library
**Decision:** Create separate `packages/daemon-client/` package

**Rationale:**
- Reusable across MCP server, VSCode extension, future clients
- Clean separation of concerns
- Easier to test in isolation
- Can be published to npm if needed

### 2. Daemon Lifecycle Management
**Decision:** Lazy start on first request, keep alive until explicit shutdown

**Rationale:**
- No configuration needed from clients
- Daemon auto-starts when needed
- Simpler client code (no manual daemon management)
- Matches Unix philosophy (start services when needed)

### 3. Health Checking
**Decision:** Send `ping` before each search request (with timeout)

**Rationale:**
- Detects stale/crashed daemons quickly
- Minimal overhead (~0.5ms)
- Enables auto-restart on crash
- Prevents waiting for timeout on dead process

### 4. Error Handling
**Decision:** Fallback to process spawning if daemon fails repeatedly

**Rationale:**
- Graceful degradation (slower but functional)
- Better user experience than complete failure
- Allows deployment even if daemon has issues
- Can be removed once daemon is proven stable

### 5. One Daemon Per Client vs Shared Daemon
**Decision:** One daemon per MCP server instance (not shared across multiple clients)

**Rationale:**
- Simpler lifecycle management (daemon owned by client)
- No IPC coordination needed
- Isolated failures (one client crash doesn't affect others)
- Matches current process spawning behavior

---

## Implementation Plan

### Phase 1: DaemonClient Library (3-4 hours)

**Deliverables:**
- `packages/daemon-client/package.json`
- `packages/daemon-client/src/index.ts` - Main exports
- `packages/daemon-client/src/client.ts` - DaemonClient class
- `packages/daemon-client/src/lifecycle.ts` - Process lifecycle management
- `packages/daemon-client/src/rpc.ts` - JSON-RPC protocol handling
- `packages/daemon-client/src/errors.ts` - Error types
- `packages/daemon-client/tests/` - Unit tests

**Key APIs:**
```typescript
class DaemonClient {
  // Initialization (lazy - daemon starts on first request)
  constructor(config: DaemonConfig)
  
  // High-level methods
  async ping(): Promise<string>
  async search(params: SearchParams): Promise<SearchResult>
  
  // Lifecycle management
  async start(): Promise<void>  // Explicit start (optional)
  async stop(): Promise<void>   // Graceful shutdown
  async restart(): Promise<void> // Force restart
  
  // Health checking
  async isHealthy(): Promise<boolean>
  
  // Low-level RPC (for extensibility)
  private async sendRequest(method: string, params: any): Promise<any>
}
```

**Testing Strategy:**
- Unit tests with mocked child process
- Integration tests with actual daemon
- Crash recovery tests
- Concurrent request tests

### Phase 2: MCP Server Migration (2-3 hours)

**Deliverables:**
- Updated `packages/maproom-mcp/src/tools/search.ts`
- Updated `packages/maproom-mcp/package.json` (add daemon-client dependency)
- Updated tests
- Updated documentation

**Changes:**
```typescript
// Before (packages/maproom-mcp/src/tools/search.ts, line 268-291)
result = await trySpawnWithCandidates(candidates, args, {
  timeout: 30000,
  captureStdout: true,
  captureStderr: true,
})
rustOutput = JSON.parse(result.stdout)

// After
import { DaemonClient } from '@maproom/daemon-client'

const daemon = new DaemonClient({ /* config */ })
const searchResult = await daemon.search({
  query,
  repo,
  worktree,
  limit,
  mode,
  debug
})
```

**Key Changes:**
1. Remove `trySpawnWithCandidates` logic
2. Remove binary candidate selection
3. Replace with `DaemonClient.search()`
4. Update error handling for JSON-RPC errors
5. Keep chunk ID fetching (still required)

### Phase 3: Testing & Validation (1-2 hours)

**Integration Testing:**
- Test daemon startup/shutdown lifecycle
- Test multiple concurrent search requests
- Test daemon crash recovery
- Test error scenarios (invalid repo, bad query, etc.)
- Benchmark latency improvements

**Performance Validation:**
- Measure cold start latency (should be similar to current)
- Measure warm request latency (target: < 10ms)
- Measure concurrent request handling
- Memory usage comparison

**Acceptance Criteria:**
- ✅ All MCP search requests use daemon
- ✅ Daemon auto-starts on first request
- ✅ Daemon survives multiple requests
- ✅ Daemon auto-restarts on crash
- ✅ Warm search latency < 10ms (vs current ~200-500ms)
- ✅ All integration tests pass
- ✅ No regressionsin functionality

### Phase 4: Documentation & Cleanup (1 hour)

**Documentation:**
- Update MCP server README
- Document daemon architecture
- Add troubleshooting guide
- Update deployment instructions

**Code Cleanup:**
- Deprecate old spawning utilities (mark with @deprecated)
- Remove unused binary candidate logic
-Update type definitions
- Clean up imports

---

## Testing Strategy

### Unit Tests (daemon-client package)
```typescript
describe('DaemonClient', () => {
  it('should start daemon on first request')
  it('should reuse daemon for subsequent requests')
  it('should send ping request correctly')
  it('should send search request with all parameters')
  it('should handle JSON-RPC responses')
  it('should handle daemon crashes')
  it('should restart daemon after crash')
  it('should timeout on unresponsive daemon')
  it('should cleanup on close()')
})
```

### Integration Tests (maproom-mcp package)
```typescript
describe('Search with Daemon', () => {
  it('should perform search via daemon')
  it('should handle concurrent requests')
  it('should recover from daemon crash')
  it('should handle invalid repository')
  it('should handle database errors')
  it('should include chunk IDs in results')
})
```

### Performance Tests
```typescript
describe('Performance', () => {
  it('cold start latency should be < 500ms')
  it('warm request latency should be < 10ms')
  it('should handle 10 concurrent requests')
  it('should handle 100 sequential requests without memory leak')
})
```

---

## Risks & Mitigations

### Risk 1: Daemon Crashes Frequently
**Likelihood:** Low (daemon already tested in MAPDAEMON)  
**Impact:** High (all searches fail)  
**Mitigation:**
- Implement auto-restart with exponential backoff
- Add fallback to process spawning
- Comprehensive error logging
- Health check before each request

### Risk 2: Daemon Becomes Unresponsive
**Likelihood:** Medium (could happen under high load)  
**Impact:** High (searches hang)  
**Mitigation:**
- Implement request timeout (30s default)
- Send ping before search to detect stale daemon
- Auto-restart on timeout
- Circuit breaker pattern

### Risk 3: Breaking Changes to MCP Functionality
**Likelihood:** Medium (refactoring error handling)  
**Impact:** High (regression in search)  
**Mitigation:**
- Comprehensive integration tests
- Gradual rollout (flag-based)
- Keep old code as fallback
- Extensive manual testing

### Risk 4: Performance Regression
**Likelihood:** Low (daemon is faster by design)  
**Impact:** Medium (slower searches)  
**Mitigation:**
- Benchmark before/after migration
- Performance tests in CI
- Monitor latency metrics

### Risk 5: Debugging Complexity
**Likelihood:** High (daemon adds indirection)  
**Impact:** Medium (harder to debug issues)  
**Mitigation:**
- Comprehensive logging in daemon-client
- Keep process spawning code for comparison
- Document daemon architecture
- Add debug mode flag

---

## Success Criteria

1. ✅ **Daemon client library exists** and is tested
2. ✅ **MCP server uses daemon** for all search operations
3. ✅ **No process spawning** for search requests (only daemon)
4. ✅ **Daemon auto-starts** on first search request
5. ✅ **Daemon survives** multiple requests (connection reuse working)
6. ✅ **Daemon auto-restarts** on crash (recovery working)
7. ✅ **Performance:** Warm search latency < 10ms (currently ~200-500ms)
8. ✅ **All tests pass:** Unit, integration, performance
9. ✅ **No functionality regressions:** All existing features work
10. ✅ **Documentation updated:** README, architecture diagrams, troubleshooting

---

## Timeline

| Phase | Estimated Time | Cumulative |
|-------|---------------|------------|
| 1. DaemonClient Library | 3-4 hours | 3-4 hours |
| 2. MCP Server Migration | 2-3 hours | 5-7 hours |
| 3. Testing & Validation | 1-2 hours | 6-9 hours |
| 4. Documentation & Cleanup | 1 hour | 7-10 hours |

**Total Estimated Effort:** 7-10 hours

---

## Future Enhancements (Out of Scope)

1. **VSCode Extension Migration**
   - Migrate `scan` command to daemon
   - Lower priority (infrequent operation)
   - Estimated: 2-3 hours

2. **Shared Daemon Mode**
   - Single daemon shared across multiple clients
   - Requires IPC coordination
   - Estimated: 5-7 hours

3. **Daemon Monitoring Dashboard**
   - Web UI to monitor daemon health, stats
   - Prometheus metrics integration
   - Estimated: 8-10 hours

4. **Batch Search API**
   - Single request for multiple searches
   - Better performance for bulk operations
   - Estimated: 3-4 hours

---

## Related Projects

- **MAPDAEMON** ✅ Complete - Daemon implementation
- **UNISRCH** - Unified search (could be combined with this project)
- **VECSRCH** - Vector search already in daemon, just needs client wiring

---

## References

- **MAPDAEMON Project:** `.agents/projects/mapdaemon_maproom-daemon-architecture/`
- **Strategic Evaluation:** `.agents/reports/maproom-strategic-evaluation-2025-11-21.md`
- **Daemon Implementation:** `crates/maproom/src/daemon/mod.rs`
- **MCP Search Tool:** `packages/maproom-mcp/src/tools/search.ts`
- **Migration Plan:** `MCP_MIGRATION_PENDING.md`

---

**Created:** 2025-11-21  
**Status:** Ready for Implementation  
**Priority:** HIGH  
**Estimated ROI:** 50-100x latency improvement for warm searches
