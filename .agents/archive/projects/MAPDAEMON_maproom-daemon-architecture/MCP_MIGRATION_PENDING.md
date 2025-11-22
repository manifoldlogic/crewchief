# MCP Server Migration to Daemon Architecture

## Status: ⚠️ PENDING

The MAPDAEMON project successfully implemented the daemon architecture in the Rust binary, but the **MCP server TypeScript code still uses the old process-spawning approach**. This migration is necessary to realize the performance benefits of the daemon.

---

## Current State (Post-MAPDAEMON)

### ✅ Completed
- **Rust Daemon:** `crewchief-maproom serve` command implemented
- **JSON-RPC Protocol:** Full support for `ping` and `search` methods
- **Connection Pooling:** Database connections shared across requests
- **Performance Validated:** Ping latency < 1ms, ready for production

### ❌ Not Yet Migrated
- **MCP Server (`packages/maproom-mcp/`):** Still spawns binary for each request
- **Process Overhead:** Each search spawns new process, no connection reuse
- **TypeScript Integration:** No daemon client implemented

---

## Files Still Using Process Spawning

### 1. `packages/maproom-mcp/src/tools/search.ts`
**Lines 268-291:** Spawns `crewchief-maproom search` for each request

```typescript
result = await trySpawnWithCandidates(candidates, args, {
  timeout: 30000,
  captureStdout: true,
  captureStderr: true,
})
```

**Impact:** High - This is the core search functionality

### 2. `packages/maproom-mcp/src/tools/upsert.ts`
**Line 154:** Spawns `crewchief-maproom scan` for indexing

```typescript
processResult = await trySpawnWithCandidates(candidates, args, spawnOptions)
```

**Impact:** Medium - Used for indexing operations

### 3. `packages/maproom-mcp/src/utils/process.ts`
**Line 301:** Core spawning utility

```typescript
export async function trySpawnWithCandidates(...)
```

**Impact:** Low - Utility function, can be replaced

---

## Migration Strategy

### Phase 1: Daemon Client Library (New)
**File:** `packages/maproom-mcp/src/utils/daemon-client.ts`

**Responsibilities:**
1. Start daemon process once (persistent)
2. Send JSON-RPC requests over stdin/stdout
3. Parse JSON-RPC responses
4. Handle daemon lifecycle (start, health check, restart)
5. Connection pooling/reuse

**Key APIs:**
```typescript
class DaemonClient {
  async start(): Promise<void>
  async ping(): Promise<string>
  async search(params: SearchParams): Promise<SearchBundle>
  async close(): Promise<void>
  private sendRequest(method: string, params: any): Promise<any>
}
```

### Phase 2: Update Search Tool
**File:** `packages/maproom-mcp/src/tools/search.ts`

**Changes:**
- Replace `trySpawnWithCandidates()` with `daemonClient.search()`
- Remove subprocess spawning code (lines 233-291)
- Simplify error handling (daemon handles validation)
- Keep chunk ID fetching logic (still needed)

**Before:**
```typescript
result = await trySpawnWithCandidates(candidates, args, {...})
rustOutput = JSON.parse(result.stdout)
```

**After:**
```typescript
searchResult = await daemonClient.search({ query, repo, worktree, limit, mode })
```

### Phase 3: Update Upsert Tool (Optional)
**File:** `packages/maproom-mcp/src/tools/upsert.ts`

**Decision:** Upsert/scan operations are less frequent and can continue using process spawning for now. The daemon is optimized for high-frequency search requests, not batch indexing.

**Recommendation:** Keep as-is unless profiling shows indexing is a bottleneck.

### Phase 4: Deprecate Process Utilities
**File:** `packages/maproom-mcp/src/utils/process.ts`

**Action:** Mark `trySpawnWithCandidates` as deprecated, leave in place for upsert tool.

---

## Implementation Checklist

- [ ] **Create `daemon-client.ts`**
  - [ ] Start daemon on first search request (lazy initialization)
  - [ ] Send JSON-RPC requests over stdin
  - [ ] Parse responses from stdout
  - [ ] Handle daemon crashes/restarts
  - [ ] Add health check (`ping`) before each request
  - [ ] Implement request timeout (30s)

- [ ] **Update `search.ts`**
  - [ ] Replace `trySpawnWithCandidates` with `daemonClient.search()`
  - [ ] Remove binary candidate logic
  - [ ] Keep chunk ID fetching
  - [ ] Update error handling for JSON-RPC errors
  - [ ] Add tests for daemon integration

- [ ] **Integration Testing**
  - [ ] Test daemon lifecycle (start, stop, restart)
  - [ ] Test multiple concurrent requests
  - [ ] Test daemon crash recovery
  - [ ] Benchmark latency improvement (expect 10-100x for warm requests)

- [ ] **Update Documentation**
  - [ ] Update MCP server README
  - [ ] Document daemon architecture
  - [ ] Update deployment instructions

---

## Performance Impact

### Current (Process Spawning)
- **Cold Start:** ~200-500ms per request (process spawn + DB handshake)
- **Concurrent Requests:** Each spawns new process and DB connection
- **Resource Usage:** High (N processes, N DB connections)

### Expected (Daemon)
- **Cold Start:** ~200-500ms (first request only, daemon initialization)
- **Warm Requests:** ~0.5-5ms (daemon already running, connection pooled)
- **Concurrent Requests:** Single daemon, shared connection pool
- **Resource Usage:** Low (1 process, 1 connection pool)

**Expected Improvement:** **50-100x latency reduction** for warm requests

---

## Risks & Mitigations

### Risk 1: Daemon Crashes
**Mitigation:** Implement auto-restart on crash, fallback to process spawning

### Risk 2: Stale Daemon State
**Mitigation:** Implement `ping` health check before each request, restart if unresponsive

### Risk 3: Debugging Complexity
**Mitigation:** Add comprehensive logging, keep process spawning code for fallback

---

## Acceptance Criteria

1. ✅ MCP server starts daemon on first search request
2. ✅ Search requests use daemon (no process spawning)
3. ✅ Daemon survives multiple requests (connection reuse)
4. ✅ Daemon auto-restarts on crash
5. ✅ Performance: Warm search latency < 10ms (vs current ~200ms)
6. ✅ Integration tests pass (daemon lifecycle, concurrency, recovery)

---

## Recommendation

**Priority:** HIGH  
**Estimated Effort:** 3-4 hours  
**Blocking:** No (current process spawning works, just slower)

This migration should be the **next project** after MAPDAEMON to realize the full benefits of the daemon architecture. The code is ready, we just need to wire up the MCP client.

---

## Related Projects

- **MAPDAEMON** ✅ Complete - Daemon implementation ready
- **UNISRCH** - May want to do this migration first to clean up TypeScript search logic
- **VECSRCH** - Vector search already exposed in daemon, just needs client wiring

---

**Created:** 2025-11-21  
**Author:** Antigravity AI Assistant  
**Status:** Documentation for future work
